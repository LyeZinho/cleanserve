use super::{Result, UpdateCheckerError};
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct BinaryDownloader;

fn err(message: impl Into<String>) -> UpdateCheckerError {
    UpdateCheckerError {
        message: message.into(),
    }
}

impl BinaryDownloader {
    /// Download `download_url` to `dest_path`, verifying it against the entry for the same
    /// file name in the `SHA256SUMS` document at `checksum_url`.
    /// On any failure (network, checksum mismatch) the partial file is removed.
    pub async fn download_release(
        download_url: &str,
        checksum_url: &str,
        dest_path: &Path,
    ) -> Result<()> {
        let expected_checksum = Self::fetch_checksum(checksum_url, download_url).await?;

        let result = Self::download_and_hash(download_url, dest_path).await;
        let actual_checksum = match result {
            Ok(sum) => sum,
            Err(e) => {
                let _ = tokio::fs::remove_file(dest_path).await;
                return Err(e);
            }
        };

        if actual_checksum != expected_checksum {
            let _ = tokio::fs::remove_file(dest_path).await;
            return Err(err(format!(
                "Checksum mismatch! Expected: {}, Actual: {}",
                expected_checksum, actual_checksum
            )));
        }
        Ok(())
    }

    fn client() -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .user_agent(format!("cleanserve-updater/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|e| err(format!("Failed to build HTTP client: {}", e)))
    }

    /// Stream the body to disk and return its lowercase hex SHA-256.
    async fn download_and_hash(download_url: &str, dest_path: &Path) -> Result<String> {
        use tokio::io::AsyncWriteExt;

        let mut res = Self::client()?
            .get(download_url)
            .send()
            .await
            .map_err(|e| err(format!("Failed to connect to download URL: {}", e)))?;

        if !res.status().is_success() {
            return Err(err(format!("Failed to download file: HTTP {}", res.status())));
        }

        let mut file = tokio::fs::File::create(dest_path)
            .await
            .map_err(|e| err(format!("Failed to create destination file: {}", e)))?;
        let mut hasher = Sha256::new();

        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|e| err(format!("Failed to download chunk: {}", e)))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| err(format!("Failed to write chunk: {}", e)))?;
            hasher.update(&chunk);
        }
        file.flush()
            .await
            .map_err(|e| err(format!("Failed to flush file: {}", e)))?;

        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn fetch_checksum(checksum_url: &str, download_url: &str) -> Result<String> {
        let filename = download_url
            .split(['?', '#'])
            .next()
            .and_then(|u| u.rsplit('/').next())
            .filter(|f| !f.is_empty())
            .ok_or_else(|| err("Invalid download URL"))?;

        let res = Self::client()?
            .get(checksum_url)
            .send()
            .await
            .map_err(|e| err(format!("Failed to connect to checksum URL: {}", e)))?;

        if !res.status().is_success() {
            return Err(err(format!(
                "Failed to download checksums: HTTP {}",
                res.status()
            )));
        }

        let text = res
            .text()
            .await
            .map_err(|e| err(format!("Failed to read checksums: {}", e)))?;

        Self::parse_checksum(&text, filename)
            .ok_or_else(|| err(format!("Checksum for {} not found in SHA256SUMS", filename)))
    }

    /// Find `filename` in `sha256sum`-style output (`<hash>  <name>` or `<hash> *<name>`).
    /// The name must match exactly: `cleanserve-linux-x86_64.tar.gz` must not match
    /// `cleanserve-linux-x86_64.tar.gz.sig`.
    pub fn parse_checksum(sums: &str, filename: &str) -> Option<String> {
        sums.lines().find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            let name = name.rsplit('/').next().unwrap_or(name);
            (name == filename && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| hash.to_ascii_lowercase())
        })
    }

    pub fn validate_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| err(format!("Failed to open file for checksum: {}", e)))?;

        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| err(format!("Failed to read file for checksum: {}", e)))?;

        let actual_checksum = format!("{:x}", hasher.finalize());
        if actual_checksum != expected_checksum.to_ascii_lowercase() {
            return Err(err(format!(
                "Checksum mismatch! Expected: {}, Actual: {}",
                expected_checksum, actual_checksum
            )));
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
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    /// Minimal local HTTP server: serves `routes` (path -> body), 404 otherwise.
    async fn serve(routes: HashMap<&'static str, Vec<u8>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { return };
                let routes = routes.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let (status, body) = match routes.get(path.as_str()) {
                        Some(b) => ("200 OK", b.clone()),
                        None => ("404 Not Found", Vec::new()),
                    };
                    let head = format!(
                        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        status,
                        body.len()
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(&body).await;
                });
            }
        });
        format!("http://{}", addr)
    }

    fn sha(data: &[u8]) -> String {
        format!("{:x}", Sha256::digest(data))
    }

    #[test]
    fn test_platform_detection() {
        let platform = BinaryDownloader::get_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains('-'));
    }

    #[test]
    fn parse_checksum_requires_exact_name() {
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let sums = format!("{a}  cleanserve-linux-x86_64.tar.gz.sig\n{b} *cleanserve-linux-x86_64.tar.gz\n");
        assert_eq!(
            BinaryDownloader::parse_checksum(&sums, "cleanserve-linux-x86_64.tar.gz"),
            Some(b)
        );
        assert_eq!(BinaryDownloader::parse_checksum(&sums, "missing"), None);
        assert_eq!(BinaryDownloader::parse_checksum("nothex  file", "file"), None);
    }

    #[tokio::test]
    async fn test_download_success() {
        let payload = b"pretend this is a binary".to_vec();
        let sums = format!("{}  cleanserve.tar.gz\n", sha(&payload));
        let base = serve(HashMap::from([
            ("/cleanserve.tar.gz", payload.clone()),
            ("/SHA256SUMS", sums.into_bytes()),
        ]))
        .await;

        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("dl");
        BinaryDownloader::download_release(
            &format!("{base}/cleanserve.tar.gz"),
            &format!("{base}/SHA256SUMS"),
            &dest,
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
    }

    #[tokio::test]
    async fn test_download_mismatch_removes_file() {
        let sums = format!("{}  cleanserve.tar.gz\n", "0".repeat(64));
        let base = serve(HashMap::from([
            ("/cleanserve.tar.gz", b"tampered".to_vec()),
            ("/SHA256SUMS", sums.into_bytes()),
        ]))
        .await;

        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("dl");
        let e = BinaryDownloader::download_release(
            &format!("{base}/cleanserve.tar.gz"),
            &format!("{base}/SHA256SUMS"),
            &dest,
        )
        .await
        .unwrap_err();
        assert!(e.message.contains("Checksum mismatch"), "{}", e.message);
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn test_download_http_error_removes_file() {
        let sums = format!("{}  gone.tar.gz\n", EMPTY_SHA256);
        let base = serve(HashMap::from([("/SHA256SUMS", sums.into_bytes())])).await;

        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("dl");
        let e = BinaryDownloader::download_release(
            &format!("{base}/gone.tar.gz"),
            &format!("{base}/SHA256SUMS"),
            &dest,
        )
        .await
        .unwrap_err();
        assert!(e.message.contains("404"), "{}", e.message);
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn test_download_fails_when_checksum_missing() {
        let base = serve(HashMap::from([
            ("/a.tar.gz", b"x".to_vec()),
            ("/SHA256SUMS", b"".to_vec()),
        ]))
        .await;
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("dl");
        let e = BinaryDownloader::download_release(
            &format!("{base}/a.tar.gz"),
            &format!("{base}/SHA256SUMS"),
            &dest,
        )
        .await
        .unwrap_err();
        assert!(e.message.contains("not found in SHA256SUMS"), "{}", e.message);
        assert!(!dest.exists());
    }

    #[test]
    fn test_validate_checksum() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("empty");
        std::fs::File::create(&file_path).unwrap();

        assert!(BinaryDownloader::validate_checksum(&file_path, EMPTY_SHA256).is_ok());
        assert!(BinaryDownloader::validate_checksum(&file_path, &EMPTY_SHA256.to_uppercase()).is_ok());
        assert!(BinaryDownloader::validate_checksum(&file_path, "bad").is_err());
    }
}
