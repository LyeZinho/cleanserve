use anyhow::Context;
use crate::commands::project_config;
use cleanserve_core::{ComposerTool, ensure_project_php};

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
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

    // Docker is only a fallback: with a PHP that includes phar, Composer runs without it.
    let backend = ComposerTool::detect_backend(&php_path).ok_or_else(|| {
        anyhow::anyhow!(
            "Composer is not available: the project PHP has no phar extension, there is no \
`composer` on PATH and Docker is not running.\nInstall Composer (https://getcomposer.org/download/) \
or Docker (`cleanserve setup docker`). Run `cleanserve doctor` to check your setup."
        )
    })?;

    println!("composer {}  [{}]\n", composer_args.join(" "), backend.describe());

    let code = ComposerTool::run(&php_path, &project_root, &composer_args).await?;
    if code != 0 {
        // Composer already printed its own diagnostics; just propagate its exit code.
        std::process::exit(code);
    }

    Ok(())
}
