//! Shared PHP runtime resolution and dev baseline checks (session, phar, …).

use crate::{CleanServeError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// Modules required for typical web apps (Yii, Laravel, Symfony, …).
pub const DEV_BASELINE_MODULES: &[&str] = &["session", "phar", "mbstring", "openssl"];

pub fn shared_runtimes_dir() -> Result<PathBuf> {
    let dir = dirs::home_dir()
        .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
        .join(".cleanserve")
        .join("runtimes");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn legacy_global_bin_dir() -> Result<PathBuf> {
    let dir = dirs::home_dir()
        .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
        .join(".cleanserve")
        .join("bin");
    Ok(dir)
}

pub fn list_loaded_modules(php_bin: &Path) -> Vec<String> {
    Command::new(php_bin)
        .arg("-m")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && *l != "[PHP Modules]" && *l != "[Zend Modules]")
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub fn missing_modules(php_bin: &Path, required: &[&str]) -> Vec<String> {
    let loaded = list_loaded_modules(php_bin);
    required
        .iter()
        .filter(|m| !loaded.iter().any(|l| l.eq_ignore_ascii_case(m)))
        .map(|s| (*s).to_string())
        .collect()
}

pub fn meets_dev_baseline(php_bin: &Path) -> bool {
    missing_modules(php_bin, DEV_BASELINE_MODULES).is_empty()
}

fn minor_version(version: &str) -> String {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}

/// Collect candidate PHP binaries under `~/.cleanserve/bin/php-{label}/`.
fn legacy_candidates_for_label(base: &Path, label: &str) -> Vec<PathBuf> {
    let root = base.join(format!("php-{}", label));
    let mut out = vec![
        root.join("php"),
        root.join("bin").join("php"),
    ];

    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let platform_path = entry.path();
            if !platform_path.is_dir() {
                continue;
            }
            out.push(platform_path.join("php"));
            out.push(platform_path.join("bin").join("php"));
            if let Ok(bin_entries) = std::fs::read_dir(&platform_path) {
                for bin_entry in bin_entries.flatten() {
                    let p = bin_entry.path();
                    if p.is_file() {
                        let name = bin_entry.file_name().to_string_lossy().into_owned();
                        if name.starts_with("php") {
                            out.push(p);
                        }
                    }
                }
            }
        }
    }

    out
}

/// Prefer a PHP binary that satisfies the dev baseline (session, phar, …).
pub fn resolve_with_fallback(primary: PathBuf, version: &str, resolved: &str) -> PathBuf {
    if meets_dev_baseline(&primary) {
        return primary;
    }

    let missing = missing_modules(&primary, DEV_BASELINE_MODULES);
    warn!(
        "Project PHP at {} is missing modules: {} — searching for a compatible runtime",
        primary.display(),
        missing.join(", ")
    );

    if let Ok(legacy_base) = legacy_global_bin_dir() {
        let mut labels = Vec::new();
        for v in [resolved, version] {
            if !labels.contains(&v.to_string()) {
                labels.push(v.to_string());
            }
            let minor = minor_version(v);
            if !labels.contains(&minor) {
                labels.push(minor);
            }
        }

        for label in labels {
            for candidate in legacy_candidates_for_label(&legacy_base, &label) {
                if candidate.is_file() && meets_dev_baseline(&candidate) {
                    info!(
                        "Using compatible PHP runtime at {} (replacing minimal build)",
                        candidate.display()
                    );
                    return candidate;
                }
            }
        }
    }

    primary
}

pub fn baseline_error_message(php_bin: &Path, version: &str) -> String {
    let missing = missing_modules(php_bin, DEV_BASELINE_MODULES);
    format!(
        "PHP at {} is missing required modules: {}.\n\
         Install a full dev runtime under ~/.cleanserve/bin/php-{}/ or re-run:\n\
         cleanserve update --version {}",
        php_bin.display(),
        missing.join(", "),
        minor_version(version),
        minor_version(version)
    )
}
