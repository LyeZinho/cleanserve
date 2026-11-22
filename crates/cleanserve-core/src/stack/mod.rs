mod manifest;
mod compose;
mod docker_cli;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Io(String),
    #[error("Docker is required but not available. Run: cleanserve setup docker (or install-docker.sh)")]
    NotAvailable,
    #[error("Docker command failed: {0}")]
    Command(String),
}

pub use manifest::{StackManifest, StackServiceDef};
pub use docker_cli::{ensure_docker, docker_available};

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const STATE_FILE: &str = "services.json";
const COMPOSE_FILE: &str = "docker-compose.yml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StackState {
    #[serde(default)]
    pub enabled: Vec<String>,
}

pub struct StackManager {
    project_root: PathBuf,
    project_slug: String,
    stack_dir: PathBuf,
    manifest: StackManifest,
}

impl StackManager {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self, DockerError> {
        let project_root = project_root.as_ref().to_path_buf();
        let manifest = StackManifest::load_builtin()?;
        let project_slug = slugify(
            project_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project"),
        );
        let stack_dir = project_root.join(".cleanserve/stack");
        Ok(Self {
            project_root,
            project_slug,
            stack_dir,
            manifest,
        })
    }

    pub fn catalog(&self) -> Vec<(String, &StackServiceDef)> {
        self.manifest
            .list_names()
            .into_iter()
            .filter_map(|n| self.manifest.get(&n).map(|d| (n, d)))
            .collect()
    }

    pub fn load_state(&self) -> Result<StackState, DockerError> {
        let path = self.stack_dir.join(STATE_FILE);
        if !path.exists() {
            return Ok(StackState::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| DockerError::Io(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| DockerError::Config(e.to_string()))
    }

    fn save_state(&self, state: &StackState) -> Result<(), DockerError> {
        std::fs::create_dir_all(&self.stack_dir).map_err(|e| DockerError::Io(e.to_string()))?;
        let content =
            serde_json::to_string_pretty(state).map_err(|e| DockerError::Config(e.to_string()))?;
        std::fs::write(self.stack_dir.join(STATE_FILE), content)
            .map_err(|e| DockerError::Io(e.to_string()))
    }

    pub fn add(&self, name: &str) -> Result<(), DockerError> {
        let def = self
            .manifest
            .get(name)
            .ok_or_else(|| DockerError::Config(format!("Unknown stack service '{}'", name)))?;

        let mut state = self.load_state()?;
        if state.enabled.iter().any(|s| s == name) {
            return Err(DockerError::Config(format!(
                "Service '{}' is already enabled",
                name
            )));
        }

        for req in &def.requires {
            if !state.enabled.iter().any(|s| s == req) {
                return Err(DockerError::Config(format!(
                    "Service '{}' requires '{}' — add it first: cleanserve stack add {}",
                    name, req, req
                )));
            }
        }

        state.enabled.push(name.to_string());
        self.regenerate(&state)?;
        self.save_state(&state)?;
        Ok(())
    }

    pub fn remove(&self, name: &str) -> Result<(), DockerError> {
        let mut state = self.load_state()?;
        let before = state.enabled.len();
        state.enabled.retain(|s| s != name);
        if state.enabled.len() == before {
            return Err(DockerError::Config(format!("Service '{}' is not enabled", name)));
        }

        for enabled in &state.enabled {
            if let Some(def) = self.manifest.get(enabled) {
                if def.requires.iter().any(|r| r == name) {
                    return Err(DockerError::Config(format!(
                        "Cannot remove '{}': service '{}' depends on it",
                        name, enabled
                    )));
                }
            }
        }

        self.regenerate(&state)?;
        self.save_state(&state)?;
        Ok(())
    }

    fn regenerate(&self, state: &StackState) -> Result<(), DockerError> {
        std::fs::create_dir_all(&self.stack_dir).map_err(|e| DockerError::Io(e.to_string()))?;
        let compose_path = self.stack_dir.join(COMPOSE_FILE);
        if state.enabled.is_empty() {
            if compose_path.exists() {
                let _ = std::fs::remove_file(&compose_path);
            }
            return Ok(());
        }

        let mut ordered: Vec<(&str, &StackServiceDef)> = Vec::new();
        for name in &state.enabled {
            if let Some(def) = self.manifest.get(name) {
                ordered.push((name.as_str(), def));
            }
        }

        let yaml = compose::render(&self.project_slug, &ordered);
        std::fs::write(&compose_path, yaml).map_err(|e| DockerError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn compose_path(&self) -> PathBuf {
        self.stack_dir.join(COMPOSE_FILE)
    }

    pub fn start(&self) -> Result<(), DockerError> {
        ensure_docker()?;
        let state = self.load_state()?;
        if state.enabled.is_empty() {
            return Err(DockerError::Config(
                "No stack services enabled. Run: cleanserve stack add mysql".into(),
            ));
        }
        if !self.compose_path().exists() {
            self.regenerate(&state)?;
        }
        docker_cli::compose_up(&self.compose_path(), &self.project_root)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), DockerError> {
        ensure_docker()?;
        if !self.compose_path().exists() {
            return Ok(());
        }
        docker_cli::compose_down(&self.compose_path(), &self.project_root, false)?;
        Ok(())
    }

    pub fn status(&self) -> Result<(), DockerError> {
        ensure_docker()?;
        let state = self.load_state()?;
        if state.enabled.is_empty() {
            println!("No stack services enabled for this project.");
            return Ok(());
        }
        println!("Enabled stack services:");
        for name in &state.enabled {
            if let Some(def) = self.manifest.get(name) {
                println!("  - {} ({})", name, def.title);
            }
        }
        if self.compose_path().exists() {
            docker_cli::compose_ps(&self.compose_path(), &self.project_root)?;
        }
        Ok(())
    }

    /// Stop containers and remove volumes/images for this project's stack.
    pub fn teardown(&self, prune_volumes: bool) -> Result<(), DockerError> {
        if !self.compose_path().exists() {
            return Ok(());
        }
        if docker_available() {
            docker_cli::compose_down(&self.compose_path(), &self.project_root, prune_volumes)?;
        }
        Ok(())
    }

    pub fn purge_local(&self) -> Result<(), DockerError> {
        if self.stack_dir.exists() {
            std::fs::remove_dir_all(&self.stack_dir).map_err(|e| DockerError::Io(e.to_string()))?;
        }
        Ok(())
    }
}

pub fn teardown_all_for_project(project_root: &Path, prune_volumes: bool) -> Result<(), DockerError> {
    let stack = StackManager::new(project_root)?;
    stack.teardown(prune_volumes)?;
    stack.purge_local()?;
    Ok(())
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
