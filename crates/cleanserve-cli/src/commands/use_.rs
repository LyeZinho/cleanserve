use anyhow::Context;
use cleanserve_core::{PhpDownloader, PhpManager};
use std::path::Path;

pub async fn run(version: String) -> anyhow::Result<()> {
    let manager = PhpManager::new().context("Failed to init PHP manager")?;

    if !manager.is_installed(&version) {
        println!("PHP {} is not installed. Downloading...", version);
        let downloader =
            PhpDownloader::global().context("Failed to initialize PHP downloader")?;
        downloader
            .download(&version)
            .await
            .with_context(|| format!("Failed to download PHP {}", version))?;
    }

    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let mut config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load cleanserve.json")?;

    config.engine.php = version.clone();
    config
        .save(config_path)
        .context("Failed to save cleanserve.json")?;

    let resolved = PhpManager::new()
        .ok()
        .and_then(|m| m.resolve_installed_version(&version))
        .unwrap_or(version.clone());

    if resolved != version {
        println!("✓ Switched to PHP {} ({})", version, resolved);
    } else {
        println!("✓ Switched to PHP {}", version);
    }

    Ok(())
}
