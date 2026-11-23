use anyhow::Context;
use cleanserve_core::{is_composer_capable, PhpDownloader, PhpManager};
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
        if is_composer_capable(&path) {
            println!(
                "✅ PHP {} is already installed at {}",
                version,
                path.display()
            );
            return Ok(());
        }
        println!(
            "⚠️  PHP {} at {} is missing Composer extensions — upgrading runtime…",
            version,
            path.display()
        );
        manager
            .remove_matching_installs(&version)
            .context("Failed to remove outdated PHP install")?;
    }

    println!("📦 Installing PHP {} to ~/.cleanserve/php/...", version);

    let downloader =
        PhpDownloader::global().context("Failed to initialize PHP downloader")?;

    match downloader.ensure_composer_capable(&version).await {
        Ok(path) => {
            println!("✅ PHP {} ready for CleanServe and Composer", version);
            println!("   Binary: {}", path.display());
            println!();
            println!("Run 'cleanserve up' to start the server.");
        }
        Err(e) => {
            anyhow::bail!("Failed to install PHP {}: {}", version, e);
        }
    }

    Ok(())
}
