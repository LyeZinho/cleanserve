# Version Management Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace HTML-scraping PHP version management with manifest-driven downloads from LyeZinho/php-runtimes.

**Architecture:** New `version_manifest.rs` module handles fetching/caching/querying versions.json. Existing `php_downloader.rs` is rewritten to use the manifest instead of scraping dl.static-php.dev. SHA256 verification is mandatory. CLI `list` command shows remote+installed versions.

**Tech Stack:** Rust, serde (JSON deserialization), sha2 (SHA256 verification), reqwest (HTTP), dirs (home directory)

---

### Task 1: Add sha2 dependency to cleanserve-core

**Files:**
- Modify: `crates/cleanserve-core/Cargo.toml`

**Step 1: Add sha2 crate**

In `crates/cleanserve-core/Cargo.toml`, add to `[dependencies]`:

```toml
sha2 = "0.10"
```

**Step 2: Verify it compiles**

Run: `cargo check -p cleanserve-core`
Expected: Compiles with no errors (may download sha2 crate)

**Step 3: Commit**

```bash
git add crates/cleanserve-core/Cargo.toml Cargo.lock
git commit -m "build: add sha2 dependency for download integrity verification"
```

---

### Task 2: Create version_manifest.rs — structs and deserialization

**Files:**
- Create: `crates/cleanserve-core/src/version_manifest.rs`
- Modify: `crates/cleanserve-core/src/lib.rs`

**Step 1: Create the manifest module with data types**

Create `crates/cleanserve-core/src/version_manifest.rs`:

```rust
use crate::{CleanServeError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
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
```

**Step 2: Register the module in lib.rs**

In `crates/cleanserve-core/src/lib.rs`, add after line 3 (`pub mod php_downloader;`):

```rust
pub mod version_manifest;
```

And add to the re-exports:

```rust
pub use version_manifest::{VersionManifest, PhpVersion, PlatformBinary};
```

**Step 3: Verify it compiles**

Run: `cargo check -p cleanserve-core`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add crates/cleanserve-core/src/version_manifest.rs crates/cleanserve-core/src/lib.rs
git commit -m "feat: add version manifest module for php-runtimes registry"
```

---

### Task 3: Rewrite php_downloader.rs to use manifest

**Files:**
- Modify: `crates/cleanserve-core/src/php_downloader.rs`

**Step 1: Rewrite php_downloader.rs**

Replace the entire content of `crates/cleanserve-core/src/php_downloader.rs` with:

```rust
use crate::{CleanServeError, Result, VersionManifest};
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct PhpDownloader {
    base_dir: PathBuf,
}

impl PhpDownloader {
    /// Create a new downloader with project-local base directory
    /// PHP will be stored at: base_dir/php-{version}/
    pub fn new(base_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_dir)
            .map_err(|e| CleanServeError::Config(format!("Cannot create PHP directory: {}", e)))?;

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Get the install path for a PHP version
    pub fn get_install_path(&self, version: &str) -> PathBuf {
        self.base_dir.join(format!("php-{}", version))
    }

    /// Check if PHP is already installed
    pub fn is_installed(&self, version: &str) -> bool {
        self.php_exe_candidates(version).iter().any(|p| p.exists())
    }

    /// Get the path to the PHP executable
    pub fn get_php_exe(&self, version: &str) -> Option<PathBuf> {
        self.php_exe_candidates(version).into_iter().find(|p| p.exists())
    }

    /// Candidate paths for the PHP binary
    fn php_exe_candidates(&self, version: &str) -> Vec<PathBuf> {
        let path = self.get_install_path(version);
        vec![
            path.join("php"),
            path.join("bin").join("php"),
        ]
    }

