use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VfsError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path error: {0}")]
    Path(String),
}

#[derive(Debug, Clone)]
pub struct VfsMetadata {
    pub size: u64,
    pub is_dir: bool,
    pub is_file: bool,
    pub modified: Option<std::time::SystemTime>,
}

pub trait FileSystem: Send + Sync {
    fn read(&self, path: &Path) -> Result<Vec<u8>, VfsError>;
    fn exists(&self, path: &Path) -> bool;
    fn metadata(&self, path: &Path) -> Result<VfsMetadata, VfsError>;
    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, VfsError>;
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                components.clear();
                components.push(component);
            }
            std::path::Component::Normal(s) => {
                components.push(std::path::Component::Normal(s));
            }
        }
    }

    components.iter().collect()
}

pub fn resolve_symlink(
    path: &Path,
    fs: &dyn FileSystem,
    max_depth: usize,
) -> Result<PathBuf, VfsError> {
    let current = normalize_path(path);
    let mut visited = Vec::new();
    let depth = 0;

    loop {
        if depth > max_depth {
            return Err(VfsError::Path(format!(
                "Max symlink depth exceeded: {}",
                path.display()
            )));
        }

        let meta = fs.metadata(&current)?;

        if !meta.is_file {
            return Ok(current);
        }

        if visited.contains(&current) {
            return Err(VfsError::Path(format!(
                "Circular symlink detected: {}",
                path.display()
            )));
        }
        visited.push(current.clone());

        break;
    }

    Ok(current)
}
