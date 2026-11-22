//! Project-local PHP: resolve/validate the runtime (shared under ~/.cleanserve/runtimes/)
//! and generate the per-project `.cleanserve/php.ini`.

use crate::php_manager::find_php_binary;
use crate::php_runtime::{
    baseline_error_message, meets_dev_baseline, missing_modules, resolve_with_fallback,
    shared_runtimes_dir, DEV_BASELINE_MODULES,
};
use crate::{CleanServeError, EngineConfig, FrameworkInfo, PhpDownloader, Result, VersionManifest};
use std::path::{Path, PathBuf};
use tracing::info;

pub fn project_php_dir(project_root: &Path) -> PathBuf {
    project_root.join(".cleanserve").join("php")
}

/// Ensure PHP is installed and return a binary suitable for web + Composer.
pub async fn ensure_project_php(project_root: &Path, version: &str) -> Result<PathBuf> {
    let path = ensure_project_php_raw(project_root, version).await?;
    let validated = validate_project_php(&path, version)?;
    Ok(validated)
}

/// A PHP binary chosen for running the project, plus any dev-baseline modules it lacks.
pub struct DevPhp {
    pub path: PathBuf,
    pub missing_modules: Vec<String>,
}

/// Prefer a complete runtime (`~/.cleanserve/bin/php-X.Y`) over a minimal build, but unlike
/// [`validate_project_php`] never fail: the caller decides what to do about missing modules
/// (the server warns and still runs, which is enough for plain PHP apps).
pub fn resolve_dev_php(path: &Path, version: &str) -> DevPhp {
    let resolved = minor_from_path_or_version(path, version);
    let chosen = resolve_with_fallback(path.to_path_buf(), version, &resolved);
    let missing = missing_modules(&chosen, DEV_BASELINE_MODULES);
    DevPhp { path: chosen, missing_modules: missing }
}

pub fn validate_project_php(path: &Path, version: &str) -> Result<PathBuf> {
    let resolved = minor_from_path_or_version(path, version);
    let chosen = resolve_with_fallback(path.to_path_buf(), version, &resolved);
    if !meets_dev_baseline(&chosen) {
        return Err(CleanServeError::Config(baseline_error_message(&chosen, version)));
    }
    Ok(chosen)
}

async fn ensure_project_php_raw(project_root: &Path, version: &str) -> Result<PathBuf> {
    let php_dir = shared_runtimes_dir()?;
    let downloader = PhpDownloader::new(&php_dir)?;

    let manifest = VersionManifest::fetch(false).await?;
    let resolved = manifest
        .resolve_version(version)
        .map(|v| v.version.clone())
        .ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} is not available. Run 'cleanserve list' to see installable versions.",
                version
            ))
        })?;

    if resolved != version {
        info!(
            "Requested PHP {}; using compatible runtime {}",
            version,
            resolved
        );
    }

    if let Some(path) = downloader.get_php_exe(&resolved) {
        return Ok(path);
    }
    if let Some(path) = downloader.get_php_exe(version) {
        return Ok(path);
    }

    // Legacy project-local installs (pre-shared-cache)
    if let Some(path) = find_project_php(&project_php_dir(project_root)) {
        return Ok(path);
    }

    downloader.download(version).await?;

    downloader
        .get_php_exe(&resolved)
        .or_else(|| downloader.get_php_exe(version))
        .or_else(|| find_project_php(&project_php_dir(project_root)))
        .or_else(|| find_project_php(&php_dir))
        .ok_or_else(|| CleanServeError::Config(format!(
            "PHP {} was downloaded but the binary was not found under {}",
            resolved,
            php_dir.display()
        )))
}

fn minor_from_path_or_version(path: &Path, version: &str) -> String {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with("php-") {
            return name.strip_prefix("php-").unwrap_or(version).to_string();
        }
    }
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() >= 3 {
        format!("{}.{}.{}", parts[0], parts[1], parts[2])
    } else {
        version.to_string()
    }
}

/// Find any installed PHP binary under `.cleanserve/php/` (fallback when version dir layout differs).
pub fn find_project_php(php_dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(php_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("php-") {
                    let path = entry.path();

                    let php_bin = path.join("php");
                    if php_bin.exists() {
                        return Some(php_bin);
                    }

                    let php_bin_alt = path.join("bin").join("php");
                    if php_bin_alt.exists() {
                        return Some(php_bin_alt);
                    }

                    if let Ok(platform_entries) = std::fs::read_dir(&path) {
                        for platform_entry in platform_entries.flatten() {
                            if let Some(platform_name) = platform_entry.file_name().to_str() {
                                if platform_name.contains("x64") || platform_name == "bin" {
                                    let platform_path = platform_entry.path();
                                    if let Ok(bin_entries) = std::fs::read_dir(&platform_path) {
                                        for bin_entry in bin_entries.flatten() {
                                            if let Some(bin_name) = bin_entry.file_name().to_str() {
                                                if bin_name.starts_with("php") && bin_entry.path().is_file() {
                                                    return Some(bin_entry.path());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

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
