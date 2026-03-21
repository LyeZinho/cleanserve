use super::{PackageManagerError, Result};
use std::path::{Path, PathBuf};

/// Manages package state for a specific project
pub struct ProjectPackageManager {
    project_root: PathBuf,
    cleanserve_json_path: PathBuf,
}

impl ProjectPackageManager {
    /// Create project manager for given project root
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let cleanserve_json_path = project_root.join("cleanserve.json");

        if !cleanserve_json_path.exists() {
            return Err(PackageManagerError {
                message: format!(
                    "cleanserve.json not found at {}",
                    cleanserve_json_path.display()
                ),
            });
        }

        Ok(Self {
            project_root,
            cleanserve_json_path,
        })
    }

    /// Get package cache directory (~/.cleanserve/tools/)
    pub fn get_global_cache_dir() -> Result<PathBuf> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| PackageManagerError {
                message: "Cannot determine home directory".to_string(),
            })?
            .join(".cleanserve")
            .join("tools");

        Ok(cache_dir)
    }

    /// Get project .cleanserve/tools directory (for symlinks)
    pub fn get_project_tools_dir(&self) -> PathBuf {
        self.project_root.join(".cleanserve").join("tools")
    }

    /// Ensure project .cleanserve/tools directory exists
    pub fn ensure_tools_dir(&self) -> Result<()> {
        let tools_dir = self.get_project_tools_dir();
        std::fs::create_dir_all(&tools_dir).map_err(|e| PackageManagerError {
            message: format!(
                "Cannot create tools directory {}: {}",
                tools_dir.display(),
                e
            ),
        })?;
        Ok(())
    }

    /// Create symlink from project to global cache
    pub fn create_symlink(&self, package_name: &str, version: &str) -> Result<()> {
        self.ensure_tools_dir()?;

        let global_path = Self::get_global_cache_dir()?
            .join(package_name)
            .join(version);

        if !global_path.exists() {
            return Err(PackageManagerError {
                message: format!("Global package not found at {}", global_path.display()),
            });
        }

        let symlink_path = self.get_project_tools_dir().join(package_name);

        // Remove existing symlink if present
        if symlink_path.exists() || std::fs::symlink_metadata(&symlink_path).is_ok() {
            std::fs::remove_file(&symlink_path).map_err(|e| PackageManagerError {
                message: format!("Cannot remove existing symlink: {}", e),
            })?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&global_path, &symlink_path).map_err(|e| {
            PackageManagerError {
                message: format!("Cannot create symlink: {}", e),
            }
        })?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&global_path, &symlink_path).map_err(|e| {
            PackageManagerError {
                message: format!("Cannot create symlink: {}", e),
            }
        })?;

        Ok(())
    }

    /// Remove symlink for package
    pub fn remove_symlink(&self, package_name: &str) -> Result<()> {
        let symlink_path = self.get_project_tools_dir().join(package_name);

        if symlink_path.exists() || std::fs::symlink_metadata(&symlink_path).is_ok() {
            std::fs::remove_file(&symlink_path).map_err(|e| PackageManagerError {
                message: format!("Cannot remove symlink: {}", e),
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        assert!(!platform.is_empty());
    }
}
