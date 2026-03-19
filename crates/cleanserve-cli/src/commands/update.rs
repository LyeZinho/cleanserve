use anyhow::Context;
use cleanserve_core::PhpDownloader;
use std::path::Path;

pub async fn run(version: Option<String>) -> anyhow::Result<()> {
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
