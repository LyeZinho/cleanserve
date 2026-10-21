use anyhow::Context;
use cleanserve_core::{
    prepare_project_php_ini, CleanServeConfig, FrameworkInfo, PhpManager,
};
use std::path::{Path, PathBuf};

use super::init::html_pages;

/// Resolve `server.root` under the project and create a default `public/` tree when missing.
pub fn ensure_document_root(
    project_root: &Path,
    root: &str,
    project_name: &str,
    php_version: &str,
) -> anyhow::Result<PathBuf> {
    let rel = root.trim_start_matches("./");
    let php_root = project_root.join(rel);

    if php_root.is_dir() {
        return php_root
            .canonicalize()
            .or_else(|_| Ok(php_root));
    }

    let stem = rel.trim_end_matches('/').trim_end_matches('\\');
    if stem == "public" {
        std::fs::create_dir_all(&php_root).context("Failed to create public/ document root")?;
        let pages_dir = project_root.join(".cleanserve/pages");
        if !pages_dir.join("index.html").exists() {
            std::fs::create_dir_all(&pages_dir).context("Failed to create .cleanserve/pages")?;
            html_pages::write_default_pages(&pages_dir, project_name, php_version)?;
        }
        println!("✓ Created missing public/ document root");
        return php_root.canonicalize().or_else(|_| Ok(php_root));
    }

    anyhow::bail!(
        "Document root \"{root}\" does not exist at {path}.\n\
         Create the directory, update server.root in cleanserve.json, or run `cleanserve up` from your PHP app directory.",
        root = root,
        path = php_root.display()
    );
}

/// Regenerate `.cleanserve/php.ini` when `cleanserve.json` exists.
pub fn project_php_ini(php_binary: &Path) -> Option<PathBuf> {
    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        return None;
    }

    let config = CleanServeConfig::load(config_path).ok()?;
    let project_root = std::env::current_dir().ok()?;
    let framework = FrameworkInfo::detect(&project_root);
    let resolved = PhpManager::new()
        .ok()
        .and_then(|m| m.resolve_installed_version(&config.engine.php))
        .unwrap_or_else(|| config.engine.php.clone());

    prepare_project_php_ini(
        &project_root,
        php_binary,
        &resolved,
        &config.engine,
        &framework,
    )
    .ok()
    .map(|runtime| runtime.ini_path)
}
