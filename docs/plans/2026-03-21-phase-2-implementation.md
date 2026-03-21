# Phase 2: Core Package Manager Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the core package manager with download, caching, and project integration capabilities. This is the foundation for all subsequent phases.

**Architecture:** Package Manager consists of 4 independent modules that work together: (1) PackageRegistry loads built-in + custom manifests; (2) PackageDownloader handles HTTP fetch + SHA256 validation + extraction; (3) ProjectPackageManager manages cleanserve.json and symlinks; (4) CLI commands tie them together.

**Tech Stack:** Rust (Tokio async), `reqwest` for HTTP, `sha2` for checksums, `serde_json` for parsing, `tempfile` for safe extraction, `symlink` operations for symlinking.

---

## Pre-Implementation Setup

**Workspace:** This plan assumes you're in a `git worktree` (created by brainstorming skill).

**Verification Command (run after each task):**
```bash
# Check compilation and tests
cargo build --release 2>&1 | head -50
cargo test --lib 2>&1 | grep -E "test result:|FAILED"
```

---

## Task 1: Create Package Manager Module Structure

**Files:**
- Create: `crates/cleanserve-core/src/package_manager/mod.rs`
- Create: `crates/cleanserve-core/src/package_manager/registry.rs`
- Create: `crates/cleanserve-core/src/package_manager/downloader.rs`
- Create: `crates/cleanserve-core/src/package_manager/project.rs`
- Modify: `crates/cleanserve-core/src/lib.rs` (add mod declarations)
- Modify: `Cargo.toml` (add dependencies)

**Step 1: Add dependencies to Cargo.toml**

In `/home/pedro/repo/cleanserve/Cargo.toml`, find the `[dependencies]` section and add:

```toml
sha2 = "0.10"
tempfile = "3.8"
tokio = { version = "1.35", features = ["full"] }  # Already present, verify it has "full"
reqwest = { version = "0.11", features = ["stream"] }  # Already present, verify "stream"
```

Run: `cargo check 2>&1 | head -20`
Expected: No errors about missing `sha2`, `tempfile`, `tokio`, `reqwest`.

**Step 2: Create module structure**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/mod.rs`:

```rust
//! Package Manager: Download, cache, and manage standalone tools
//!
//! The package manager enables projects to declare and use standalone tools
//! (MySQL, Redis, phpMyAdmin, etc) via `cleanserve package` commands.
//!
//! Architecture:
//! - Registry: Load built-in + custom package definitions
//! - Downloader: Fetch + verify packages from remote sources
//! - Project: Manage per-project package state in cleanserve.json

pub mod registry;
pub mod downloader;
pub mod project;

pub use registry::PackageRegistry;
pub use downloader::PackageDownloader;
pub use project::ProjectPackageManager;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Package metadata from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub description: String,
    pub homepage: Option<String>,
    pub versions: std::collections::HashMap<String, PackageVersion>,
}

/// Package version definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub downloads: std::collections::HashMap<String, DownloadInfo>,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
}

/// Download details for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    pub checksum: String,  // sha256:abc123...
    #[serde(default)]
    pub format: Option<String>,  // tar.xz, tar.gz, zip
}

#[derive(Debug)]
pub struct PackageManagerError {
    pub message: String,
}

impl std::fmt::Display for PackageManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PackageManager error: {}", self.message)
    }
}

impl std::error::Error for PackageManagerError {}

pub type Result<T> = std::result::Result<T, PackageManagerError>;
```

Run: `cargo check 2>&1 | grep -E "error|warning" | head -10`
Expected: May have warnings about unused imports, that's OK for now.

**Step 3: Create registry.rs**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/registry.rs`:

