use anyhow::Context;
use cleanserve_core::{PhpDownloader, PhpManager};
use std::path::Path;
use std::process::{Command, Stdio};

const DEFAULT_PHP: &str = "8.4";

pub async fn run(args: Vec<String>, php_override: Option<String>) -> anyhow::Result<()> {
    let php_version = php_override
        .or_else(|| {
            let config_path = Path::new("cleanserve.json");
            if config_path.exists() {
                cleanserve_core::CleanServeConfig::load(config_path)
                    .ok()
                    .map(|c| c.engine.php)
            } else {
                None
            }
        })
        .unwrap_or_else(|| DEFAULT_PHP.to_string());

    let manager = PhpManager::new().context("Failed to init PHP manager")?;
    let php_path = if let Some(path) = manager.get_path(&php_version) {
        path
    } else {
        println!("📦 PHP {} not found. Downloading...", php_version);
        PhpDownloader::global()
            .context("Failed to initialize PHP downloader")?
            .ensure(&php_version)
            .await
            .with_context(|| format!("PHP {} not found. Run 'cleanserve update --version {}' first.", php_version, php_version))?
    };

    let php_bin_dir = php_path.parent().unwrap_or(Path::new("."));
    let mut dirs = vec![php_bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&existing));
    }
    let path_env = std::env::join_paths(dirs).unwrap_or_else(|_| php_bin_dir.as_os_str().to_os_string());

    let mut cmd = Command::new(&php_path);
    if let Some(ini) = crate::commands::project_env::project_php_ini(&php_path) {
        cmd.arg("-c").arg(ini);
    }
    let status = cmd
        .args(&args)
        .env("PATH", path_env)
        .env("PHP_BINARY", &php_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to execute PHP")?;

    if !status.success() {
        anyhow::bail!("PHP exited with code: {:?}", status.code());
    }

    Ok(())
}
