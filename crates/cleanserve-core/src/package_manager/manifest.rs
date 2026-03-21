use super::{Package, PackageManagerError, Result};
use serde::Deserialize;
use std::collections::HashMap;

/// Built-in package manifest (embedded in binary)
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub packages: HashMap<String, Package>,
}

impl Manifest {
    /// Load built-in manifest from embedded resource
    pub fn load_builtin() -> Result<Self> {
        const MANIFEST_JSON: &str = include_str!("../../../../resources/packages-manifest.json");

        serde_json::from_str(MANIFEST_JSON).map_err(|e| PackageManagerError {
            message: format!("Failed to parse built-in manifest: {}", e),
        })
    }

    /// Validate manifest integrity
    pub fn validate(&self) -> Result<()> {
        if self.version.is_empty() {
            return Err(PackageManagerError {
                message: "Manifest version is empty".to_string(),
            });
        }

        if self.packages.is_empty() {
            return Err(PackageManagerError {
                message: "Manifest has no packages".to_string(),
            });
        }

        for (name, package) in &self.packages {
            if package.versions.is_empty() {
                return Err(PackageManagerError {
                    message: format!("Package '{}' has no versions", name),
                });
            }

            for (version, pkg_version) in &package.versions {
                if pkg_version.downloads.is_empty() {
                    return Err(PackageManagerError {
                        message: format!("Package '{}/{}' has no downloads", name, version),
                    });
                }

                for (platform, download_info) in &pkg_version.downloads {
                    if download_info.url.is_empty() {
                        return Err(PackageManagerError {
                            message: format!(
                                "Package '{}/{}/{}' has empty URL",
                                name, version, platform
                            ),
                        });
                    }

                    if !download_info.checksum.starts_with("sha256:") {
                        return Err(PackageManagerError {
                            message: format!(
                                "Package '{}/{}/{}'has invalid checksum format",
                                name, version, platform
                            ),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_builtin_manifest() {
        let manifest = Manifest::load_builtin();
        assert!(manifest.is_ok());
    }

    #[test]
    fn test_validate_builtin_manifest() {
        let manifest = Manifest::load_builtin().unwrap();
        let result = manifest.validate();
        assert!(result.is_ok());
    }
}
