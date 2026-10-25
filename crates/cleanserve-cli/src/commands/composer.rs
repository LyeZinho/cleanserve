use anyhow::Context;
use cleanserve_core::{CleanServeConfig, ComposerTool, ensure_project_php};
use std::path::Path;

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;

    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let config = CleanServeConfig::load(config_path).context("Failed to load config")?;
    let php_version = &config.engine.php;

    let php_path = ensure_project_php(&project_root, php_version)
        .await
        .context(format!(
            "PHP {} not available. Run 'cleanserve update --version {}' or start the server once to download it.",
            php_version, php_version
        ))?;

    let composer_args: Vec<String> = if args.is_empty() {
        vec!["install".to_string()]
    } else {
        args
    };

    println!("Running Composer with PHP {} (project-local)", php_version);
    println!("composer {}\n", composer_args.join(" "));

    let code = ComposerTool::run(&php_path, &project_root, &composer_args).await?;
    if code != 0 {
        anyhow::bail!("Composer exited with code {}", code);
    }

    Ok(())
}
