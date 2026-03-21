use super::{PackageManagerError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Downloads and verifies packages
pub struct PackageDownloader;

impl PackageDownloader {
    /// Validate checksum format (sha256:abc123...)
    pub fn validate_checksum_format(checksum: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = checksum.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(PackageManagerError {
                message: format!(
                    "Invalid checksum format: {}. Expected 'algorithm:hash'",
                    checksum
                ),
            });
        }

        let algorithm = parts[0];
        let hash = parts[1];

        if algorithm != "sha256" {
            return Err(PackageManagerError {
                message: format!(
                    "Unsupported checksum algorithm: {}. Only 'sha256' supported",
                    algorithm
                ),
            });
        }

        if hash.len() != 64 {
            return Err(PackageManagerError {
                message: format!(
                    "Invalid SHA256 hash length: {}. Expected 64 hex chars",
                    hash.len()
                ),
            });
        }

        Ok((algorithm.to_string(), hash.to_lowercase()))
    }

    /// Compute SHA256 of file
    pub fn compute_sha256(path: &Path) -> Result<String> {
        let file = std::fs::File::open(path).map_err(|e| PackageManagerError {
            message: format!("Cannot read file for checksum: {}", e),
        })?;

        let mut hasher = Sha256::new();
        let mut reader = std::io::BufReader::new(file);
        use std::io::Read;
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|e| PackageManagerError {
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

    /// Download package with checksum validation
    pub async fn download_with_verification(url: &str, dest_path: &Path, expected_checksum: &str) -> Result<()> {
        Self::verify_checksum_format(expected_checksum)?;
        
        let client = reqwest::Client::new();
        let response = client
            .get(url)
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

        let bytes = response
            .bytes()
            .await
            .map_err(|e| PackageManagerError {
                message: format!("Error reading response: {}", e),
            })?;

        std::fs::write(dest_path, &bytes)
            .map_err(|e| PackageManagerError {
                message: format!("Cannot write file: {}", e),
            })?;

        Self::verify_checksum(dest_path, expected_checksum)?;

        Ok(())
    }

    /// Helper to verify checksum format
    fn verify_checksum_format(checksum: &str) -> Result<()> {
        let (_algorithm, _hash) = Self::validate_checksum_format(checksum)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_checksum_format_valid() {
        let result = PackageDownloader::validate_checksum_format(
            "sha256:abc123abc123abc123abc123abc123abc123abc123abc123abc123abc123abc1",
        );
        assert!(result.is_ok());
        let (algo, hash) = result.unwrap();
        assert_eq!(algo, "sha256");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_validate_checksum_format_invalid_algorithm() {
        let result = PackageDownloader::validate_checksum_format("md5:abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_checksum_format_invalid_length() {
        let result = PackageDownloader::validate_checksum_format("sha256:tooshort");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_sha256() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let hash = PackageDownloader::compute_sha256(&file_path).unwrap();
        // Known SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_get_platform() {
        let platform = PackageDownloader::get_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains('-'));
    }
}
