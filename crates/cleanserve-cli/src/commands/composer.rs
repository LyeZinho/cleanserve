use anyhow::Context;
use crate::commands::project_config;
use cleanserve_core::{ComposerTool, ensure_docker, ensure_project_php};

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    ensure_docker().map_err(|e| anyhow::anyhow!(e))?;

    let project_root = std::env::current_dir().context("Cannot read current directory")?;

    let config = project_config::load_or_adopt(&project_root).await?;
    let php_version = &config.engine.php;

    let php_path = ensure_project_php(&project_root, php_version)
        .await
        .context(format!(
            "PHP {} not available. Run 'cleanserve update --version {}' first.",
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
