use super::{Package, PackageManagerError, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Registry of available packages from built-in and custom manifests
#[derive(Debug)]
pub struct PackageRegistry {
    packages: HashMap<String, Package>,
}

/// Manifest structure for JSON parsing
#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    packages: HashMap<String, Package>,
}

impl PackageRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    /// Load built-in manifest from embedded JSON
    pub fn with_builtin() -> Result<Self> {
        let mut registry = Self::new();
        registry.load_builtin()?;
        Ok(registry)
    }

    /// Load built-in package manifest
    fn load_builtin(&mut self) -> Result<()> {
        // TODO: Embed manifest as string constant
        // For now, return empty registry
        Ok(())
    }

    /// Load custom manifest from file path
    pub fn load_custom(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            // Custom manifest is optional
            return Ok(());
        }

        let content = std::fs::read_to_string(path).map_err(|e| PackageManagerError {
            message: format!("Failed to read custom manifest: {}", e),
        })?;

        let manifest: Manifest =
            serde_json::from_str(&content).map_err(|e| PackageManagerError {
                message: format!("Invalid custom manifest JSON: {}", e),
            })?;

        // Merge into registry
        for (name, package) in manifest.packages {
            self.packages.insert(name, package);
        }

        Ok(())
    }

    /// Get package by name
    pub fn get(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }

    /// List all available packages
    pub fn list(&self) -> Vec<&Package> {
        self.packages.values().collect()
    }

    /// Verify package exists and has version
    pub fn verify(&self, name: &str, version: &str) -> Result<()> {
        let package = self.get(name).ok_or_else(|| PackageManagerError {
            message: format!("Package '{}' not found in registry", name),
        })?;

        if !package.versions.contains_key(version) {
            return Err(PackageManagerError {
                message: format!("Version '{}' not found for package '{}'", version, name),
            });
        }

        Ok(())
    }
}

impl Default for PackageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = PackageRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_get_missing_package() {
        let registry = PackageRegistry::new();
        assert!(registry.get("mysql").is_none());
    }
}
