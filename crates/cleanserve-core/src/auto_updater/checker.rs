use super::{Result, UpdateInfo, UpdateCheckerError};
use semver::Version;
use serde::Deserialize;

pub struct UpdateChecker;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

impl UpdateChecker {
    pub async fn check_for_updates(current_version: &str) -> Result<UpdateInfo> {
        let release = Self::fetch_latest_release().await?;
        
        let latest = release.tag_name.trim_start_matches('v').to_string();
        
        let current = Version::parse(current_version)
            .map_err(|e| UpdateCheckerError {
                message: format!("Invalid current version '{}': {}", current_version, e),
            })?;
        
        let latest_version = Version::parse(&latest)
            .map_err(|e| UpdateCheckerError {
                message: format!("Invalid latest version from API '{}': {}", latest, e),
            })?;
        
        let needs_update = latest_version > current;

        let platform = Self::detect_platform()?;
        let asset_name = format!("cleanserve-{}", platform);
        
        let download_url = release.assets.iter()
            .find(|a| a.name.starts_with(&asset_name) || a.name.contains(&platform))
            .map(|a| a.browser_download_url.clone())
            .unwrap_or_else(|| format!("https://github.com/LyeZinho/cleanserve/releases/download/v{}/{}", latest, asset_name));
            
        let checksum_url = format!(
            "https://github.com/LyeZinho/cleanserve/releases/download/v{}/SHA256SUMS",
            latest
        );
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest.clone(),
            download_url,
            checksum_url,
            needs_update,
        })
    }

    #[cfg(not(test))]
    async fn fetch_latest_release() -> Result<GitHubRelease> {
        let client = reqwest::Client::builder()
            .user_agent(format!("cleanserve-updater/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| UpdateCheckerError { message: format!("Failed to build HTTP client: {}", e) })?;

        let response = client
            .get("https://api.github.com/repos/LyeZinho/cleanserve/releases/latest")
            .send()
            .await
            .map_err(|e| UpdateCheckerError { message: format!("Network error: {}", e) })?;

        if !response.status().is_success() {
            return Err(UpdateCheckerError { message: format!("GitHub API returned error: {}", response.status()) });
        }

        response.json::<GitHubRelease>()
            .await
            .map_err(|e| UpdateCheckerError { message: format!("Failed to parse GitHub API response: {}", e) })
    }

    #[cfg(test)]
    async fn fetch_latest_release() -> Result<GitHubRelease> {
        Ok(GitHubRelease {
            tag_name: "v0.3.1".to_string(),
            assets: vec![
                GitHubAsset {
                    name: format!("cleanserve-{}", Self::detect_platform().unwrap_or_else(|_| "unknown".to_string())),
                    browser_download_url: "http://example.com/download".to_string()
                }
            ]
        })
    }

    pub fn detect_platform() -> Result<String> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        let os_str = match os {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            _ => return Err(UpdateCheckerError { message: format!("Unsupported OS: {}", os) }),
        };

        let arch_str = match arch {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            _ => return Err(UpdateCheckerError { message: format!("Unsupported architecture: {}", arch) }),
        };

        Ok(format!("{}-{}", os_str, arch_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version_comparison() {
        let info = UpdateChecker::check_for_updates("0.3.0").await.unwrap();
        assert!(info.needs_update);
        assert_eq!(info.latest_version, "0.3.1");
    }

    #[tokio::test]
    async fn test_no_update_needed() {
        let info = UpdateChecker::check_for_updates("0.3.1").await.unwrap();
        assert!(!info.needs_update);
    }
    
    #[test]
    fn test_platform_detection() {
        let platform = UpdateChecker::detect_platform();
        assert!(platform.is_ok());
        let platform_str = platform.unwrap();
        assert!(platform_str.contains("-"));
    }
}
