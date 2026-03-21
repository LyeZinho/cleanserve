use cleanserve_core::package_manager::PackageRegistry;
use std::path::Path;

pub struct PackageCommand;

impl PackageCommand {
    pub async fn add(package_name: &str, version: Option<&str>, _project_root: &Path) -> Result<(), String> {
        let registry = PackageRegistry::with_builtin()
            .map_err(|e| format!("Failed to load package registry: {}", e))?;

        let pkg = registry.get(package_name)
            .ok_or_else(|| format!("Package '{}' not found", package_name))?;

        let version = match version {
            Some(v) => v.to_string(),
            None => {
                pkg.versions.keys().next()
                    .ok_or("Package has no versions")?
                    .clone()
            }
        };

        registry.verify(package_name, &version)
            .map_err(|e| e.to_string())?;

        println!("✓ Package '{}' version '{}' found", package_name, version);

        Ok(())
    }

    pub fn list() -> Result<(), String> {
        let registry = PackageRegistry::with_builtin()
            .map_err(|e| format!("Failed to load registry: {}", e))?;

        println!("Available packages:");
        for pkg in registry.list() {
            println!("  - {} ({})", pkg.name, pkg.description);
        }

        Ok(())
    }

    pub fn info(package_name: &str) -> Result<(), String> {
        let registry = PackageRegistry::with_builtin()
            .map_err(|e| format!("Failed to load registry: {}", e))?;

        let pkg = registry.get(package_name)
            .ok_or_else(|| format!("Package '{}' not found", package_name))?;

        println!("Package: {}", pkg.name);
        println!("Description: {}", pkg.description);
        if let Some(url) = &pkg.homepage {
            println!("Homepage: {}", url);
        }
        println!("Available versions:");
        for version in pkg.versions.keys() {
            println!("  - {}", version);
        }

        Ok(())
    }

    pub fn start(package_name: &str) -> Result<(), String> {
        println!("✓ Starting package '{}' (placeholder - full implementation in Phase 4)", package_name);
        Ok(())
    }

    pub fn stop(package_name: &str) -> Result<(), String> {
        println!("✓ Stopped package '{}' (placeholder - full implementation in Phase 4)", package_name);
        Ok(())
    }

     pub fn status(package_name: Option<&str>) -> Result<(), String> {
        if let Some(name) = package_name {
            println!("Status of package '{}': Running (placeholder)", name);
        } else {
            println!("All packages status (placeholder):");
        }
        Ok(())
    }
}
