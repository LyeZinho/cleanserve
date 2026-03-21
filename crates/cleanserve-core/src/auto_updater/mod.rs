pub mod checker;
pub mod downloader;

pub use checker::UpdateChecker;
pub use downloader::BinaryDownloader;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub checksum_url: String,
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
