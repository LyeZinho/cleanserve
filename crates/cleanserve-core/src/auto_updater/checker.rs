use super::{Result, UpdateCheckerError, UpdateInfo};
use semver::Version;
use serde::Deserialize;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/LyeZinho/cleanserve/releases/latest";

pub struct UpdateChecker;

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

fn err(message: impl Into<String>) -> UpdateCheckerError {
    UpdateCheckerError {
        message: message.into(),
    }
}

impl UpdateChecker {
    /// Ask GitHub for the latest release and compare it with `current_version`.
    pub async fn check_for_updates(current_version: &str) -> Result<UpdateInfo> {
        let release = Self::fetch_latest_release().await?;
        Self::evaluate(current_version, &release)
    }

    /// Pure comparison of `current_version` against a release (no network).
    pub fn evaluate(current_version: &str, release: &GitHubRelease) -> Result<UpdateInfo> {
        let latest = release.tag_name.trim_start_matches('v').to_string();

        let current = Version::parse(current_version).map_err(|e| {
            err(format!(
                "Invalid current version '{}': {}",
                current_version, e
            ))
        })?;
        let latest_version = Version::parse(&latest).map_err(|e| {
            err(format!(
                "Invalid latest version from API '{}': {}",
                latest, e
            ))
        })?;

        let asset_name = Self::asset_name()?;
        let download_url = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .map(|a| a.browser_download_url.clone());
        let checksum_url = release
            .assets
            .iter()
            .find(|a| a.name == "SHA256SUMS")
            .map(|a| a.browser_download_url.clone());

        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest,
            download_url,
            checksum_url,
            needs_update: latest_version > current,
        })
    }

    async fn fetch_latest_release() -> Result<GitHubRelease> {
        let client = reqwest::Client::builder()
            .user_agent(format!("cleanserve-updater/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| err(format!("Failed to build HTTP client: {}", e)))?;

        let mut request = client
            .get(LATEST_RELEASE_URL)
            .header("Accept", "application/vnd.github+json");
        // Optional: lifts the anonymous API rate limit (60 requests/hour per IP).
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                request = request.bearer_auth(token);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| err(format!("Network error: {}", e)))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(err("No releases have been published yet"));
        }
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            return Err(err(format!(
                "GitHub API refused the request ({}), most likely a rate limit. \
                 Try again later or set GITHUB_TOKEN.",
                status
            )));
        }
        if !status.is_success() {
            return Err(err(format!("GitHub API returned error: {}", status)));
        }

        response
            .json::<GitHubRelease>()
            .await
            .map_err(|e| err(format!("Failed to parse GitHub API response: {}", e)))
    }

    /// Platform label used in release asset names, e.g. `linux-x86_64`, `darwin-aarch64`.
    /// Must stay in sync with `.github/workflows/release.yml` and `install.sh`.
    pub fn detect_platform() -> Result<String> {
        let os = match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            other => return Err(err(format!("Unsupported OS: {}", other))),
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            other => return Err(err(format!("Unsupported architecture: {}", other))),
        };
        Ok(format!("{}-{}", os, arch))
    }

    /// File name of the release archive for this platform.
    pub fn asset_name() -> Result<String> {
        let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
        Ok(format!("cleanserve-{}.{}", Self::detect_platform()?, ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> GitHubRelease {
        let name = UpdateChecker::asset_name().unwrap();
        serde_json::from_value(serde_json::json!({
            "tag_name": tag,
            "assets": [
                { "name": name, "browser_download_url": "https://example.com/archive" },
                { "name": "SHA256SUMS", "browser_download_url": "https://example.com/sums" },
                { "name": "unrelated.txt", "browser_download_url": "https://example.com/x" }
            ]
        }))
        .unwrap()
    }

    #[test]
    fn newer_release_needs_update() {
        let info = UpdateChecker::evaluate("0.3.0", &release("v0.3.1")).unwrap();
        assert!(info.needs_update);
        assert_eq!(info.latest_version, "0.3.1");
        assert_eq!(info.download_url.as_deref(), Some("https://example.com/archive"));
        assert_eq!(info.checksum_url.as_deref(), Some("https://example.com/sums"));
    }

    #[test]
    fn same_or_older_release_does_not_need_update() {
        assert!(!UpdateChecker::evaluate("0.3.1", &release("v0.3.1")).unwrap().needs_update);
        assert!(!UpdateChecker::evaluate("0.4.0", &release("v0.3.1")).unwrap().needs_update);
    }

    #[test]
    fn prerelease_is_older_than_release() {
        assert!(UpdateChecker::evaluate("0.3.1-rc.1", &release("0.3.1")).unwrap().needs_update);
    }

    #[test]
    fn missing_assets_are_reported_as_none() {
        let rel: GitHubRelease =
            serde_json::from_str(r#"{"tag_name":"v9.9.9","assets":[]}"#).unwrap();
        let info = UpdateChecker::evaluate("0.3.0", &rel).unwrap();
        assert!(info.needs_update);
        assert!(info.download_url.is_none());
        assert!(info.checksum_url.is_none());
    }

    #[test]
    fn invalid_versions_are_errors() {
        assert!(UpdateChecker::evaluate("not-a-version", &release("v1.0.0")).is_err());
        assert!(UpdateChecker::evaluate("0.3.0", &release("latest")).is_err());
    }

    #[test]
    fn platform_matches_release_naming() {
        let platform = UpdateChecker::detect_platform().unwrap();
        assert!(platform.contains('-'));
        let name = UpdateChecker::asset_name().unwrap();
        assert!(name.starts_with("cleanserve-"));
        assert!(name.ends_with(".tar.gz") || name.ends_with(".zip"));
    }
}
