use anyhow::Context;

pub async fn run(version: String) -> anyhow::Result<()> {
    let manager = cleanserve_core::PhpManager::new()
        .context("Failed to initialize PHP manager")?;
    
    if !manager.is_installed(&version) {
        anyhow::bail!(
            "PHP {} is not installed. Run 'cleanserve update --version {}' to download it.",
            version, version
        );
    }
    
    let config_path = std::path::Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }
    
    let mut config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load cleanserve.json")?;
    
    config.engine.php = version.clone();
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;
    
    println!("✓ Switched to PHP {}", version);
    
    Ok(())
}
