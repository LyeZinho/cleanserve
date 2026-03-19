use cleanserve_shared::{CleanServeError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanServeConfig {
    pub name: String,
    pub engine: EngineConfig,
    #[serde(default)]
    pub server: ServerConfig,
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            port: default_port(),
            hot_reload: default_hot_reload(),
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
}
