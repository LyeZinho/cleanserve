use super::{Result, UpdateInfo, UpdateCheckerError};
use semver::Version;

pub struct UpdateChecker;

impl UpdateChecker {
    pub async fn check_for_updates(current_version: &str) -> Result<UpdateInfo> {
        let latest = Self::fetch_latest_release().await?;
        
        let current = Version::parse(current_version)
            .map_err(|e| UpdateCheckerError {
                message: format!("Invalid current version: {}", e),
            })?;
        
        let latest_version = Version::parse(&latest)
            .map_err(|e| UpdateCheckerError {
                message: format!("Invalid latest version from API: {}", e),
            })?;
        
        let needs_update = latest_version > current;
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest.clone(),
            download_url: format!(
                "https://github.com/LyeZinho/cleanserve/releases/download/v{}/cleanserve",
                latest
            ),
            checksum_url: format!(
                "https://github.com/LyeZinho/cleanserve/releases/download/v{}/SHA256SUMS",
                latest
            ),
            needs_update,
        })
    }

    async fn fetch_latest_release() -> Result<String> {
        // Placeholder: In Phase 4b, integrate with GitHub API
        // For now, return mock version
        Ok("0.3.1".to_string())
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
}