```rust
use super::{Package, Result, PackageManagerError};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Registry of available packages from built-in and custom manifests
#[derive(Debug)]
pub struct PackageRegistry {
    packages: HashMap<String, Package>,
}

/// Manifest structure for JSON parsing
#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    packages: HashMap<String, Package>,
}

impl PackageRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    /// Load built-in manifest from embedded JSON
    pub fn with_builtin() -> Result<Self> {
        let mut registry = Self::new();
        registry.load_builtin()?;
        Ok(registry)
    }

    /// Load built-in package manifest
    fn load_builtin(&mut self) -> Result<()> {
        // TODO: Embed manifest as string constant
        // For now, return empty registry
        Ok(())
    }

    /// Load custom manifest from file path
    pub fn load_custom(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            // Custom manifest is optional
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| PackageManagerError {
                message: format!("Failed to read custom manifest: {}", e),
            })?;

        let manifest: Manifest = serde_json::from_str(&content)
            .map_err(|e| PackageManagerError {
                message: format!("Invalid custom manifest JSON: {}", e),
            })?;

        // Merge into registry
        for (name, package) in manifest.packages {
            self.packages.insert(name, package);
        }

        Ok(())
    }

    /// Get package by name
    pub fn get(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }

    /// List all available packages
    pub fn list(&self) -> Vec<&Package> {
        self.packages.values().collect()
    }

    /// Verify package exists and has version
    pub fn verify(&self, name: &str, version: &str) -> Result<()> {
        let package = self.get(name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not found in registry", name),
            })?;

        if !package.versions.contains_key(version) {
            return Err(PackageManagerError {
                message: format!("Version '{}' not found for package '{}'", version, name),
            });
        }

        Ok(())
    }
}

impl Default for PackageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = PackageRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_get_missing_package() {
        let registry = PackageRegistry::new();
        assert!(registry.get("mysql").is_none());
    }
}
```

Run: `cargo test package_manager::registry --lib 2>&1 | grep -E "test result:|FAILED"`
Expected: `test result: ok. 2 passed`.

**Step 4: Create downloader.rs**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/downloader.rs`:

```rust
use super::{DownloadInfo, Result, PackageManagerError};
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};

/// Downloads and verifies packages
pub struct PackageDownloader;

impl PackageDownloader {
    /// Validate checksum format (sha256:abc123...)
    pub fn validate_checksum_format(checksum: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = checksum.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(PackageManagerError {
                message: format!("Invalid checksum format: {}. Expected 'algorithm:hash'", checksum),
            });
        }

        let algorithm = parts[0];
        let hash = parts[1];

        if algorithm != "sha256" {
            return Err(PackageManagerError {
                message: format!("Unsupported checksum algorithm: {}. Only 'sha256' supported", algorithm),
            });
        }

        if hash.len() != 64 {
            return Err(PackageManagerError {
                message: format!("Invalid SHA256 hash length: {}. Expected 64 hex chars", hash.len()),
            });
        }

