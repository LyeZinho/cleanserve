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

/// Resolve `path` to the normalized location of an existing entry.
///
/// The [`FileSystem`] trait does not expose symlink targets, so there is nothing to follow
/// here: symlinks are resolved transparently by the backing filesystem when it reads
/// metadata. This only normalizes `path` and guarantees the entry exists, returning
/// `NotFound`/`PermissionDenied` (via `metadata`) otherwise. `_max_depth` is kept for API
/// compatibility with callers that bound symlink chains.
pub fn resolve_symlink(
    path: &Path,
    fs: &dyn FileSystem,
    _max_depth: usize,
) -> Result<PathBuf, VfsError> {
    let current = normalize_path(path);
    fs.metadata(&current)?;
    Ok(current)
}
