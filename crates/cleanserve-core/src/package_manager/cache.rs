use super::{PackageManagerError, Result};
use std::path::PathBuf;

/// Manages global package cache at ~/.cleanserve/tools/
pub struct PackageCache;

impl PackageCache {
    /// Get cache root directory
    pub fn root() -> Result<PathBuf> {
        let cache = dirs::home_dir()
            .ok_or_else(|| PackageManagerError {
                message: "Cannot determine home directory".to_string(),
            })?
            .join(".cleanserve");

        Ok(cache)
    }

    /// Get package version cache path
    pub fn package_path(name: &str, version: &str) -> Result<PathBuf> {
        Self::root().map(|root| root.join("tools").join(name).join(version))
    }

    /// Ensure package version directory exists
    pub fn ensure_package_dir(name: &str, version: &str) -> Result<()> {
        let path = Self::package_path(name, version)?;
        std::fs::create_dir_all(&path).map_err(|e| PackageManagerError {
            message: format!("Cannot create cache directory {}: {}", path.display(), e),
        })?;
        Ok(())
    }

    /// Get temp download directory
    pub fn temp_dir() -> Result<PathBuf> {
        let temp = Self::root()?.join("tmp");
        std::fs::create_dir_all(&temp).map_err(|e| PackageManagerError {
            message: format!("Cannot create temp directory: {}", e),
        })?;
        Ok(temp)
    }

    /// Get logs directory
    pub fn logs_dir() -> Result<PathBuf> {
        let logs = Self::root()?.join("logs");
        std::fs::create_dir_all(&logs).map_err(|e| PackageManagerError {
            message: format!("Cannot create logs directory: {}", e),
        })?;
        Ok(logs)
    }

    /// Check if package version is already cached
    pub fn exists(name: &str, version: &str) -> Result<bool> {
        Ok(Self::package_path(name, version)?.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_root() {
        let root = PackageCache::root();
        assert!(root.is_ok());
        let path = root.unwrap();
        assert!(path.to_string_lossy().contains(".cleanserve"));
    }

    #[test]
    fn test_package_path() {
        let path = PackageCache::package_path("mysql", "8.0");
        assert!(path.is_ok());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("mysql"));
        assert!(p.to_string_lossy().contains("8.0"));
    }
}
