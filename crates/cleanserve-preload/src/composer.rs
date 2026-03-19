use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComposerError {
    #[error("Failed to read composer.json: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse composer.json: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("No autoload section found in composer.json")]
    NoAutoload,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposerJson {
    #[serde(rename = "autoload")]
    pub autoload: Option<AutoloadConfig>,
    #[serde(rename = "autoload-dev")]
    pub autoload_dev: Option<AutoloadConfig>,
    #[serde(rename = "extra")]
    pub extra: Option<ComposerExtra>,
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "description")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AutoloadConfig {
    #[serde(rename = "psr-4", default)]
    pub psr4: Option<HashMap<String, String>>,
    #[serde(rename = "psr-0", default)]
    pub psr0: Option<HashMap<String, Vec<String>>>,
    #[serde(rename = "classmap", default)]
    pub classmap: Option<Vec<String>>,
    #[serde(rename = "files", default)]
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComposerExtra {
    #[serde(rename = "preload", default)]
    pub preload: Option<PreloadExtra>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PreloadExtra {
    #[serde(rename = "paths", default)]
    pub paths: Option<Vec<String>>,
    #[serde(rename = "exclude", default)]
    pub exclude: Option<Vec<String>>,
    #[serde(rename = "extensions", default)]
    pub extensions: Option<Vec<String>>,
}

impl ComposerJson {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ComposerError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let json: ComposerJson = serde_json::from_str(&content)?;
        Ok(json)
    }

    pub fn from_project(project_root: impl AsRef<Path>) -> Result<Self, ComposerError> {
        let path = project_root.as_ref().join("composer.json");
        Self::load(&path)
    }

    pub fn get_psr4_namespaces(&self) -> HashMap<String, String> {
        let mut namespaces = HashMap::new();

        if let Some(ref autoload) = self.autoload {
            if let Some(ref psr4) = autoload.psr4 {
                for (prefix, path) in psr4 {
                    namespaces.insert(prefix.clone(), path.clone());
                }
            }
        }

        if let Some(ref autoload_dev) = self.autoload_dev {
            if let Some(ref psr4) = autoload_dev.psr4 {
                for (prefix, path) in psr4 {
                    if !namespaces.contains_key(prefix) {
                        namespaces.insert(prefix.clone(), path.clone());
                    }
                }
            }
        }

        namespaces
    }

    pub fn get_extra_preload_paths(&self) -> Vec<String> {
        self.extra
            .as_ref()
            .and_then(|e| e.preload.as_ref())
            .and_then(|p| p.paths.clone())
            .unwrap_or_default()
    }

    pub fn get_extra_exclude_patterns(&self) -> Vec<String> {
        self.extra
            .as_ref()
            .and_then(|e| e.preload.as_ref())
            .and_then(|p| p.exclude.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_composer_json(content: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("composer.json"), content).unwrap();
        dir
    }

    #[test]
    fn test_parse_basic_composer_json() {
        let dir = create_test_composer_json(
            r#"{
            "autoload": {
                "psr-4": {
                    "App\\": "src/"
                }
            }
        }"#,
        );

        let composer = ComposerJson::load(dir.path().join("composer.json")).unwrap();
        let namespaces = composer.get_psr4_namespaces();

        assert_eq!(namespaces.get("App\\"), Some(&"src/".to_string()));
    }

    #[test]
    fn test_parse_multiple_namespaces() {
        let dir = create_test_composer_json(
            r#"{
            "autoload": {
                "psr-4": {
                    "App\\": "src/",
                    "Database\\": "database/"
                }
            }
        }"#,
        );

        let composer = ComposerJson::load(dir.path().join("composer.json")).unwrap();
        let namespaces = composer.get_psr4_namespaces();

        assert_eq!(namespaces.len(), 2);
        assert_eq!(namespaces.get("App\\"), Some(&"src/".to_string()));
        assert_eq!(namespaces.get("Database\\"), Some(&"database/".to_string()));
    }

    #[test]
    fn test_extra_preload_paths() {
        let dir = create_test_composer_json(
            r#"{
            "autoload": {
                "psr-4": {
                    "App\\": "src/"
                }
            },
            "extra": {
                "preload": {
                    "paths": ["src/Cache", "src/Events"],
                    "exclude": ["tests/"]
                }
            }
        }"#,
        );

        let composer = ComposerJson::load(dir.path().join("composer.json")).unwrap();

        assert_eq!(
            composer.get_extra_preload_paths(),
            vec!["src/Cache", "src/Events"]
        );
        assert_eq!(composer.get_extra_exclude_patterns(), vec!["tests/"]);
    }
}
