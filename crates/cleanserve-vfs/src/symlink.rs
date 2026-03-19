use crate::traits::VfsError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct SymlinkCache {
    cache: HashMap<PathBuf, PathBuf>,
    targets: HashMap<PathBuf, Vec<PathBuf>>,
}

impl SymlinkCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    pub fn add_link(&mut self, link: PathBuf, target: PathBuf) {
        self.cache.insert(link.clone(), target.clone());
        self.targets.entry(target).or_default().push(link);
    }

    pub fn resolve(&self, path: &Path) -> Option<&PathBuf> {
        self.cache.get(path)
    }

    pub fn is_symlink(&self, path: &Path) -> bool {
        self.cache.contains_key(path)
    }

    pub fn resolve_deep(&self, path: &Path, max_depth: usize) -> Result<PathBuf, VfsError> {
        let mut current = path.to_path_buf();
        let mut visited = Vec::new();
        let mut depth = 0;

        while depth < max_depth {
            if visited.contains(&current) {
                return Err(VfsError::Path(format!(
                    "Circular symlink detected: {}",
                    path.display()
                )));
            }

            match self.cache.get(&current) {
                Some(target) => {
                    visited.push(current);
                    current = target.clone();
                }
                None => break,
            }
            depth += 1;
        }

        if depth >= max_depth {
            return Err(VfsError::Path(format!(
                "Max symlink depth exceeded: {}",
                path.display()
            )));
        }

        Ok(current)
    }
}

impl Default for SymlinkCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_resolve() {
        let mut cache = SymlinkCache::new();
        cache.add_link(PathBuf::from("current"), PathBuf::from("target"));

        assert_eq!(
            cache.resolve(Path::new("current")),
            Some(&PathBuf::from("target"))
        );
        assert!(!cache.is_symlink(Path::new("nonexistent")));
    }

    #[test]
    fn test_deep_resolve() {
        let mut cache = SymlinkCache::new();
        cache.add_link(PathBuf::from("a"), PathBuf::from("b"));
        cache.add_link(PathBuf::from("b"), PathBuf::from("c"));

        let resolved = cache.resolve_deep(Path::new("a"), 10).unwrap();
        assert_eq!(resolved, PathBuf::from("c"));
    }

    #[test]
    fn test_circular_detection() {
        let mut cache = SymlinkCache::new();
        cache.add_link(PathBuf::from("a"), PathBuf::from("b"));
        cache.add_link(PathBuf::from("b"), PathBuf::from("a"));

        let result = cache.resolve_deep(Path::new("a"), 10);
        assert!(result.is_err());
    }
}
