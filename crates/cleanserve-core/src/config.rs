use cleanserve_shared::{CleanServeError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanServeConfig {
    pub name: String,
    pub engine: EngineConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub packages: Option<PackagesConfig>,
    /// Applications served by this project. Empty means auto-detect.
    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
    /// Docker-backed services managed by `cleanserve stack`.
    #[serde(default)]
    pub stack: HashMap<String, StackService>,
}

/// One served application inside a project (Yii2 frontend/backend, Laravel, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Document root, relative to the project root.
    pub root: String,
    /// Public hostname. Defaults to `<name>.<server.domain_suffix>` for
    /// multi-app projects and `localhost` for single-app projects.
    #[serde(default)]
    pub domain: Option<String>,
    /// Framework adapter name (yii2, laravel, symfony, ...).
    #[serde(default)]
    pub framework: Option<String>,
    /// Optional front-controller/router script, relative to `root`.
    #[serde(default)]
    pub router: Option<String>,
}

/// A Docker service declared in `cleanserve.json` under `stack`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackService {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub port: Option<u16>,
    /// Extra environment variables passed to the container.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

impl Default for StackService {
    fn default() -> Self {
        Self {
            version: None,
            enabled: true,
            port: None,
            env: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub php: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default = "default_display_errors")]
    pub display_errors: bool,
    #[serde(default)]
    pub memory_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
    /// Suffix used for generated app domains (e.g. `frontend.<suffix>`).
    #[serde(default = "default_domain_suffix")]
    pub domain_suffix: String,
}

fn default_display_errors() -> bool {
    true
}

fn default_root() -> String {
    "public/".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_hot_reload() -> bool {
    true
}

fn default_domain_suffix() -> String {
    "serve".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesConfig {
    #[serde(flatten)]
    pub packages: HashMap<String, PackageSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageSpec {
    Simple(String),
    Detailed {
        version: String,
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        path: Option<String>,
    },
}

impl PackagesConfig {
    pub fn validate(&self) -> Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }
        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            port: default_port(),
            hot_reload: default_hot_reload(),
            domain_suffix: default_domain_suffix(),
        }
    }
}

impl CleanServeConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| CleanServeError::Config(format!("Failed to read: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| CleanServeError::Config(format!("Parse error: {}", e)))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CleanServeError::Config(format!("Serialize error: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| CleanServeError::Config(format!("Write error: {}", e)))
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(ref packages) = self.packages {
            packages.validate()?;
        }
        Ok(())
    }
}
