use cleanserve_core::StackManager;

pub struct PackageCommand;

impl PackageCommand {
    pub async fn add(package_name: &str, _version: Option<&str>, project_root: &std::path::Path) -> Result<(), String> {
        let stack = StackManager::new(project_root).map_err(|e| e.to_string())?;
        stack.add(package_name).map_err(|e| e.to_string())?;
        println!("✓ Added stack service '{}'", package_name);
        println!("  Run: cleanserve stack up");
        Ok(())
    }

    pub fn list() -> Result<(), String> {
        let stack = StackManager::new(".").map_err(|e| e.to_string())?;
        println!("Available stack services (use `cleanserve stack add <name>`):\n");
        for (name, def) in stack.catalog() {
            println!("  {} — {}", name, def.title);
        }
        Ok(())
    }

    pub fn info(package_name: &str) -> Result<(), String> {
        let stack = StackManager::new(".").map_err(|e| e.to_string())?;
        let (_, def) = stack
            .catalog()
            .into_iter()
            .find(|(n, _)| n == package_name)
            .ok_or_else(|| format!("Service '{}' not found", package_name))?;
        println!("{} — {}", def.title, def.description);
        println!("Image: {}", def.image);
        if !def.requires.is_empty() {
            println!("Requires: {}", def.requires.join(", "));
        }
        Ok(())
    }

    pub fn start(_package_name: &str) -> Result<(), String> {
        Err("Use `cleanserve stack up` to start all enabled services".into())
    }

    pub fn stop(_package_name: &str) -> Result<(), String> {
        Err("Use `cleanserve stack down` to stop stack services".into())
    }

    pub fn status(_package_name: Option<&str>) -> Result<(), String> {
        Err("Use `cleanserve stack status`".into())
    }
}
