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
        let candidates = vec![
            path.join("php"),
            path.join("bin").join("php"),
        ];
        #[cfg(windows)]
        let candidates = vec![
            path.join("php.exe"),
            path.join("bin").join("php.exe"),
        ];

        candidates.iter().any(|p| p.exists())
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

    #[cfg(not(windows))]
    pub async fn download(&self, version: &str) -> Result<()> {
        use std::io::Write;
        use tokio::process::Command;

        if self.is_installed(version) {
            info!("PHP {} is already installed", version);
            return Ok(());
        }

        let install_path = self.get_install_path(version);

        info!("Downloading PHP {}...", version);

        // Detect architecture
        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            return Err(CleanServeError::Download("Unsupported architecture".into()));
        };

        // Find the latest patch version for the requested minor version
        // e.g., "8.4" -> find "8.4.19" from dl.static-php.dev
        let full_version = Self::find_latest_patch_version(version, arch).await?;

        // Use static-php-cli pre-built binaries from dl.static-php.dev
        let url = format!(
            "https://dl.static-php.dev/static-php-cli/bulk/php-{ver}-cli-linux-{arch}.tar.gz",
            ver = full_version,
            arch = arch,
        );

        info!("Downloading from: {}", url);

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| CleanServeError::Download(format!("Failed to create client: {}", e)))?;

        let response = client.get(&url).send().await
            .map_err(|e| CleanServeError::Download(format!("Failed to download: {}", e)))?;

        if !response.status().is_success() {
            return Err(CleanServeError::Download(format!(
                "Download failed with status: {} from {}",
                response.status(), url
            )));
        }

        let bytes = response.bytes().await
            .map_err(|e| CleanServeError::Download(format!("Failed to read response: {}", e)))?;

        info!("Extracting PHP {} ({}MB)...", full_version, bytes.len() / 1024 / 1024);

        // Write to temp file
        let temp_tarball = std::env::temp_dir().join(format!("php-{}.tar.gz", full_version));
        let mut file = std::fs::File::create(&temp_tarball)?;
        file.write_all(&bytes)?;

        // Extract
        std::fs::create_dir_all(&install_path)?;

        let output = Command::new("tar")
            .args(["-xzf", temp_tarball.to_str().unwrap(), "-C", install_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| CleanServeError::Download(format!("Failed to extract: {}", e)))?;

        if !output.status.success() {
            return Err(CleanServeError::Download(format!(
                "tar extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Make PHP executable
        let php_exe = install_path.join("php");
        if php_exe.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&php_exe)
                    .map_err(|e| CleanServeError::Download(format!("{}", e)))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&php_exe, perms)
                    .map_err(|e| CleanServeError::Download(format!("{}", e)))?;
            }
        }

        // Cleanup
        let _ = std::fs::remove_file(&temp_tarball);

        info!("✅ PHP {} installed successfully at {}", full_version, install_path.display());
        Ok(())
    }

    /// Find the latest patch version for a given minor version from static-php.dev
    async fn find_latest_patch_version(minor_version: &str, arch: &str) -> Result<String> {
        let base_url = "https://dl.static-php.dev/static-php-cli/bulk/";

        let client = reqwest::Client::new();
        let html = client.get(base_url).send().await
            .map_err(|e| CleanServeError::Download(format!("Failed to fetch version list: {}", e)))?
            .text().await
            .map_err(|e| CleanServeError::Download(format!("Failed to read version list: {}", e)))?;

        // Parse HTML to find matching versions like php-8.4.X-cli-linux-x86_64.tar.gz
        let escaped_version = minor_version.replace('.', r"\.");
        let pattern = format!(r"php-{}\.(\d+)-cli-linux-{}\.tar\.gz", escaped_version, arch);
        let re = regex::Regex::new(&pattern)
            .map_err(|e| CleanServeError::Download(format!("Regex error: {}", e)))?;

        let mut patches: Vec<u32> = re.captures_iter(&html)
            .filter_map(|cap| cap.get(1)?.as_str().parse().ok())
            .collect();

        if patches.is_empty() {
            return Err(CleanServeError::Download(
                format!("No PHP {} binary found for {}", minor_version, arch)
            ));
        }

        patches.sort();
        let latest_patch = patches.last().unwrap();
        let full_version = format!("{}.{}", minor_version, latest_patch);
        info!("Found latest PHP {} patch: {}", minor_version, full_version);
        Ok(full_version)
    }

    /// Get the path to the PHP executable
    pub fn get_php_exe(&self, version: &str) -> Option<PathBuf> {
        if !self.is_installed(version) {
            return None;
        }

        let path = self.get_install_path(version);
        let candidates = vec![
            path.join("php"),
            path.join("bin").join("php"),
        ];
        #[cfg(windows)]
        let candidates = vec![
            path.join("php.exe"),
            path.join("bin").join("php.exe"),
        ];

        candidates.into_iter().find(|p| p.exists())
    }
}
