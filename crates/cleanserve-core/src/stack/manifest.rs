use super::DockerError;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct StackManifest {
    pub version: String,
    pub services: HashMap<String, StackServiceDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StackServiceDef {
    pub title: String,
    pub description: String,
    pub image: String,
    #[serde(default)]
    pub ports: Vec<PortMapping>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PortMapping {
    pub host: u16,
    pub container: u16,
}

impl StackManifest {
    pub fn load_builtin() -> Result<Self, DockerError> {
        let json = include_str!("../../../../resources/stack-services.json");
        serde_json::from_str(json).map_err(|e| DockerError::Config(format!("Invalid stack manifest: {}", e)))
    }

    pub fn get(&self, name: &str) -> Option<&StackServiceDef> {
        self.services.get(name)
    }

    pub fn list(&self) -> Vec<&StackServiceDef> {
        let mut names: Vec<_> = self.services.keys().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|n| self.services.get(n))
            .collect()
    }

    pub fn list_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.services.keys().cloned().collect();
        names.sort();
        names
    }
}
