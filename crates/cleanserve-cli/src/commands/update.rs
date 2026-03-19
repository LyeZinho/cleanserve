use anyhow::Context;
use cleanserve_core::PhpDownloader;

pub async fn run(version: Option<String>) -> anyhow::Result<()> {
    let version = version.unwrap_or_else(|| "8.4.0".to_string());
    
    let downloader = PhpDownloader::new()
        .context("Failed to initialize PHP downloader")?;
    
    // Check if already installed
    if downloader.is_installed(&version) {
        println!("✅ PHP {} is already installed", version);
        return Ok(());
    }
    
    println!("📦 Installing PHP {}...", version);
    
    match downloader.download(&version).await {
        Ok(_) => {
            println!("✅ PHP {} installed successfully!", version);
            println!();
            println!("You can now run 'cleanserve up' to start the server.");
        }
        Err(e) => {
            anyhow::bail!("Failed to install PHP {}: {}", version, e);
        }
    }
    
    Ok(())
}
