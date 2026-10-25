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
        self.php_exe_candidates(version)
            .into_iter()
            .find(|p| p.exists())
    }

    /// Candidate paths for the PHP binary
    fn php_exe_candidates(&self, version: &str) -> Vec<PathBuf> {
        let path = self.get_install_path(version);
        let mut candidates = vec![path.join("php"), path.join("bin").join("php")];

        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let platform_path = entry.path();
                if !platform_path.is_dir() {
                    continue;
                }
                let direct = platform_path.join("php");
                if direct.is_file() {
                    candidates.push(direct);
                }
                let nested_bin = platform_path.join("bin").join("php");
                if nested_bin.is_file() {
                    candidates.push(nested_bin);
                }
                if let Ok(bin_entries) = std::fs::read_dir(&platform_path) {
                    for bin_entry in bin_entries.flatten() {
                        let bin_path = bin_entry.path();
                        if bin_path.is_file() {
                            let name = bin_entry.file_name().to_string_lossy().into_owned();
                            if name.starts_with("php") {
                                candidates.push(bin_path);
                            }
                        }
                    }
                }
            }
        }

        candidates
    }

    /// Download and install PHP using the version manifest.
    /// `version` can be a minor version ("8.4") or exact ("8.4.19").
    pub async fn download(&self, version: &str) -> Result<()> {
        let manifest = VersionManifest::fetch(false).await?;

        let php_version = manifest.resolve_version(version).ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} is not available. Run 'cleanserve list' to see installable versions.",
                version
            ))
        })?;

        let resolved = &php_version.version;
        if resolved != version {
            info!(
                "PHP {} not in manifest; using compatible build {}",
                version,
                resolved
            );
        }

        if self.is_installed(resolved) {
            info!("PHP {} is already installed", resolved);
            return Ok(());
        }

        let platform = current_platform();
        let binary = manifest.get_platform_binary(resolved, platform).ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} is not available for platform '{}'. Only Linux is supported currently.",
                resolved, platform
            ))
        })?;

        info!("Downloading PHP {} ({:.1} MB)...", resolved, binary.size_bytes as f64 / 1_048_576.0);
        info!("URL: {}", binary.download_url);

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

        let temp_tarball = std::env::temp_dir().join(format!("php-{}.tar.gz", resolved));
        std::fs::write(&temp_tarball, &bytes)?;

        let install_path = self.get_install_path(resolved);
        std::fs::create_dir_all(&install_path)?;

        info!("Extracting to {}...", install_path.display());

        let output = tokio::process::Command::new("tar")
            .args(["-xzf", temp_tarball.to_str().unwrap(), "-C", install_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| CleanServeError::Download(format!("Failed to extract: {}", e)))?;

        if !output.status.success() {
            let _ = std::fs::remove_dir_all(&install_path);
            return Err(CleanServeError::Download(format!(
                "tar extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn locates_linux_x64_php_layout() {
        let dir = TempDir::new().unwrap();
        let install = dir.path().join("php-8.4.19").join("linux-x64");
        fs::create_dir_all(&install).unwrap();
        let bin = install.join("php-8.4.19");
        fs::write(&bin, b"#!/bin/sh\n").unwrap();

        let downloader = PhpDownloader::new(dir.path()).unwrap();
        let found = downloader.get_php_exe("8.4.19").expect("binary should be found");
        assert_eq!(found, bin);
    }
}
