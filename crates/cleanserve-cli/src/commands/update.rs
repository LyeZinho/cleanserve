use anyhow::Context;
use cleanserve_core::auto_updater::{archive, BinaryDownloader, UpdateChecker, UpdateInfo, UpdateInstaller};
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

    let info = UpdateChecker::check_for_updates(current_version)
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
            println!("✓ Update available: {} → {}", info.current_version, info.latest_version);
            println!("  Run `cleanserve update` to install it.");
        }
        return Ok(());
    }

    install_info(&info).await
}

/// Download, verify and install the release described by `info` (upgrade or downgrade),
/// rolling back if the new binary fails its sanity check.
pub async fn install_info(info: &UpdateInfo) -> anyhow::Result<()> {
    let platform_asset = UpdateChecker::asset_name().map_err(|e| anyhow::anyhow!(e))?;
    let download_url = info.download_url.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Release v{} has no build for this platform ({}).",
            info.latest_version,
            platform_asset
        )
    })?;
    // Never install a binary that cannot be verified.
    let checksum_url = info.checksum_url.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Release v{} does not publish SHA256SUMS; refusing to install an unverified binary.",
            info.latest_version
        )
    })?;

    let current_exe = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .context("Cannot locate the running cleanserve binary")?;

    let work_dir = tempfile::tempdir().context("Cannot create temporary directory")?;
    let archive_path = work_dir.path().join(&platform_asset);
    let new_binary = work_dir.path().join(archive::binary_name());

    println!("⬇  Downloading {}...", platform_asset);
    BinaryDownloader::download_release(&download_url, &checksum_url, &archive_path)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    println!("✓ Checksum verified");

    archive::extract_binary(&archive_path, &new_binary).map_err(|e| anyhow::anyhow!(e))?;

    let backup = UpdateInstaller::backup_current_binary(&current_exe, None)
        .map_err(|e| anyhow::anyhow!(e))?;

    if let Err(e) = UpdateInstaller::install_binary(&new_binary, &current_exe).await {
        anyhow::bail!(
            "{}\nCould not replace {}. If it lives in a protected directory, re-run with sudo \
             or reinstall with install.sh.",
            e,
            current_exe.display()
        );
    }

    if let Err(e) = UpdateInstaller::verify_installation(&current_exe).await {
        eprintln!("✗ New binary failed verification: {}. Rolling back...", e);
        UpdateInstaller::rollback(&backup, &current_exe).map_err(|e| anyhow::anyhow!(e))?;
        anyhow::bail!("Update aborted; previous version restored from {}", backup.display());
    }

    let _ = UpdateInstaller::cleanup_old_backups(None);
    ensure_short_alias(&current_exe);
    println!(
        "✅ CleanServe {} → {} ({})",
        info.current_version,
        info.latest_version,
        current_exe.display()
    );

    Ok(())
}

/// Make `cs` available next to `cleanserve` (symlink on Unix).
pub fn ensure_short_alias(target: &Path) {
    #[cfg(unix)]
    if let Some(dir) = target.parent() {
        let alias = dir.join("cs");
        if std::fs::symlink_metadata(&alias).is_err() {
            let _ = std::os::unix::fs::symlink(target.file_name().unwrap_or_default(), alias);
        }
    }
    #[cfg(not(unix))]
    let _ = target;
}
