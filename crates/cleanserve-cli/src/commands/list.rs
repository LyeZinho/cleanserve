use anyhow::Context;
use cleanserve_core::{PhpDownloader, VersionManifest};
use std::path::Path;

pub async fn run(refresh: bool, installed_only: bool) -> anyhow::Result<()> {
    let php_dir = Path::new(".cleanserve").join("php");

    if installed_only {
        return list_installed(&php_dir);
    }

    let manifest = VersionManifest::fetch(refresh)
        .await
        .context("Failed to fetch version manifest")?;

    let downloader = PhpDownloader::new(&php_dir).ok();
    let versions = manifest.list_available();

    if versions.is_empty() {
        println!("No PHP versions available in manifest.");
        return Ok(());
    }

    println!("Available PHP versions:");
    println!();

    let mut installed_count = 0u32;

    for v in &versions {
        let is_installed = downloader
            .as_ref()
            .map(|d| d.is_installed(&v.version))
            .unwrap_or(false);

        if is_installed {
            installed_count += 1;
        }

        let size = v.platforms.first().map(|p| p.size_bytes).unwrap_or(0);
        let size_mb = size as f64 / 1_048_576.0;

        let marker = if is_installed { "  \u{2713} installed" } else { "" };
        println!("  {:<12} ({:.1} MB){}", v.version, size_mb, marker);
    }

    println!();
    println!(
        "Installed: {} | Available: {}",
        installed_count,
        versions.len()
    );

    Ok(())
}

fn list_installed(php_dir: &Path) -> anyhow::Result<()> {
    if !php_dir.exists() {
        println!("No PHP versions installed.");
        println!("Run 'cleanserve update --version 8.4' to install PHP.");
        return Ok(());
    }

    let mut versions: Vec<String> = std::fs::read_dir(php_dir)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.strip_prefix("php-").map(|s| s.to_string())
        })
        .collect();

    if versions.is_empty() {
        println!("No PHP versions installed.");
        println!("Run 'cleanserve update --version 8.4' to install PHP.");
        return Ok(());
    }

    versions.sort();
    println!("Installed PHP versions:");
    for version in &versions {
        println!("  \u{2022} {}", version);
    }

    Ok(())
}
