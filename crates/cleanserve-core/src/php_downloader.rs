use crate::php_manager::{find_php_binary, PhpManager};
use crate::{CleanServeError, Result, UrlValidator, VersionManifest};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct PhpDownloader {
    base_dir: PathBuf,
}

impl PhpDownloader {
    /// Create a downloader that installs into `base_dir/php-{version}/`.
    pub fn new(base_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_dir)
            .map_err(|e| CleanServeError::Config(format!("Cannot create PHP directory: {}", e)))?;

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Install into the shared global cache (`~/.cleanserve/php`).
    pub fn global() -> Result<Self> {
        Self::new(&PhpManager::global_php_dir()?)
    }

    pub fn get_install_path(&self, version: &str) -> PathBuf {
        self.base_dir.join(format!("php-{}", version))
    }

    pub fn is_installed(&self, version: &str) -> bool {
        self.get_php_exe(version).is_some()
    }

    pub fn get_php_exe(&self, version: &str) -> Option<PathBuf> {
        PhpManager::with_search_dirs(vec![self.base_dir.clone()]).get_path(version)
    }

    /// Download and install PHP using the version manifest.
    /// `version` can be a minor version ("8.4") or exact ("8.4.19").
    pub async fn download(&self, version: &str) -> Result<PathBuf> {
        let manifest = VersionManifest::fetch(false).await?;

        let php_version = manifest.resolve_version(version).ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} is not available. Run 'cleanserve list' to see installable versions.",
                version
            ))
        })?;

        let resolved = php_version.version.clone();
        if resolved != version {
            info!(
                "PHP {} not in manifest; using compatible build {}",
                version, resolved
            );
        }

        if let Some(existing) = self.get_php_exe(&resolved) {
            info!("PHP {} is already installed", resolved);
            return Ok(existing);
        }

        let platform = current_platform();
        let binary = manifest
            .get_platform_binary(&resolved, platform)
            .ok_or_else(|| {
                CleanServeError::Download(format!(
                    "PHP {} is not available for platform '{}'.",
                    resolved, platform
                ))
            })?;

        info!(
            "Downloading PHP {} ({:.1} MB)...",
            resolved,
            binary.size_bytes as f64 / 1_048_576.0
        );
        info!("URL: {}", binary.download_url);

        let validator = UrlValidator::new();
        validator
            .validate_url(&binary.download_url)
            .map_err(|e| CleanServeError::Download(format!("URL validation failed: {:?}", e)))?;

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .map_err(|e| CleanServeError::Download(format!("HTTP client error: {}", e)))?;

        let response = client
            .get(&binary.download_url)
            .send()
            .await
            .map_err(|e| CleanServeError::Download(format!("Failed to download: {}", e)))?;

        if !response.status().is_success() {
            return Err(CleanServeError::Download(format!(
                "Download failed with status: {} from {}",
                response.status(),
                binary.download_url
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| CleanServeError::Download(format!("Failed to read response: {}", e)))?;

        info!("Verifying SHA256 checksum...");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let computed = format!("{:x}", hasher.finalize());

        if !computed.eq_ignore_ascii_case(&binary.sha256) {
            return Err(CleanServeError::Download(format!(
                "SHA256 mismatch!\n  Expected: {}\n  Got:      {}\nDownload may be corrupted. Try again.",
                binary.sha256, computed
            )));
        }
        info!("SHA256 verified OK");

        let temp_tarball = std::env::temp_dir().join(format!("php-{}.tar.gz", resolved));
        std::fs::write(&temp_tarball, &bytes)?;

        let install_path = self.get_install_path(&resolved);
        if install_path.exists() {
            let _ = std::fs::remove_dir_all(&install_path);
        }
        std::fs::create_dir_all(&install_path)?;

        info!("Extracting to {}...", install_path.display());

        let output = tokio::process::Command::new("tar")
            .args([
                "-xzf",
                temp_tarball.to_str().unwrap(),
                "-C",
                install_path.to_str().unwrap(),
            ])
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

        let php_bin = find_php_binary(&install_path).ok_or_else(|| {
            let _ = std::fs::remove_dir_all(&install_path);
            CleanServeError::Download(format!(
                "PHP {} extracted but no CLI binary was found in {}",
                resolved,
                install_path.display()
            ))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&php_bin)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&php_bin, perms)?;
        }

        let stable = install_path.join(if cfg!(windows) { "php.exe" } else { "php" });
        if php_bin != stable && !stable.exists() {
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(&php_bin, &stable);
            }
            #[cfg(windows)]
            {
                let _ = std::fs::copy(&php_bin, &stable);
            }
        }

        let _ = std::fs::remove_file(&temp_tarball);

        info!(
            "PHP {} installed successfully at {}",
            resolved,
            php_bin.display()
        );
        Ok(php_bin)
    }

    /// Return an existing binary or download it into this downloader's base dir.
    pub async fn ensure(&self, version: &str) -> Result<PathBuf> {
        if let Some(existing) = PhpManager::new()
            .ok()
            .and_then(|m| m.get_path(version))
            .or_else(|| self.get_php_exe(version))
        {
            return Ok(existing);
        }
        self.download(version).await
    }
}

pub fn current_platform() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux-x64"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macos-arm64"
        } else {
            "macos-x64"
        }
    } else if cfg!(target_os = "windows") {
        "windows-x64"
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
