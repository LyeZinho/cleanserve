use crate::{CleanServeError, Result};
use std::path::PathBuf;
use tracing::info;

pub struct PhpDownloader {
    cache_dir: PathBuf,
}

impl PhpDownloader {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
            .join(".cleanserve")
            .join("bin");
        
        std::fs::create_dir_all(&cache_dir)?;
        
        Ok(Self { cache_dir })
    }

    /// Get the install path for a PHP version
    pub fn get_install_path(&self, version: &str) -> PathBuf {
        self.cache_dir.join(format!("php-{}", version))
    }

    /// Check if PHP is already installed
    pub fn is_installed(&self, version: &str) -> bool {
        let path = self.get_install_path(version);
        #[cfg(windows)]
        let exe = path.join("php.exe");
        #[cfg(not(windows))]
        let exe = path.join("bin").join("php");
        
        exe.exists()
    }

    /// Download and install PHP for Windows
    #[cfg(windows)]
    pub async fn download(&self, version: &str) -> Result<()> {
        let install_path = self.get_install_path(version);
        
        if self.is_installed(version) {
            info!("PHP {} is already installed", version);
            return Ok(());
        }

        info!("Downloading PHP {}...", version);
        
        // PHP Windows download URL (example for PHP 8.4)
        let url = format!(
            "https://windows.php.net/downloads/releases/php-{}-Win32-vs17-x64.zip",
            version
        );
        
        let client = reqwest::Client::new();
        let response = client.get(&url).send().await
            .map_err(|e| CleanServeError::Download(format!("Failed to download: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(CleanServeError::Download(format!(
                "Download failed with status: {}", response.status()
            )));
        }
        
        let bytes = response.bytes().await
            .map_err(|e| CleanServeError::Download(format!("Failed to read response: {}", e)))?;
        
        info!("Extracting PHP {}...", version);
        
        // Create extraction directory
        std::fs::create_dir_all(&install_path)?;
        
        // Extract ZIP
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| CleanServeError::Download(format!("Failed to read ZIP: {}", e)))?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| CleanServeError::Download(format!("Failed to read file in ZIP: {}", e)))?;
            
            let outpath = match file.enclosed_name() {
                Some(path) => install_path.join(path),
                None => continue,
            };
            
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        
        info!("✅ PHP {} installed successfully at {}", version, install_path.display());
        Ok(())
    }

    /// Download and install PHP for Unix/Linux/macOS
    #[cfg(not(windows))]
    pub async fn download(&self, version: &str) -> Result<()> {
        use std::io::Write;
        use tokio::process::Command;
        
        let install_path = self.get_install_path(version);
        
        if self.is_installed(version) {
            info!("PHP {} is already installed", version);
            return Ok(());
        }

        info!("Downloading PHP {}...", version);
        
        // For Unix, download from php.net
        let url = format!(
            "https://www.php.net/distributions/php-{}.tar.gz",
            version
        );
        
        let client = reqwest::Client::new();
        let response = client.get(&url).send().await
            .map_err(|e| CleanServeError::Download(format!("Failed to download: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(CleanServeError::Download(format!(
                "Download failed with status: {}", response.status()
            )));
        }
        
        let bytes = response.bytes().await
            .map_err(|e| CleanServeError::Download(format!("Failed to read response: {}", e)))?;
        
        info!("Extracting PHP {}...", version);
        
        // Write to temp file
        let temp_tarball = std::env::temp_dir().join("php.tar.gz");
        let mut file = std::fs::File::create(&temp_tarball)?;
        file.write_all(&bytes)?;
        
        // Extract
        std::fs::create_dir_all(&install_path)?;
        
        let output = Command::new("tar")
            .args(["-xzf", temp_tarball.to_str().unwrap(), "-C", install_path.to_str().unwrap(), "--strip-components=1"])
            .output()
            .await
            .map_err(|e| CleanServeError::Download(format!("Failed to extract: {}", e)))?;
        
        if !output.status.success() {
            return Err(CleanServeError::Download(format!(
                "tar extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_tarball);
        
        info!("✅ PHP {} installed successfully", version);
        Ok(())
    }

    /// Get the path to the PHP executable
    pub fn get_php_exe(&self, version: &str) -> Option<PathBuf> {
        if !self.is_installed(version) {
            return None;
        }
        
        let path = self.get_install_path(version);
        #[cfg(windows)]
        let exe = path.join("php.exe");
        #[cfg(not(windows))]
        let exe = path.join("bin").join("php");
        
        if exe.exists() {
            Some(exe)
        } else {
            None
        }
    }
}
