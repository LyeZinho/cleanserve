use anyhow::Context;
use cleanserve_core::{PhpDownloader, PhpManager};
use std::path::Path;

pub async fn run_php_update(version: Option<String>) -> anyhow::Result<()> {
    let version = version.unwrap_or_else(|| {
        Path::new("cleanserve.json")
            .exists()
            .then(|| {
                cleanserve_core::CleanServeConfig::load("cleanserve.json")
                    .ok()
                    .map(|c| c.engine.php)
            })
            .flatten()
            .unwrap_or_else(|| "8.4".to_string())
    });

    let manager = PhpManager::new().context("Failed to init PHP manager")?;
    if let Some(path) = manager.get_path(&version) {
        println!(
            "✅ PHP {} is already installed at {}",
            version,
            path.display()
        );
        return Ok(());
    }

    println!("📦 Downloading PHP {} to ~/.cleanserve/php/...", version);

    let downloader =
        PhpDownloader::global().context("Failed to initialize PHP downloader")?;

    match downloader.download(&version).await {
        Ok(path) => {
            println!("✅ PHP {} downloaded successfully!", version);
            println!("   Binary: {}", path.display());
            println!();
            println!("Run 'cleanserve up' to start the server.");
        }
        Err(e) => {
            anyhow::bail!("Failed to download PHP {}: {}", version, e);
        }
    }

    Ok(())
}

pub async fn run_cleanserve_update(check_only: bool, force: bool) -> anyhow::Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    println!("Checking for CleanServe updates...");

    let info = cleanserve_core::auto_updater::UpdateChecker::check_for_updates(current_version)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("Current version: {}", info.current_version);
    println!("Latest version: {}", info.latest_version);

    if !info.needs_update && !force {
        println!("✓ Already up to date!");
        return Ok(());
    }

    if check_only {
        if info.needs_update {
            println!(
                "✓ Update available: {} → {}",
                info.current_version, info.latest_version
            );
        }
        return Ok(());
    }

    println!("✓ Update would install: {}", info.latest_version);

    Ok(())
}
