//! Static Asset Server with Compression
//! 
//! High-performance static file serving with Gzip/Brotli compression
//! and LRU caching for production workloads.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Compressed asset with metadata
#[derive(Debug, Clone)]
pub struct CompressedAsset {
    pub content: Vec<u8>,
    pub content_type: String,
    pub original_size: usize,
    pub compressed_size: usize,
    pub encoding: ContentEncoding,
}

/// Content encoding type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentEncoding {
    None,
    Gzip,
    Brotli,
}

impl std::fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentEncoding::None => write!(f, "identity"),
            ContentEncoding::Gzip => write!(f, "gzip"),
            ContentEncoding::Brotli => write!(f, "br"),
        }
    }
}

/// LRU Cache for compressed assets
pub struct AssetCache {
    cache: RwLock<HashMap<String, CompressedAsset>>,
    max_entries: usize,
    max_size_bytes: usize,
    current_size: RwLock<usize>,
}

impl AssetCache {
    pub fn new(max_entries: usize, max_size_bytes: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_entries,
            max_size_bytes,
            current_size: RwLock::new(0),
        }
    }

    pub async fn get(&self, key: &str) -> Option<CompressedAsset> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    pub async fn insert(&self, key: String, asset: CompressedAsset) {
        let size = asset.compressed_size;
        
        // Evict if necessary
        self.evict_if_needed(size).await;
        
        let mut cache = self.cache.write().await;
        let mut current_size = self.current_size.write().await;
        
        // Remove old entry if exists
        if let Some(old) = cache.get(&key) {
            *current_size = current_size.saturating_sub(old.compressed_size);
        }
        
        cache.insert(key.clone(), asset.clone());
        *current_size += size;
    }

    async fn evict_if_needed(&self, new_size: usize) {
        let mut cache = self.cache.write().await;
        let mut current_size = self.current_size.write().await;
        
        // Evict by entries
        while cache.len() >= self.max_entries {
            if let Some((key, asset)) = cache.remove(&cache.keys().next().unwrap().clone()) {
                *current_size = current_size.saturating_sub(asset.compressed_size);
            }
        }
        
        // Evict by size
        while *current_size + new_size > self.max_size_bytes && !cache.is_empty() {
            if let Some((key, asset)) = cache.remove(&cache.keys().next().unwrap().clone()) {
                *current_size = current_size.saturating_sub(asset.compressed_size);
            }
        }
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let mut current_size = self.current_size.write().await;
        cache.clear();
        *current_size = 0;
    }

    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }
}

/// Static file server with compression
pub struct StaticServer {
    root: PathBuf,
    cache: Arc<AssetCache>,
    enable_gzip: bool,
    enable_brotli: bool,
}

impl StaticServer {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: Arc::new(AssetCache::new(1000, 100 * 1024 * 1024)), // 100MB cache
            enable_gzip: true,
            enable_brotli: true,
        }
    }

    pub fn with_cache_size(mut self, max_entries: usize, max_bytes: usize) -> Self {
        self.cache = Arc::new(AssetCache::new(max_entries, max_bytes));
        self
    }

    pub fn with_gzip(mut self, enabled: bool) -> Self {
        self.enable_gzip = enabled;
        self
    }

    pub fn with_brotli(mut self, enabled: bool) -> Self {
        self.enable_brotli = enabled;
        self
    }

    /// Serve a static file
    pub async fn serve(&self, path: &str, accept_encoding: &[String]) -> Option<StaticResponse> {
        // Normalize path
        let clean_path = path.trim_start_matches('/');
        let file_path = self.root.join(clean_path);
        
        // Security: prevent directory traversal
        if !file_path.starts_with(&self.root) {
            warn!("Blocked directory traversal attempt: {}", path);
            return None;
        }
        
        // Check if file exists
        if !file_path.is_file() {
            return None;
        }
        
        // Determine content type
        let content_type = mime_guess::from_path(&file_path)
            .first_or_octet_stream()
            .to_string();
        
        // Check cache
        let cache_key = format!("{}:{}", path, accept_encoding.join(","));
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Some(StaticResponse {
                content: cached.content,
                content_type: cached.content_type,
                encoding: cached.encoding,
                from_cache: true,
            });
        }
        
        // Read file
        let content = match tokio::fs::read(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read file {}: {}", file_path.display(), e);
                return None;
            }
        };
        
        let original_size = content.len();
        
        // Determine best encoding
        let (compressed, encoding) = self.compress(&content, accept_encoding);
        
        let asset = CompressedAsset {
            content: compressed.clone(),
            content_type: content_type.clone(),
            original_size,
            compressed_size: compressed.len(),
            encoding,
        };
        
        // Cache the result
        self.cache.insert(cache_key, asset).await;
        
        Some(StaticResponse {
            content: compressed,
            content_type,
            encoding,
            from_cache: false,
        })
    }

    fn compress(&self, content: &[u8], accept_encoding: &[String]) -> (Vec<u8>, ContentEncoding) {
        // Check if compression is worthwhile (don't compress small files)
        if content.len() < 256 {
            return (content.to_vec(), ContentEncoding::None);
        }
        
        // Prefer Brotli (better compression)
        if self.enable_brotli && accept_encoding.iter().any(|e| e.contains("br")) {
            if let Ok(compressed) = brotli::BrotliCompress(&mut std::io::Cursor::new(content), &mut Vec::new(), &brotli::enc::BrotliEncoderOptions::new(6)) {
                let ratio = compressed.len() as f64 / content.len() as f64;
                if ratio < 0.95 {
                    debug!("Brotli compression: {} -> {} ({:.1}%)", content.len(), compressed.len(), ratio * 100.0);
                    return (compressed, ContentEncoding::Brotli);
                }
            }
        }
        
        // Fall back to Gzip
        if self.enable_gzip && accept_encoding.iter().any(|e| e.contains("gzip")) {
            if let Ok(compressed) = compress_with_gzip(content) {
                let ratio = compressed.len() as f64 / content.len() as f64;
                if ratio < 0.95 {
                    debug!("Gzip compression: {} -> {} ({:.1}%)", content.len(), compressed.len(), ratio * 100.0);
                    return (compressed, ContentEncoding::Gzip);
                }
            }
        }
        
        // Return uncompressed
        (content.to_vec(), ContentEncoding::None)
    }
}

/// Static file response
pub struct StaticResponse {
    pub content: Vec<u8>,
    pub content_type: String,
    pub encoding: ContentEncoding,
    pub from_cache: bool,
}

/// Simple Gzip compression using deflate
fn compress_with_gzip(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = flate2::write::GzEncoder::new(
        Vec::with_capacity(data.len()),
        flate2::Compression::default(),
    );
    encoder.write_all(data)?;
    encoder.finish()
}

/// Check if a path is a static file
pub fn is_static_path(path: &str) -> bool {
    let static_extensions = [
        ".css", ".js", ".jpg", ".jpeg", ".png", ".gif", ".ico",
        ".svg", ".woff", ".woff2", ".ttf", ".eot", ".map",
        ".html", ".htm", ".txt", ".xml", ".json", ".yaml", ".yml",
        ".md", ".webp", ".avif", ".webm", ".mp4", ".mp3",
        ".pdf", ".zip", ".tar", ".gz",
    ];
    
    let path_lower = path.to_lowercase();
    static_extensions.iter().any(|ext| path_lower.ends_with(ext))
}

/// Parse Accept-Encoding header
pub fn parse_accept_encoding(header: Option<&str>) -> Vec<String> {
    header
        .map(|h| h.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default()
}