    /// Download and install PHP using the version manifest.
    /// `version` can be a minor version ("8.4") or exact ("8.4.19").
    pub async fn download(&self, version: &str) -> Result<()> {
        // Fetch manifest
        let manifest = VersionManifest::fetch(false).await?;

        // Resolve version
        let php_version = manifest.find_version(version).ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} not found in manifest. Run 'cleanserve list' to see available versions.",
                version
            ))
        })?;

        let resolved = &php_version.version;

        if self.is_installed(resolved) {
            info!("PHP {} is already installed", resolved);
            return Ok(());
        }

        // Get platform binary
        let platform = current_platform();
        let binary = manifest.get_platform_binary(resolved, platform).ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} is not available for platform '{}'. Only Linux is supported currently.",
                resolved, platform
            ))
        })?;

        info!("Downloading PHP {} ({:.1} MB)...", resolved, binary.size_bytes as f64 / 1_048_576.0);
        info!("URL: {}", binary.download_url);

        // Download
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| CleanServeError::Download(format!("HTTP client error: {}", e)))?;

        let response = client.get(&binary.download_url).send().await
            .map_err(|e| CleanServeError::Download(format!("Failed to download: {}", e)))?;

        if !response.status().is_success() {
            return Err(CleanServeError::Download(format!(
                "Download failed with status: {} from {}",
                response.status(), binary.download_url
            )));
        }

        let bytes = response.bytes().await
            .map_err(|e| CleanServeError::Download(format!("Failed to read response: {}", e)))?;

        // SHA256 verification (mandatory)
        info!("Verifying SHA256 checksum...");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let computed = format!("{:x}", hasher.finalize());

        if computed != binary.sha256 {
            return Err(CleanServeError::Download(format!(
                "SHA256 mismatch!\n  Expected: {}\n  Got:      {}\nDownload may be corrupted. Try again.",
                binary.sha256, computed
            )));
        }
        info!("SHA256 verified OK");

        // Write to temp file
        let temp_tarball = std::env::temp_dir().join(format!("php-{}.tar.gz", resolved));
        std::fs::write(&temp_tarball, &bytes)?;

        // Extract
        let install_path = self.get_install_path(resolved);
        std::fs::create_dir_all(&install_path)?;

        info!("Extracting to {}...", install_path.display());

        let output = tokio::process::Command::new("tar")
            .args(["-xzf", temp_tarball.to_str().unwrap(), "-C", install_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| CleanServeError::Download(format!("Failed to extract: {}", e)))?;

        if !output.status.success() {
            // Cleanup on failure
            let _ = std::fs::remove_dir_all(&install_path);
            return Err(CleanServeError::Download(format!(
                "tar extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Make PHP executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for candidate in self.php_exe_candidates(resolved) {
                if candidate.exists() {
                    let mut perms = std::fs::metadata(&candidate)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&candidate, perms)?;
                }
            }
        }

        // Cleanup temp file
        let _ = std::fs::remove_file(&temp_tarball);

        info!("PHP {} installed successfully at {}", resolved, install_path.display());
        Ok(())
    }
}

/// Detect current platform for manifest lookup
fn current_platform() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p cleanserve-core`
Expected: Compiles with no errors

**Step 3: Verify all crates compile (since up.rs and update.rs depend on PhpDownloader)**

Run: `cargo check --workspace`
Expected: Compiles with no errors (API is preserved)

**Step 4: Commit**

```bash
git add crates/cleanserve-core/src/php_downloader.rs
git commit -m "feat: rewrite php_downloader to use manifest-driven downloads with SHA256 verification"
```

---

### Task 4: Rewrite list.rs — show remote + installed versions

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/list.rs`
- Modify: `crates/cleanserve-cli/src/main.rs`

**Step 1: Add --refresh and --installed flags to CLI**

In `crates/cleanserve-cli/src/main.rs`, change the `List` variant from:

```rust
    /// List installed PHP versions
    List,
```

to:

```rust
    /// List available and installed PHP versions
    List {
        /// Force refresh of the version manifest
        #[arg(long)]
        refresh: bool,
        /// Show only installed versions
        #[arg(long)]
        installed: bool,
    },
```

And update the match arm from:

```rust
        Commands::List => {
            commands::list::run().await?;
        }
```

to:

```rust
        Commands::List { refresh, installed } => {
            commands::list::run(refresh, installed).await?;
        }
```

**Step 2: Rewrite list.rs**

Replace the entire content of `crates/cleanserve-cli/src/commands/list.rs` with:

