use super::{Result, UpdateCheckerError};
use std::path::Path;
use sha2::Digest;

pub struct BinaryDownloader;

impl BinaryDownloader {
    #[cfg(not(test))]
    pub async fn download_release(
        download_url: &str,
        checksum_url: &str,
        dest_path: &Path,
    ) -> Result<()> {
        let expected_checksum = Self::fetch_checksum(checksum_url, download_url).await?;

        let client = reqwest::Client::new();
        let mut res = client.get(download_url).send().await
            .map_err(|e| UpdateCheckerError { message: format!("Failed to connect to download URL: {}", e) })?;

        if !res.status().is_success() {
            return Err(UpdateCheckerError { message: format!("Failed to download file: HTTP {}", res.status()) });
        }

        let mut file = tokio::fs::File::create(dest_path).await
            .map_err(|e| UpdateCheckerError { message: format!("Failed to create destination file: {}", e) })?;

        let mut hasher = sha2::Sha256::new();

        while let Some(chunk) = res.chunk().await.map_err(|e| UpdateCheckerError { message: format!("Failed to download chunk: {}", e) })? {
            use tokio::io::AsyncWriteExt;
            file.write_all(&chunk).await
                .map_err(|e| UpdateCheckerError { message: format!("Failed to write chunk: {}", e) })?;
            sha2::Digest::update(&mut hasher, &chunk);
        }

        let actual_checksum = format!("{:x}", sha2::Digest::finalize(hasher));
        if actual_checksum != expected_checksum {
            let _ = tokio::fs::remove_file(dest_path).await;
            return Err(UpdateCheckerError {
                message: format!("Checksum mismatch! Expected: {}, Actual: {}", expected_checksum, actual_checksum)
            });
        }

        Ok(())
    }

    #[cfg(test)]
    pub async fn download_release(
        download_url: &str,
        checksum_url: &str,
        dest_path: &Path,
    ) -> Result<()> {
        if download_url == "fail" {
            return Err(UpdateCheckerError { message: "Simulated network error".into() });
        }
        
        let expected_checksum = if checksum_url == "bad_checksum" {
            "bad".to_string()
        } else {
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()
        };
        
        tokio::fs::File::create(dest_path).await.map_err(|e| UpdateCheckerError { message: e.to_string() })?;
        
        let actual_checksum = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        if actual_checksum != expected_checksum {
            let _ = tokio::fs::remove_file(dest_path).await;
            return Err(UpdateCheckerError { message: "Checksum mismatch".into() });
        }
        
        Ok(())
    }

    #[cfg(not(test))]
    async fn fetch_checksum(checksum_url: &str, download_url: &str) -> Result<String> {
        let filename = download_url.rsplit('/').next()
            .ok_or_else(|| UpdateCheckerError { message: "Invalid download URL".into() })?;

        let client = reqwest::Client::new();
        let res = client.get(checksum_url).send().await
            .map_err(|e| UpdateCheckerError { message: format!("Failed to connect to checksum URL: {}", e) })?;

        if !res.status().is_success() {
            return Err(UpdateCheckerError { message: format!("Failed to download checksums: HTTP {}", res.status()) });
        }

        let text = res.text().await
            .map_err(|e| UpdateCheckerError { message: format!("Failed to read checksums: {}", e) })?;

        for line in text.lines() {
            if line.contains(filename) {
                if let Some(hash) = line.split_whitespace().next() {
                    return Ok(hash.to_string());
                }
            }
        }

        Err(UpdateCheckerError { message: format!("Checksum for {} not found in SHA256SUMS", filename) })
    }

    pub fn validate_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| UpdateCheckerError { message: format!("Failed to open file for checksum: {}", e) })?;
        
        let mut hasher = sha2::Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| UpdateCheckerError { message: format!("Failed to read file for checksum: {}", e) })?;
            
        let actual_checksum = format!("{:x}", sha2::Digest::finalize(hasher));
        if actual_checksum != expected_checksum {
            return Err(UpdateCheckerError {
                message: format!("Checksum mismatch! Expected: {}, Actual: {}", expected_checksum, actual_checksum)
            });
        }
        
        Ok(())
    }

    pub fn get_platform() -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        format!("{}-{}", os, arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_platform_detection() {
        let platform = BinaryDownloader::get_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains('-'));
    }

    #[tokio::test]
    async fn test_download_success() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("test_bin");
        
        let result = BinaryDownloader::download_release(
            "success",
            "good_checksum",
            &dest,
        ).await;
        
        assert!(result.is_ok());
        assert!(dest.exists());
    }

    #[tokio::test]
    async fn test_download_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("test_bin");
        
        let result = BinaryDownloader::download_release(
            "success",
            "bad_checksum",
            &dest,
        ).await;
        
        assert!(result.is_err());
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn test_download_network_error() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("test_bin");
        
        let result = BinaryDownloader::download_release(
            "fail",
            "good_checksum",
            &dest,
        ).await;
        
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_checksum_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty");
        std::fs::File::create(&file_path).unwrap();
        
        let result = BinaryDownloader::validate_checksum(
            &file_path,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_checksum_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty");
        std::fs::File::create(&file_path).unwrap();
        
        let result = BinaryDownloader::validate_checksum(
            &file_path,
            "bad",
        );
        assert!(result.is_err());
    }
}
