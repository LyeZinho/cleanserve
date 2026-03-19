use crate::traits::{FileSystem, VfsError, VfsMetadata};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct MemoryBackend {
    files: Arc<RwLock<HashMap<PathBuf, Arc<Vec<u8>>>>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, path: impl Into<PathBuf>, content: Vec<u8>) {
        self.files.write().insert(path.into(), Arc::new(content));
    }

    pub fn remove(&self, path: &Path) -> Option<Vec<u8>> {
        self.files.write().remove(path).map(|arc| (*arc).clone())
    }

    pub fn clear(&self) {
        self.files.write().clear();
    }

    pub fn len(&self) -> usize {
        self.files.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.files.read().keys().cloned().collect()
    }

    pub fn total_size(&self) -> usize {
        self.files
            .read()
            .values()
            .map(|arc: &Arc<Vec<u8>>| arc.len())
            .sum()
    }

    pub fn get(&self, path: &Path) -> Option<Vec<u8>> {
        self.files.read().get(path).map(|arc| arc.as_ref().to_vec())
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for MemoryBackend {
    fn read(&self, path: &Path) -> Result<Vec<u8>, VfsError> {
        self.files
            .read()
            .get(path)
            .map(|arc| arc.as_ref().to_vec())
            .ok_or_else(|| VfsError::NotFound(path.to_string_lossy().to_string()))
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.read().contains_key(path)
    }

    fn metadata(&self, path: &Path) -> Result<VfsMetadata, VfsError> {
        self.files
            .read()
            .get(path)
            .map(|arc: &Arc<Vec<u8>>| VfsMetadata {
                size: arc.len() as u64,
                is_dir: false,
                is_file: true,
                modified: None,
            })
            .ok_or_else(|| VfsError::NotFound(path.to_string_lossy().to_string()))
    }

    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, VfsError> {
        let files: Vec<PathBuf> = self
            .files
            .read()
            .keys()
            .filter(|p| {
                p.parent() == Some(path) || (path.as_os_str().is_empty() && p.parent().is_none())
            })
            .cloned()
            .collect();
        Ok(files)
    }
}

impl From<HashMap<PathBuf, Vec<u8>>> for MemoryBackend {
    fn from(files: HashMap<PathBuf, Vec<u8>>) -> Self {
        let backend = Self::new();
        for (path, content) in files {
            backend.insert(path, content);
        }
        backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_read() {
        let backend = MemoryBackend::new();
        backend.insert("test.php", b"<?php echo 'test';".to_vec());

        let content = backend.read(Path::new("test.php")).unwrap();
        assert_eq!(content, b"<?php echo 'test';");
    }

    #[test]
    fn test_not_found() {
        let backend = MemoryBackend::new();
        let result = backend.read(Path::new("nonexistent.php"));
        assert!(result.is_err());
    }

    #[test]
    fn test_exists() {
        let backend = MemoryBackend::new();
        backend.insert("exists.php", b"<?php".to_vec());

        assert!(backend.exists(Path::new("exists.php")));
        assert!(!backend.exists(Path::new("missing.php")));
    }

    #[test]
    fn test_total_size() {
        let backend = MemoryBackend::new();
        backend.insert("a.php", b"short".to_vec());
        backend.insert("b.php", b"a bit longer content".to_vec());

        assert_eq!(backend.total_size(), 25);
    }
}
