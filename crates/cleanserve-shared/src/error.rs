use thiserror::Error;

#[derive(Error, Debug)]
pub enum CleanServeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("PHP not found: {0}")]
    PhpNotFound(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Watch error: {0}")]
    Watch(String),
}

pub type Result<T> = std::result::Result<T, CleanServeError>;
