use anyhow::Context;
use std::process::Command;

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    // Load config to get PHP version
    let config_path = std::path::Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }
    
    let config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load config")?;
    
    let php_version = &config.engine.php;
    
    // Get PHP path
    let manager = cleanserve_core::PhpManager::new()
        .context("Failed to init PHP manager")?;
    
    let php_path = manager.get_path(php_version)
        .context(format!("PHP {} not found. Run 'cleanserve update --version {}' first.", php_version, php_version))?;
    
    // Find composer
    let composer_path = find_composer()
        .context("Composer not found. Please install Composer first.")?;
    
    println!("🎼 Running Composer with PHP {}", php_version);
    println!("📦 Command: composer {}\n", args.join(" "));
    
    // Run composer with project's PHP
    let status = Command::new(&php_path)
        .arg(&composer_path)
        .args(&args)
        .status()
        .context("Failed to execute composer")?;
    
    if !status.success() {
        anyhow::bail!("Composer exited with code: {:?}", status.code());
    }
    
    Ok(())
}

fn find_composer() -> Option<std::path::PathBuf> {
    // Try common locations
    let possible_paths: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("composer.phar"),
        dirs::home_dir()?.join(".composer").join("vendor").join("bin").join("composer"),
        dirs::home_dir()?.join(".composer").join("composer.phar"),
    ];
    
    for path in possible_paths {
        if path.exists() {
            return Some(path);
        }
    }
    
    // Try to find composer in PATH
    if let Ok(output) = std::process::Command::new("which").arg("composer").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            let path = std::path::PathBuf::from(path.trim());
            if path.exists() {
                return Some(path);
            }
        }
    }
    
    None
}