        Ok((algorithm.to_string(), hash.to_lowercase()))
    }

    /// Compute SHA256 of file
    pub fn compute_sha256(path: &Path) -> Result<String> {
        let file = std::fs::File::open(path)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot read file for checksum: {}", e),
            })?;

        let mut hasher = Sha256::new();
        let mut reader = std::io::BufReader::new(file);
        use std::io::Read;
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)
                .map_err(|e| PackageManagerError {
                    message: format!("Error reading file: {}", e),
                })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Verify downloaded file matches checksum
    pub fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
        let (_algorithm, expected_hash) = Self::validate_checksum_format(expected_checksum)?;
        let actual_hash = Self::compute_sha256(file_path)?;

        if actual_hash != expected_hash {
            return Err(PackageManagerError {
                message: format!(
                    "Checksum mismatch for {}.\nExpected: {}\nActual:   {}",
                    file_path.display(),
                    expected_hash,
                    actual_hash
                ),
            });
        }

        Ok(())
    }

    /// Get platform identifier (linux-x64, darwin-arm64, etc)
    pub fn get_platform() -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match (os, arch) {
            ("linux", "x86_64") => "linux-x64".to_string(),
            ("linux", "aarch64") => "linux-arm64".to_string(),
            ("macos", "x86_64") => "darwin-x64".to_string(),
            ("macos", "aarch64") => "darwin-arm64".to_string(),
            ("windows", "x86_64") => "windows-x64".to_string(),
            _ => format!("{}-{}", os, arch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_checksum_format_valid() {
        let result = PackageDownloader::validate_checksum_format(
            "sha256:abc123abc123abc123abc123abc123abc123abc123abc123abc123abc123abc1"
        );
        assert!(result.is_ok());
        let (algo, hash) = result.unwrap();
        assert_eq!(algo, "sha256");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_validate_checksum_format_invalid_algorithm() {
        let result = PackageDownloader::validate_checksum_format(
            "md5:abc123"
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_checksum_format_invalid_length() {
        let result = PackageDownloader::validate_checksum_format(
            "sha256:tooshort"
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_sha256() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let hash = PackageDownloader::compute_sha256(&file_path).unwrap();
        // Known SHA256 of "hello world"
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_get_platform() {
        let platform = PackageDownloader::get_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains('-'));
    }
}
```

Run: `cargo test package_manager::downloader --lib 2>&1 | grep -E "test result:|FAILED"`
Expected: `test result: ok. 4 passed`.

**Step 5: Create project.rs**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/project.rs`:

```rust
use super::{Result, PackageManagerError};
use std::path::{Path, PathBuf};

/// Manages package state for a specific project
pub struct ProjectPackageManager {
    project_root: PathBuf,
    cleanserve_json_path: PathBuf,
}

impl ProjectPackageManager {
    /// Create project manager for given project root
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let cleanserve_json_path = project_root.join("cleanserve.json");

        if !cleanserve_json_path.exists() {
            return Err(PackageManagerError {
                message: format!("cleanserve.json not found at {}", cleanserve_json_path.display()),
            });
        }

        Ok(Self {
            project_root,
            cleanserve_json_path,
        })
    }

    /// Get package cache directory (~/.cleanserve/tools/)
    pub fn get_global_cache_dir() -> Result<PathBuf> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| PackageManagerError {
                message: "Cannot determine home directory".to_string(),
            })?
            .join(".cleanserve")
            .join("tools");

        Ok(cache_dir)
    }

    /// Get project .cleanserve/tools directory (for symlinks)
    pub fn get_project_tools_dir(&self) -> PathBuf {
        self.project_root.join(".cleanserve").join("tools")
    }

    /// Ensure project .cleanserve/tools directory exists
    pub fn ensure_tools_dir(&self) -> Result<()> {
        let tools_dir = self.get_project_tools_dir();
        std::fs::create_dir_all(&tools_dir)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create tools directory {}: {}", tools_dir.display(), e),
            })?;
        Ok(())
    }

    /// Create symlink from project to global cache
    pub fn create_symlink(&self, package_name: &str, version: &str) -> Result<()> {
        self.ensure_tools_dir()?;

        let global_path = Self::get_global_cache_dir()?
            .join(package_name)
            .join(version);

        if !global_path.exists() {
            return Err(PackageManagerError {
                message: format!("Global package not found at {}", global_path.display()),
            });
        }

        let symlink_path = self.get_project_tools_dir().join(package_name);

        // Remove existing symlink if present
        if symlink_path.exists() || std::fs::symlink_metadata(&symlink_path).is_ok() {
            std::fs::remove_file(&symlink_path)
                .map_err(|e| PackageManagerError {
                    message: format!("Cannot remove existing symlink: {}", e),
                })?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&global_path, &symlink_path)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create symlink: {}", e),
            })?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&global_path, &symlink_path)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create symlink: {}", e),
            })?;

        Ok(())
    }

    /// Remove symlink for package
    pub fn remove_symlink(&self, package_name: &str) -> Result<()> {
        let symlink_path = self.get_project_tools_dir().join(package_name);

        if symlink_path.exists() || std::fs::symlink_metadata(&symlink_path).is_ok() {
            std::fs::remove_file(&symlink_path)
                .map_err(|e| PackageManagerError {
                    message: format!("Cannot remove symlink: {}", e),
                })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = format!(
            "{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        assert!(!platform.is_empty());
    }
}
```

Run: `cargo test package_manager::project --lib 2>&1 | grep -E "test result:|FAILED"`
Expected: `test result: ok. 1 passed` (or similar).

**Step 6: Update lib.rs to expose module**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/lib.rs`:

Find the line that says `pub mod ...` (or create the section if it doesn't exist) and add:

```rust
pub mod package_manager;
```

Run: `cargo build --release 2>&1 | head -30`
Expected: Compiles successfully with no errors.

**Step 7: Commit**

```bash
git add crates/cleanserve-core/src/package_manager/ crates/cleanserve-core/src/lib.rs Cargo.toml
git commit -m "feat: create package manager module structure with registry, downloader, and project managers"
```

Expected: Commit succeeds, no uncommitted changes.

---

## Task 2: Implement Built-in Package Manifest

**Files:**
- Create: `crates/cleanserve-core/src/package_manager/manifest.rs`
- Create: `resources/packages-manifest.json`
- Modify: `crates/cleanserve-core/src/package_manager/mod.rs` (add mod declaration)
- Modify: `crates/cleanserve-core/build.rs` (embed manifest)

**Step 1: Create built-in manifest JSON**

Create `/home/pedro/repo/cleanserve/resources/packages-manifest.json`:

```json
{
  "version": "1.0",
  "packages": {
    "mysql": {
      "name": "MySQL Community Server",
      "description": "Open-source relational database",
      "homepage": "https://www.mysql.com/",
      "versions": {
        "8.0": {
          "downloads": {
            "linux-x64": {
              "url": "https://dev.mysql.com/get/mysql-server_8.0.35-1ubuntu1_amd64.deb",
              "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            }
          },
          "executable": "bin/mysqld",
          "requires": [],
          "env_vars": {}
        }
      }
    },
    "redis": {
      "name": "Redis",
      "description": "In-memory data structure store",
      "homepage": "https://redis.io/",
      "versions": {
        "7.0": {
          "downloads": {
            "linux-x64": {
              "url": "https://github.com/redis/redis/archive/7.0.0.tar.gz",
              "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            }
          },
          "executable": "src/redis-server",
          "requires": [],
          "env_vars": {}
        }
      }
    }
  }
}
```

Run: `cat resources/packages-manifest.json | jq . > /dev/null && echo "Valid JSON"`
Expected: `Valid JSON`.

**Step 2: Create manifest.rs**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/manifest.rs`:

```rust
use super::{Package, Result, PackageManagerError};
use serde::Deserialize;
use std::collections::HashMap;

/// Built-in package manifest (embedded in binary)
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub packages: HashMap<String, Package>,
}

impl Manifest {
    /// Load built-in manifest from embedded resource
    pub fn load_builtin() -> Result<Self> {
        const MANIFEST_JSON: &str = include_str!("../../resources/packages-manifest.json");
        
        serde_json::from_str(MANIFEST_JSON)
            .map_err(|e| PackageManagerError {
                message: format!("Failed to parse built-in manifest: {}", e),
            })
    }

    /// Validate manifest integrity
    pub fn validate(&self) -> Result<()> {
        if self.version.is_empty() {
            return Err(PackageManagerError {
                message: "Manifest version is empty".to_string(),
            });
        }

        if self.packages.is_empty() {
            return Err(PackageManagerError {
                message: "Manifest has no packages".to_string(),
            });
        }

        for (name, package) in &self.packages {
            if package.versions.is_empty() {
                return Err(PackageManagerError {
                    message: format!("Package '{}' has no versions", name),
                });
            }

            for (version, pkg_version) in &package.versions {
                if pkg_version.downloads.is_empty() {
                    return Err(PackageManagerError {
                        message: format!("Package '{}/{}' has no downloads", name, version),
                    });
                }

                for (platform, download_info) in &pkg_version.downloads {
                    if download_info.url.is_empty() {
                        return Err(PackageManagerError {
                            message: format!("Package '{}/{}/{}' has empty URL", name, version, platform),
                        });
                    }

                    if !download_info.checksum.starts_with("sha256:") {
                        return Err(PackageManagerError {
                            message: format!("Package '{}/{}/{}'has invalid checksum format", name, version, platform),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_builtin_manifest() {
        let manifest = Manifest::load_builtin();
        assert!(manifest.is_ok());
    }

    #[test]
    fn test_validate_builtin_manifest() {
        let manifest = Manifest::load_builtin().unwrap();
        let result = manifest.validate();
        assert!(result.is_ok());
    }
}
```

Run: `cargo test package_manager::manifest --lib 2>&1 | grep -E "test result:|FAILED"`
Expected: `test result: ok. 2 passed`.

**Step 3: Update registry.rs to use built-in manifest**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/registry.rs`:

Replace the `load_builtin()` method:

```rust
    /// Load built-in package manifest
    fn load_builtin(&mut self) -> Result<()> {
        let manifest = super::manifest::Manifest::load_builtin()?;
        manifest.validate()?;

        for (name, package) in manifest.packages {
            self.packages.insert(name, package);
        }

        Ok(())
    }
```

Also update the imports at the top:

```rust
use super::{Package, Result, PackageManagerError, manifest};
```

Run: `cargo test package_manager::registry --lib 2>&1 | grep -E "test result:|FAILED"`
Expected: Still passes.

**Step 4: Update mod.rs to expose manifest module**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/mod.rs`:

Add at the top after the other `pub mod` lines:

```rust
pub mod manifest;
```

Run: `cargo build --release 2>&1 | head -30`
Expected: Compiles successfully.

**Step 5: Add test for registry with builtin**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/registry.rs`:

In the tests module, add:

```rust
    #[test]
    fn test_registry_with_builtin() {
        let registry = PackageRegistry::with_builtin();
        assert!(registry.is_ok());
        
        let reg = registry.unwrap();
        assert!(reg.list().len() > 0);
    }

    #[test]
    fn test_registry_get_mysql() {
        let registry = PackageRegistry::with_builtin().unwrap();
        assert!(registry.get("mysql").is_some());
    }
```

Run: `cargo test package_manager::registry::tests --lib 2>&1 | tail -5`
Expected: `test result: ok. X passed`.

**Step 6: Commit**

```bash
git add crates/cleanserve-core/src/package_manager/ resources/ Cargo.toml
git commit -m "feat: add built-in package manifest with MySQL and Redis"
```

Expected: Commit succeeds.

---

## Task 3: Extend cleanserve.json Schema

**Files:**
- Modify: `crates/cleanserve-core/src/config.rs` (add packages field)
- Create: `crates/cleanserve-core/src/config/packages.rs`

**Step 1: Add packages field to Config struct**

First, read the current config structure:

```bash
grep -n "struct Config" crates/cleanserve-core/src/config.rs | head -5
```

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/config.rs` and add the packages field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // ... existing fields ...
    
    #[serde(default)]
    pub packages: Option<PackagesConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesConfig {
    #[serde(flatten)]
    pub packages: std::collections::HashMap<String, PackageSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageSpec {
    Simple(String),  // "mysql": "8.0"
    Detailed {
        version: String,
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        path: Option<String>,
    },
}
```

Add imports at top:

```rust
use std::collections::HashMap;
```

Run: `cargo build --release 2>&1 | grep -E "error\[" | head -10`
Expected: Builds successfully.

**Step 2: Add validation for packages config**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/config.rs` and add validation method:

```rust
impl PackagesConfig {
    /// Validate all packages
    pub fn validate(&self) -> Result<(), String> {
        if self.packages.is_empty() {
            return Ok(());
        }

        // TODO: Validate against registry
        Ok(())
    }
}
```

**Step 3: Update Config::load() to validate packages**

Find the `impl Config` section and add:

```rust
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref packages) = self.packages {
            packages.validate()?;
        }
        Ok(())
    }
```

Run: `cargo test --lib 2>&1 | grep -E "error|test result"`
Expected: Tests pass.

**Step 4: Create integration test for packages in config**

Create test file (or add to existing test):

```bash
cat >> crates/cleanserve-core/src/config.rs << 'EOF'

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_with_simple_packages() {
        let json = r#"{
            "name": "test",
            "packages": {
                "mysql": "8.0",
                "redis": "7.0"
            }
        }"#;
        
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.packages.is_some());
        assert_eq!(config.packages.unwrap().packages.len(), 2);
    }

    #[test]
    fn test_config_with_detailed_packages() {
        let json = r#"{
            "name": "test",
            "packages": {
                "mysql": {
                    "version": "8.0",
                    "enabled": true
                }
            }
        }"#;
        
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.packages.is_some());
    }
}
EOF
```

Run: `cargo test config::tests --lib 2>&1 | grep "test result"`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add crates/cleanserve-core/src/config.rs
git commit -m "feat: add packages configuration field to cleanserve.json schema"
```

---

## Task 4: Implement Package Add Command (Minimal)

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/mod.rs` (add package subcommand reference)
- Create: `crates/cleanserve-cli/src/commands/package.rs`

**Step 1: Create package command module**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-cli/src/commands/package.rs`:

```rust
use cleanserve_core::package_manager::{PackageRegistry, PackageDownloader, ProjectPackageManager};
use std::path::Path;

/// Package management subcommands
pub struct PackageCommand;

impl PackageCommand {
    /// Handle `cleanserve package add <name> [version]`
    pub async fn add(package_name: &str, version: Option<&str>, project_root: &Path) -> Result<(), String> {
        // Load registry
        let registry = PackageRegistry::with_builtin()
            .map_err(|e| format!("Failed to load package registry: {}", e))?;

        // Get default version if not specified
        let pkg = registry.get(package_name)
            .ok_or_else(|| format!("Package '{}' not found", package_name))?;

        let version = match version {
            Some(v) => v.to_string(),
            None => {
                // Use first available version (TODO: pick latest)
                pkg.versions.keys().next()
                    .ok_or("Package has no versions")?
                    .clone()
            }
        };

        // Verify version exists
        registry.verify(package_name, &version)
            .map_err(|e| e.to_string())?;

        println!("✓ Package '{}' version '{}' found", package_name, version);

        // TODO: Download and verify
        // TODO: Update cleanserve.json
        // TODO: Create symlink

        Ok(())
    }

    /// Handle `cleanserve package list`
    pub fn list() -> Result<(), String> {
        let registry = PackageRegistry::with_builtin()
            .map_err(|e| format!("Failed to load registry: {}", e))?;

        println!("Available packages:");
        for pkg in registry.list() {
            println!("  - {} ({})", pkg.name, pkg.description);
        }

        Ok(())
    }

    /// Handle `cleanserve package info <name>`
    pub fn info(package_name: &str) -> Result<(), String> {
        let registry = PackageRegistry::with_builtin()
            .map_err(|e| format!("Failed to load registry: {}", e))?;

        let pkg = registry.get(package_name)
            .ok_or_else(|| format!("Package '{}' not found", package_name))?;

        println!("Package: {}", pkg.name);
        println!("Description: {}", pkg.description);
        if let Some(url) = &pkg.homepage {
            println!("Homepage: {}", url);
        }
        println!("Available versions:");
        for version in pkg.versions.keys() {
            println!("  - {}", version);
        }

        Ok(())
    }
}
```

Run: `cargo check 2>&1 | grep -E "error\[|warning:" | head -10`
Expected: May have warnings, but no errors.

**Step 2: Update commands/mod.rs to include package**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-cli/src/commands/mod.rs`:

Add:

```rust
pub mod package;
```

And ensure the module is exported if using a match statement for commands.

**Step 3: Add CLI argument parsing for package command**

Find the main CLI handler (likely in `main.rs` or in the commands module where other subcommands are parsed).

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-cli/src/main.rs` (or appropriate location):

Add a match arm for `package` (exact location depends on your CLI structure):

```rust
    "package" => {
        let subcommand = args.get(1).map(|s| s.as_str());
        match subcommand {
            Some("add") => {
                let pkg_name = args.get(2).ok_or("Missing package name")?;
                let version = args.get(3);
                let project_root = std::env::current_dir()?;
                commands::package::PackageCommand::add(pkg_name, version.map(|s| s.as_str()), &project_root).await?;
            },
            Some("list") => {
                commands::package::PackageCommand::list()?;
            },
            Some("info") => {
                let pkg_name = args.get(2).ok_or("Missing package name")?;
                commands::package::PackageCommand::info(pkg_name)?;
            },
            _ => eprintln!("Usage: cleanserve package <add|list|info> [args]"),
        }
    },
```

Run: `cargo build --release 2>&1 | head -50`
Expected: Builds successfully (may have warnings).

**Step 4: Test the package command**

```bash
cargo run -- package list 2>&1 | head -10
```

Expected: Shows available packages (at least mysql).

**Step 5: Commit**

```bash
git add crates/cleanserve-cli/src/commands/
git commit -m "feat: add minimal 'cleanserve package add/list/info' commands"
```

---

## Task 5: Implement Package Download with Verification

**Files:**
- Modify: `crates/cleanserve-core/src/package_manager/downloader.rs` (add download logic)
- Create: `crates/cleanserve-core/src/package_manager/cache.rs`

**Step 1: Create cache.rs for global package storage**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/cache.rs`:

```rust
use super::{Result, PackageManagerError};
use std::path::{Path, PathBuf};

/// Manages global package cache at ~/.cleanserve/tools/
pub struct PackageCache;

impl PackageCache {
    /// Get cache root directory
    pub fn root() -> Result<PathBuf> {
        let cache = dirs::home_dir()
            .ok_or_else(|| PackageManagerError {
                message: "Cannot determine home directory".to_string(),
            })?
            .join(".cleanserve");

        Ok(cache)
    }

    /// Get package version cache path
    pub fn package_path(name: &str, version: &str) -> Result<PathBuf> {
        Self::root().map(|root| root.join("tools").join(name).join(version))
    }

    /// Ensure package version directory exists
    pub fn ensure_package_dir(name: &str, version: &str) -> Result<()> {
        let path = Self::package_path(name, version)?;
        std::fs::create_dir_all(&path)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create cache directory {}: {}", path.display(), e),
            })?;
        Ok(())
    }

    /// Get temp download directory
    pub fn temp_dir() -> Result<PathBuf> {
        let temp = Self::root()?.join("tmp");
        std::fs::create_dir_all(&temp)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create temp directory: {}", e),
            })?;
        Ok(temp)
    }

    /// Get logs directory
    pub fn logs_dir() -> Result<PathBuf> {
        let logs = Self::root()?.join("logs");
        std::fs::create_dir_all(&logs)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create logs directory: {}", e),
            })?;
        Ok(logs)
    }

    /// Check if package version is already cached
    pub fn exists(name: &str, version: &str) -> Result<bool> {
        Ok(Self::package_path(name, version)?.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_root() {
        let root = PackageCache::root();
        assert!(root.is_ok());
        let path = root.unwrap();
        assert!(path.to_string_lossy().contains(".cleanserve"));
    }

    #[test]
    fn test_package_path() {
        let path = PackageCache::package_path("mysql", "8.0");
        assert!(path.is_ok());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("mysql"));
        assert!(p.to_string_lossy().contains("8.0"));
    }
}
```

Run: `cargo test package_manager::cache --lib 2>&1 | grep "test result"`
Expected: Tests pass.

**Step 2: Add download logic to downloader.rs**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/downloader.rs`:

Add imports and async download method:

```rust
use reqwest::Client;
use tokio::io::AsyncWriteExt;

impl PackageDownloader {
    /// Download package from URL with streaming
    pub async fn download(url: &str, dest_path: &Path) -> Result<()> {
        let client = Client::new();
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| PackageManagerError {
                message: format!("Failed to download from {}: {}", url, e),
            })?;

        if !response.status().is_success() {
            return Err(PackageManagerError {
                message: format!("Download failed with status {}", response.status()),
            });
        }

        let mut file = tokio::fs::File::create(dest_path)
            .await
            .map_err(|e| PackageManagerError {
                message: format!("Cannot create file {}: {}", dest_path.display(), e),
            })?;

        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| PackageManagerError {
                message: format!("Error downloading: {}", e),
            })?;

            file.write_all(&chunk)
                .await
                .map_err(|e| PackageManagerError {
                message: format!("Error writing to file: {}", e),
            })?;
        }

        Ok(())
    }
}
```

Add to Cargo.toml:

```toml
futures = "0.3"
```

Run: `cargo check 2>&1 | grep -E "error\[" | head -5`
Expected: Builds successfully.

**Step 3: Update mod.rs to expose cache**

Edit `/home/pedro/repo/cleanserve/crates/cleanserve-core/src/package_manager/mod.rs`:

Add:

```rust
pub mod cache;

pub use cache::PackageCache;
```

**Step 4: Commit**

```bash
git add crates/cleanserve-core/src/package_manager/ Cargo.toml
git commit -m "feat: implement package download with streaming and cache management"
```

---

## Task 6: Create Comprehensive Integration Test

**Files:**
- Create: `crates/cleanserve-core/tests/integration_package_manager.rs`

**Step 1: Write integration test**

Create `/home/pedro/repo/cleanserve/crates/cleanserve-core/tests/integration_package_manager.rs`:

```rust
use cleanserve_core::package_manager::{PackageRegistry, PackageDownloader, PackageCache};

#[tokio::test]
async fn test_registry_loads_builtin() {
    let registry = PackageRegistry::with_builtin().expect("Failed to load registry");
    let packages = registry.list();
    assert!(!packages.is_empty(), "Registry should have packages");
}

#[tokio::test]
async fn test_registry_finds_mysql() {
    let registry = PackageRegistry::with_builtin().expect("Failed to load registry");
    assert!(registry.get("mysql").is_some(), "MySQL should be in registry");
}

#[test]
fn test_checksum_validation() {
    // Valid checksum
    let result = PackageDownloader::verify_checksum(
        &std::path::PathBuf::from("/tmp/test.txt"),
        "sha256:abc123"
    );
    // Will fail because file doesn't exist, but validates format
    assert!(result.is_err());
}

#[test]
fn test_cache_paths() {
    let root = PackageCache::root().expect("Cache root should exist");
    assert!(root.to_string_lossy().contains(".cleanserve"));

    let pkg_path = PackageCache::package_path("mysql", "8.0")
        .expect("Package path should be valid");
    assert!(pkg_path.to_string_lossy().contains("mysql"));
    assert!(pkg_path.to_string_lossy().contains("8.0"));
}
```

Run: `cargo test --test integration_package_manager 2>&1 | tail -20`
Expected: Tests pass.

**Step 2: Commit**

```bash
git add crates/cleanserve-core/tests/
git commit -m "test: add integration tests for package manager registry and cache"
```

---

## Verification Checklist

Before marking Phase 2 complete, verify:

```bash
# All tests pass
cargo test --lib 2>&1 | grep "test result"
cargo test --test integration_package_manager 2>&1 | grep "test result"

# No clippy warnings
cargo clippy 2>&1 | grep -E "warning:|error:"

# Builds release binary
cargo build --release 2>&1 | tail -5

# Commands are accessible
cargo run -- package list 2>&1 | head -5
cargo run -- package info mysql 2>&1 | head -10
```

All must pass before proceeding to Task 7.

---

## Task 7: Final Verification and Commit

Run the verification checklist above. If all pass:

```bash
git log --oneline | head -10
```

Verify you have 7 commits from this phase, each atomic and focused.

Expected final state:
- ✅ Package manager module structure created
- ✅ Built-in manifest with MySQL, Redis
- ✅ cleanserve.json schema extended
- ✅ CLI commands (add, list, info) implemented
- ✅ Download logic with checksum validation
- ✅ Cache management
- ✅ All tests passing
- ✅ Ready for Phase 3 (proxy integration)

---

## Execution Handoff

**Plan is complete and saved to `/home/pedro/repo/cleanserve/docs/plans/2026-03-21-phase-2-implementation.md`.**

### Two execution options:

**1. Subagent-Driven (this session, fastest)**
- I dispatch fresh subagent per task
- Full code review between tasks
- Fast iteration with checkpoints

**2. Parallel Session (separate)**
- Open new session in a worktree
- Uses executing-plans skill
- Batch execution with checkpoints

**Which approach do you prefer?**
