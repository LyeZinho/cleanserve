use anyhow::Context;
use cleanserve_core::PhpDownloader;
use std::path::Path;

pub async fn run(version: String) -> anyhow::Result<()> {
    let php_dir = Path::new(".cleanserve").join("php");
    let downloader = PhpDownloader::new(&php_dir)
        .context("Failed to initialize PHP downloader")?;

    if !downloader.is_installed(&version) {
        println!("PHP {} is not installed. Downloading...", version);
        downloader.download(&version).await
            .context(format!("Failed to download PHP {}", version))?;
    }

    let resolved = find_resolved_version(&php_dir, &version)?;

    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let mut config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load cleanserve.json")?;

    config.engine.php = resolved.clone();
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;

    println!("\u{2713} Switched to PHP {}", resolved);

    Ok(())
}

fn find_resolved_version(php_dir: &Path, query: &str) -> anyhow::Result<String> {
    if php_dir.join(format!("php-{}", query)).exists() {
        return Ok(query.to_string());
    }

    let prefix = format!("php-{}.", query);
    let mut matches: Vec<String> = std::fs::read_dir(php_dir)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with(&prefix) {
                Some(name.strip_prefix("php-")?.to_string())
            } else {
                None
            }
        })
        .collect();

    if matches.is_empty() {
        anyhow::bail!("PHP {} was downloaded but directory not found", query);
    }

    matches.sort_by(|a, b| {
        let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
        parse(a).cmp(&parse(b))
    });

    Ok(matches.pop().unwrap())
}
