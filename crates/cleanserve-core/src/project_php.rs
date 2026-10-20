use crate::php_manager::find_php_binary;
use crate::{CleanServeError, EngineConfig, FrameworkInfo, Result};
use std::path::{Path, PathBuf};

/// Project-local PHP runtime files under `.cleanserve/`.
pub struct ProjectPhpRuntime {
    pub ini_path: PathBuf,
}

/// Write `.cleanserve/php.ini` and return its path for `php -c`.
pub fn prepare_project_php_ini(
    project_root: &Path,
    php_binary: &Path,
    resolved_version: &str,
    engine: &EngineConfig,
    framework: &FrameworkInfo,
) -> Result<ProjectPhpRuntime> {
    let cleanserve_dir = project_root.join(".cleanserve");
    std::fs::create_dir_all(&cleanserve_dir).map_err(|e| {
        CleanServeError::Config(format!("Cannot create .cleanserve/: {}", e))
    })?;

    let ini_path = cleanserve_dir.join("php.ini");
    let install_root = php_install_root(php_binary);
    let ini = generate_php_ini(
        resolved_version,
        install_root.as_deref(),
        engine,
        framework,
    )?;

    std::fs::write(&ini_path, ini).map_err(|e| {
        CleanServeError::Config(format!("Failed to write {}: {}", ini_path.display(), e))
    })?;

    Ok(ProjectPhpRuntime { ini_path })
}

fn php_install_root(php_binary: &Path) -> Option<PathBuf> {
    for ancestor in php_binary.ancestors() {
        if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("php-")
                && name.chars().nth(4).is_some_and(|c| c.is_ascii_digit())
                && find_php_binary(ancestor).is_some()
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn generate_php_ini(
    version: &str,
    install_root: Option<&Path>,
    engine: &EngineConfig,
    framework: &FrameworkInfo,
) -> Result<String> {
    let mut ini = String::new();
    ini.push_str("; CleanServe generated php.ini\n");
    ini.push_str(&format!("; PHP {}\n\n", version));
    ini.push_str("[PHP]\n");
    ini.push_str("engine = On\n");
    ini.push_str(&format!(
        "display_errors = {}\n",
        if engine.display_errors { "On" } else { "Off" }
    ));
    ini.push_str("display_startup_errors = On\n");
    ini.push_str("log_errors = On\n");
    ini.push_str("error_log = php://stderr\n");
    ini.push_str("error_reporting = E_ALL\n");
    ini.push_str(&format!(
        "memory_limit = {}\n",
        engine.memory_limit.as_deref().unwrap_or("128M")
    ));
    ini.push_str("variables_order = EGPCS\n");
    ini.push_str("cgi.fix_pathinfo = 1\n");

    if let Some(root) = install_root {
        let ext_dir = extension_dir_for_install(root);
        if ext_dir.is_dir() {
            ini.push_str(&format!("extension_dir = \"{}\"\n", ext_dir.display()));
            if !engine.extensions.is_empty() {
                ini.push_str("\n; Project extensions\n");
                for ext in &engine.extensions {
                    let name = ext.strip_prefix("extension=").unwrap_or(ext.as_str());
                    ini.push_str(&format!("extension={}\n", name));
                }
            }
        }
    }

    let framework_lines = framework.get_php_config();
    if !framework_lines.is_empty() {
        ini.push_str("\n; Framework\n");
        for line in framework_lines {
            ini.push_str(&format!("{}\n", line));
        }
    }

    Ok(ini)
}

fn extension_dir_for_install(install_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        return install_root.join("ext");
    }
    #[cfg(not(windows))]
    {
        install_root.join("lib").join("php").join("extensions")
    }
}
