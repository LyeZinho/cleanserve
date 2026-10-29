use anyhow::Context;
use cleanserve_core::{CleanServeConfig, ProjectLayout};
use std::path::Path;

/// Load cleanserve.json or create it automatically for an existing PHP project.
pub async fn load_or_adopt(project_root: &Path) -> anyhow::Result<CleanServeConfig> {
    let config_path = project_root.join("cleanserve.json");
    if config_path.exists() {
        return CleanServeConfig::load(&config_path)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to load cleanserve.json");
    }

    if !project_root.join("composer.json").exists() {
        anyhow::bail!(
            "No cleanserve.json found. Run `cleanserve init` for a new project, \
             or `cleanserve init --existing` in an existing PHP repo."
        );
    }

    adopt_existing_project(project_root, None, None).await
}

pub async fn adopt_existing_project(
    project_root: &Path,
    name: Option<String>,
    php: Option<String>,
) -> anyhow::Result<CleanServeConfig> {
    let config_path = project_root.join("cleanserve.json");
    if config_path.exists() {
        anyhow::bail!("cleanserve.json already exists. Run `cleanserve uninstall` to remove it first.");
    }

    let layout = ProjectLayout::detect(project_root);
    let project_name = name.unwrap_or_else(|| {
        project_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_string())
    });
    let php_version = php
        .or(layout.suggested_php.clone())
        .unwrap_or_else(|| "8.4".to_string());

    let mut server = cleanserve_core::ServerConfig::default();
    server.root = layout.document_root_for_config(project_root);

    let config = CleanServeConfig {
        name: project_name,
        engine: cleanserve_core::EngineConfig {
            php: php_version,
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server,
        packages: None,
    };

    config
        .save(&config_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to write cleanserve.json")?;

    let pages_dir = project_root.join(".cleanserve/pages");
    std::fs::create_dir_all(&pages_dir).context("Failed to create .cleanserve/pages/")?;

    append_gitignore_cleanserve(project_root)?;

    println!("✓ Adopted existing project ({})", layout.framework);
    println!("✓ Document root: {}", config.server.root);
    println!("✓ Created cleanserve.json (no public/ scaffold)");

    Ok(config)
}

pub fn uninstall_project(project_root: &Path) -> anyhow::Result<()> {
    if cleanserve_core::docker_available() {
        if let Err(e) = cleanserve_core::teardown_all_for_project(project_root, true) {
            eprintln!("Warning: stack cleanup: {}", e);
        } else {
            println!("✓ Removed Docker stack (containers, volumes, local images)");
        }
    }

    let config_path = project_root.join("cleanserve.json");
    let cleanserve_dir = project_root.join(".cleanserve");

    if config_path.exists() {
        std::fs::remove_file(&config_path).context("Failed to remove cleanserve.json")?;
        println!("✓ Removed cleanserve.json");
    }

    if cleanserve_dir.exists() {
        std::fs::remove_dir_all(&cleanserve_dir).context("Failed to remove .cleanserve/")?;
        println!("✓ Removed .cleanserve/");
    }

    if !config_path.exists() && !cleanserve_dir.exists() {
        println!("Nothing to uninstall (no cleanserve.json or .cleanserve/)");
    }

    Ok(())
}

fn append_gitignore_cleanserve(project_root: &Path) -> anyhow::Result<()> {
    let gitignore_path = project_root.join(".gitignore");
    let marker = ".cleanserve/";
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, format!("# CleanServe\n{}\n", marker))?;
        return Ok(());
    }
    let content = std::fs::read_to_string(&gitignore_path)?;
    if content.contains(marker) {
        return Ok(());
    }
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&gitignore_path)?;
    use std::io::Write;
    writeln!(f, "\n# CleanServe")?;
    writeln!(f, "{}", marker)?;
    Ok(())
}
