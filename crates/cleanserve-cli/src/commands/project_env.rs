use cleanserve_core::{
    prepare_project_php_ini, CleanServeConfig, FrameworkInfo, PhpManager,
};
use std::path::{Path, PathBuf};

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
