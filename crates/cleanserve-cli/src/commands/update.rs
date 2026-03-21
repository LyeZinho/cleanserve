use anyhow::Context;
use cleanserve_core::PhpDownloader;
use cleanserve_core::auto_updater::UpdateChecker;
use std::path::Path;

pub async fn run_php_update(version: Option<String>) -> anyhow::Result<()> {
    let version = version.unwrap_or_else(|| "8.4".to_string());

    // Use project-local .cleanserve/php/ directory
    let php_dir = Path::new(".cleanserve").join("php");
    let downloader = PhpDownloader::new(&php_dir)
        .context("Failed to initialize PHP downloader")?;

    // Check if already installed
    if downloader.is_installed(&version) {
        println!("✅ PHP {} is already installed in .cleanserve/php/", version);
        return Ok(());
    }

    println!("📦 Downloading PHP {} to .cleanserve/php/...", version);

    match downloader.download(&version).await {
        Ok(_) => {
            println!("✅ PHP {} downloaded successfully!", version);
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
    
    let info = UpdateChecker::check_for_updates(current_version).await
        .map_err(|e| anyhow::anyhow!(e))?;
    
    println!("Current version: {}", info.current_version);
    println!("Latest version: {}", info.latest_version);
    
    if !info.needs_update && !force {
        println!("✓ Already up to date!");
        return Ok(());
    }
    
    if check_only {
        if info.needs_update {
            println!("✓ Update available: {} → {}", info.current_version, info.latest_version);
        }
        return Ok(());
    }
    
    // Placeholder: In Phase 4b, implement actual update
    println!("✓ Update would install: {}", info.latest_version);
    
    Ok(())
}

pub async fn run(version: Option<String>) -> anyhow::Result<()> {
    run_php_update(version).await
}
