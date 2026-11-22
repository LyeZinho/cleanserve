pub mod archive;
pub mod checker;
pub mod downloader;
pub mod installer;

pub use checker::{GitHubAsset, GitHubRelease, UpdateChecker};
pub use downloader::BinaryDownloader;
pub use installer::UpdateInstaller;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    /// Release archive for this platform (`None` if the release has no build for it).
    pub download_url: Option<String>,
    /// `SHA256SUMS` asset of the release (`None` if the release does not publish one).
    pub checksum_url: Option<String>,
    pub needs_update: bool,
}

#[derive(Debug)]
pub struct UpdateCheckerError {
    pub message: String,
}

impl std::fmt::Display for UpdateCheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Update error: {}", self.message)
    }
}

impl std::error::Error for UpdateCheckerError {}

pub type Result<T> = std::result::Result<T, UpdateCheckerError>;
