//! Package Manager: Download, cache, and manage standalone tools
//!
//! The package manager enables projects to declare and use standalone tools
//! (MySQL, Redis, phpMyAdmin, etc) via `cleanserve package` commands.
//!
//! Architecture:
//! - Registry: Load built-in + custom package definitions
//! - Downloader: Fetch + verify packages from remote sources
//! - Project: Manage per-project package state in cleanserve.json

pub mod cache;
pub mod downloader;
pub mod lifecycle;
pub mod manifest;
pub mod project;
pub mod registry;
pub mod runtime;

pub use cache::PackageCache;
pub use downloader::PackageDownloader;
pub use lifecycle::PackageLifecycle;
pub use project::ProjectPackageManager;
pub use registry::PackageRegistry;
pub use runtime::{PackageRuntime, RuntimeStatus};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Package metadata from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub description: String,
    pub homepage: Option<String>,
    pub versions: std::collections::HashMap<String, PackageVersion>,
}

/// Package version definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub downloads: std::collections::HashMap<String, DownloadInfo>,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub default_port: Option<u16>,
    #[serde(default)]
    pub health_check: Option<String>,
    #[serde(default)]
    pub proxy_path: Option<String>,
    #[serde(default)]
    pub server_type: Option<String>,
}

/// Download details for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    pub checksum: String, // sha256:abc123...
    #[serde(default)]
    pub format: Option<String>, // tar.xz, tar.gz, zip
}

/// Error type for package manager operations
#[derive(Debug)]
pub struct PackageManagerError {
    pub message: String,
}

impl std::fmt::Display for PackageManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PackageManager error: {}", self.message)
    }
}

impl std::error::Error for PackageManagerError {}

/// Result type for package manager operations
pub type Result<T> = std::result::Result<T, PackageManagerError>;
