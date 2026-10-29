use anyhow::Context;
use crate::commands::project_config;
use cleanserve_core::ensure_project_php;
use std::process::Command;

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    if args.is_empty() {
        anyhow::bail!("Usage: cleanserve php <script> [args...]  (e.g. cleanserve php init)");
    }

    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let config = project_config::load_or_adopt(&project_root).await?;
    let php_path = ensure_project_php(&project_root, &config.engine.php)
        .await
        .context("Failed to prepare project PHP")?;

    let mut cmd = Command::new(&php_path);
    cmd.current_dir(&project_root);
    if args[0].starts_with('-') {
        cmd.args(&args);
    } else {
        cmd.arg(&args[0]).args(&args[1..]);
    }

    let status = cmd.status().context("Failed to run PHP")?;

    if !status.success() {
        anyhow::bail!("PHP exited with code {:?}", status.code());
    }

    Ok(())
}
