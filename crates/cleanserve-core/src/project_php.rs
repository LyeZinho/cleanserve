//! Project-local PHP resolution (shared runtimes under ~/.cleanserve/runtimes/)

use crate::php_runtime::{resolve_with_fallback, shared_runtimes_dir, meets_dev_baseline, baseline_error_message};
use crate::{CleanServeError, PhpDownloader, Result, VersionManifest};
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
