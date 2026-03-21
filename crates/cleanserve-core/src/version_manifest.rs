use crate::{CleanServeError, Result};
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{info, warn};

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/LyeZinho/php-runtimes/main/manifests/versions.json";
const CACHE_TTL_SECS: u64 = 3600; // 1 hour

#[derive(Debug, Clone, Deserialize)]
pub struct VersionManifest {
    pub schema_version: String,
    pub updated_at: String,
    pub repository: String,
    pub versions: Vec<PhpVersion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhpVersion {
    pub version: String,
    pub tag: String,
    pub published_at: String,
    pub html_url: String,
    pub platforms: Vec<PlatformBinary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformBinary {
    pub platform: String,
    pub filename: String,
    pub download_url: String,
    pub size_bytes: u64,
    pub sha256: String,
}

impl VersionManifest {
    /// Cache directory: ~/.cleanserve/cache/
    fn cache_dir() -> Result<PathBuf> {
        let dir = dirs::home_dir()
            .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
            .join(".cleanserve")
            .join("cache");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn cache_path() -> Result<PathBuf> {
        Ok(Self::cache_dir()?.join("versions.json"))
    }

    fn meta_path() -> Result<PathBuf> {
        Ok(Self::cache_dir()?.join("manifest.meta"))
    }

    /// Check if cache is still valid (within TTL)
    fn is_cache_valid() -> bool {
        let meta = match Self::meta_path() {
            Ok(p) => p,
            Err(_) => return false,
        };
        let cache = match Self::cache_path() {
            Ok(p) => p,
            Err(_) => return false,
        };
        if !meta.exists() || !cache.exists() {
            return false;
        }
        match std::fs::metadata(&meta).and_then(|m| m.modified()) {
            Ok(modified) => {
                modified.elapsed().map(|d| d.as_secs() < CACHE_TTL_SECS).unwrap_or(false)
            }
            Err(_) => false,
        }
    }

    /// Write manifest to cache
    fn write_cache(json: &str) -> Result<()> {
        let cache = Self::cache_path()?;
        let meta = Self::meta_path()?;
        std::fs::write(&cache, json)?;
        std::fs::write(&meta, "")?; // touch to update mtime
        Ok(())
    }

    /// Read manifest from cache
    fn read_cache() -> Result<String> {
        let cache = Self::cache_path()?;
        std::fs::read_to_string(&cache)
            .map_err(|e| CleanServeError::Config(format!("Cannot read manifest cache: {}", e)))
    }

    /// Fetch manifest from remote, falling back to cache on failure
    pub async fn fetch(force_refresh: bool) -> Result<Self> {
        // Use cache if valid and not forcing refresh
        if !force_refresh && Self::is_cache_valid() {
            info!("Using cached version manifest");
            let json = Self::read_cache()?;
            let manifest: VersionManifest = serde_json::from_str(&json)
                .map_err(|e| CleanServeError::Parse(format!("Invalid cached manifest: {}", e)))?;
            return Ok(manifest);
        }

        // Fetch from remote
        info!("Fetching version manifest from {}", MANIFEST_URL);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| CleanServeError::Download(format!("HTTP client error: {}", e)))?;

        match client.get(MANIFEST_URL).send().await {
            Ok(response) if response.status().is_success() => {
                let json = response.text().await
                    .map_err(|e| CleanServeError::Download(format!("Failed to read manifest: {}", e)))?;
                let manifest: VersionManifest = serde_json::from_str(&json)
                    .map_err(|e| CleanServeError::Parse(format!("Invalid manifest JSON: {}", e)))?;

                // Update cache
                if let Err(e) = Self::write_cache(&json) {
                    warn!("Failed to cache manifest: {}", e);
                }

                Ok(manifest)
            }
            Ok(response) => {
                warn!("Manifest fetch failed with status: {}", response.status());
                Self::fallback_to_cache()
            }
            Err(e) => {
                warn!("Manifest fetch failed: {}", e);
                Self::fallback_to_cache()
            }
        }
    }

    /// Fallback to cached manifest when fetch fails
    fn fallback_to_cache() -> Result<Self> {
        let cache = Self::cache_path()?;
        if cache.exists() {
            warn!("Using stale cached manifest (offline mode)");
            let json = Self::read_cache()?;
            let manifest: VersionManifest = serde_json::from_str(&json)
                .map_err(|e| CleanServeError::Parse(format!("Invalid cached manifest: {}", e)))?;
            Ok(manifest)
        } else {
            Err(CleanServeError::Download(
                "Cannot fetch version manifest and no cache available. Check your internet connection.".into()
            ))
        }
    }

    /// Find the latest patch for a minor version (e.g., "8.4" -> "8.4.19")
    /// Also accepts exact versions (e.g., "8.4.19" -> "8.4.19")
    pub fn find_version(&self, query: &str) -> Option<&PhpVersion> {
        // Try exact match first
        if let Some(v) = self.versions.iter().find(|v| v.version == query) {
            return Some(v);
        }

        // Try as minor version prefix — find latest patch
        let prefix = format!("{}.", query);
        self.versions
            .iter()
            .filter(|v| v.version.starts_with(&prefix))
            .max_by(|a, b| version_cmp(&a.version, &b.version))
    }

    /// Get platform binary for a resolved version
    pub fn get_platform_binary(&self, version: &str, platform: &str) -> Option<&PlatformBinary> {
        self.versions
            .iter()
            .find(|v| v.version == version)?
            .platforms
            .iter()
            .find(|p| p.platform == platform)
    }

    /// All available versions, sorted newest first
    pub fn list_available(&self) -> Vec<&PhpVersion> {
        let mut versions: Vec<&PhpVersion> = self.versions.iter().collect();
        versions.sort_by(|a, b| version_cmp(&b.version, &a.version));
        versions
    }
}

/// Compare semver-like version strings (e.g., "8.4.19" vs "8.4.2")
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    parse(a).cmp(&parse(b))
}