```rust
use anyhow::Context;
use cleanserve_core::{PhpDownloader, VersionManifest};
use std::path::Path;

pub async fn run(refresh: bool, installed_only: bool) -> anyhow::Result<()> {
    let php_dir = Path::new(".cleanserve").join("php");

    if installed_only {
        return list_installed(&php_dir);
    }

    // Fetch manifest
    let manifest = VersionManifest::fetch(refresh)
        .await
        .context("Failed to fetch version manifest")?;

    let downloader = PhpDownloader::new(&php_dir).ok();
    let versions = manifest.list_available();

    if versions.is_empty() {
        println!("No PHP versions available in manifest.");
        return Ok(());
    }

    println!("Available PHP versions:");
    println!();

    let mut installed_count = 0u32;

    for v in &versions {
        let is_installed = downloader
            .as_ref()
            .map(|d| d.is_installed(&v.version))
            .unwrap_or(false);

        if is_installed {
            installed_count += 1;
        }

        let size = v.platforms.first().map(|p| p.size_bytes).unwrap_or(0);
        let size_mb = size as f64 / 1_048_576.0;

        let marker = if is_installed { "  \u{2713} installed" } else { "" };
        println!("  {:<12} ({:.1} MB){}", v.version, size_mb, marker);
    }

    println!();
    println!(
        "Installed: {} | Available: {}",
        installed_count,
        versions.len()
    );

    Ok(())
}

fn list_installed(php_dir: &Path) -> anyhow::Result<()> {
    if !php_dir.exists() {
        println!("No PHP versions installed.");
        println!("Run 'cleanserve update --version 8.4' to install PHP.");
        return Ok(());
    }

    let mut versions: Vec<String> = std::fs::read_dir(php_dir)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.strip_prefix("php-").map(|s| s.to_string())
        })
        .collect();

    if versions.is_empty() {
        println!("No PHP versions installed.");
        println!("Run 'cleanserve update --version 8.4' to install PHP.");
        return Ok(());
    }

    versions.sort();
    println!("Installed PHP versions:");
    for version in &versions {
        println!("  \u{2022} {}", version);
    }

    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add crates/cleanserve-cli/src/commands/list.rs crates/cleanserve-cli/src/main.rs
git commit -m "feat: list command shows remote + installed versions from manifest"
```

---

### Task 5: Update use_.rs — auto-download if missing

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/use_.rs`

**Step 1: Rewrite use_.rs to auto-download**

Replace the entire content of `crates/cleanserve-cli/src/commands/use_.rs` with:

```rust
use anyhow::Context;
use cleanserve_core::PhpDownloader;
use std::path::Path;

pub async fn run(version: String) -> anyhow::Result<()> {
    let php_dir = Path::new(".cleanserve").join("php");
    let downloader = PhpDownloader::new(&php_dir)
        .context("Failed to initialize PHP downloader")?;

    if !downloader.is_installed(&version) {
        println!("PHP {} is not installed. Downloading...", version);
        downloader.download(&version).await
            .context(format!("Failed to download PHP {}", version))?;
    }

    // Resolve actual version (user may have passed "8.4", downloader installed "8.4.19")
    // Find which versioned directory was created
    let resolved = find_resolved_version(&php_dir, &version)?;

    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let mut config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load cleanserve.json")?;

    config.engine.php = resolved.clone();
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;

    println!("\u{2713} Switched to PHP {}", resolved);

    Ok(())
}

/// If user passed "8.4", find the actual installed version like "8.4.19"
fn find_resolved_version(php_dir: &Path, query: &str) -> anyhow::Result<String> {
    // Exact match
    if php_dir.join(format!("php-{}", query)).exists() {
        return Ok(query.to_string());
    }

    // Prefix match — find latest installed version matching the minor
    let prefix = format!("php-{}.", query);
    let mut matches: Vec<String> = std::fs::read_dir(php_dir)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with(&prefix) {
                Some(name.strip_prefix("php-")?.to_string())
            } else {
                None
            }
        })
        .collect();

    if matches.is_empty() {
        anyhow::bail!("PHP {} was downloaded but directory not found", query);
    }

    matches.sort_by(|a, b| {
        let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
        parse(a).cmp(&parse(b))
    });

    Ok(matches.pop().unwrap())
}
```

**Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add crates/cleanserve-cli/src/commands/use_.rs
git commit -m "feat: use command auto-downloads PHP version if not installed"
```

---

### Task 6: Final workspace compilation and cleanup

**Step 1: Full workspace check**

Run: `cargo check --workspace`
Expected: Compiles with no errors

**Step 2: Check for warnings**

Run: `cargo check --workspace 2>&1 | grep warning`
Expected: No warnings related to our changes (pre-existing warnings OK)

**Step 3: Check that unused PhpManager is acknowledged**

Note: `PhpManager` in `php_manager.rs` uses `~/.cleanserve/bin/` (global dir) while the new system is project-local (`.cleanserve/php/`). The `list.rs` no longer uses it. It may become dead code. Leave it for now — it can be removed in a follow-up cleanup.

**Step 4: Commit if any fixups needed**

```bash
git add -A
git commit -m "chore: fix warnings from version management rewrite"
```
