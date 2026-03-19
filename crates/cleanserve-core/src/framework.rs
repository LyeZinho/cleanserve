//! Framework Detection Module
//!
//! Automatically detects PHP frameworks (Laravel, Symfony, etc.)
//! and adjusts configuration accordingly. RF-P04

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Detected PHP frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Framework {
    /// Laravel framework
    Laravel,
    /// Symfony framework  
    Symfony,
    /// CodeIgniter framework
    CodeIgniter,
    /// WordPress CMS
    WordPress,
    /// Drupal CMS
    Drupal,
    /// Unknown/custom framework
    Unknown,
}

impl std::fmt::Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Framework::Laravel => write!(f, "Laravel"),
            Framework::Symfony => write!(f, "Symfony"),
            Framework::CodeIgniter => write!(f, "CodeIgniter"),
            Framework::WordPress => write!(f, "WordPress"),
            Framework::Drupal => write!(f, "Drupal"),
            Framework::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Framework detection result
#[derive(Debug, Clone)]
pub struct FrameworkInfo {
    pub framework: Framework,
    pub version: Option<String>,
    pub writable_dirs: Vec<PathBuf>,
    pub entry_point: PathBuf,
    pub needs_storage_dirs: bool,
    pub needs_var_dirs: bool,
}

impl FrameworkInfo {
    /// Detect framework from project root
    pub fn detect(root: &Path) -> Self {
        // Laravel detection
        if root.join("artisan").exists() && root.join("composer.json").exists() {
            return Self::laravel(root);
        }

        // Symfony detection
        if root.join("bin/console").exists() && root.join("composer.json").exists() {
            return Self::symfony(root);
        }

        // CodeIgniter detection
        if root.join("index.php").exists() && root.join("system").exists() {
            return Self::codeigniter(root);
        }

        // WordPress detection
        if root.join("wp-config.php").exists() || root.join("wp-load.php").exists() {
            return Self::wordpress(root);
        }

        // Drupal detection
        if root.join("core").exists() && root.join("autoload.php").exists() {
            return Self::drupal(root);
        }

        // Default: vanilla PHP or unknown
        Self::unknown(root)
    }

    fn laravel(root: &Path) -> Self {
        let version = Self::detect_laravel_version(root);

        Self {
            framework: Framework::Laravel,
            version,
            writable_dirs: vec![
                root.join("storage"),
                root.join("storage/app"),
                root.join("storage/app/public"),
                root.join("storage/framework"),
                root.join("storage/framework/cache"),
                root.join("storage/framework/sessions"),
                root.join("storage/framework/views"),
                root.join("storage/logs"),
                root.join("bootstrap/cache"),
            ],
            entry_point: root.join("public/index.php"),
            needs_storage_dirs: true,
            needs_var_dirs: false,
        }
    }

    fn symfony(root: &Path) -> Self {
        let version = Self::detect_symfony_version(root);

        Self {
            framework: Framework::Symfony,
            version,
            writable_dirs: vec![
                root.join("var"),
                root.join("var/cache"),
                root.join("var/log"),
                root.join("var/sessions"),
            ],
            entry_point: root.join("public/index.php"),
            needs_storage_dirs: false,
            needs_var_dirs: true,
        }
    }

    fn codeigniter(root: &Path) -> Self {
        Self {
            framework: Framework::CodeIgniter,
            version: None,
            writable_dirs: vec![
                root.join("application/cache"),
                root.join("application/logs"),
                root.join("writable"),
            ],
            entry_point: root.join("index.php"),
            needs_storage_dirs: true,
            needs_var_dirs: false,
        }
    }

    fn wordpress(root: &Path) -> Self {
        Self {
            framework: Framework::WordPress,
            version: Self::detect_wordpress_version(root),
            writable_dirs: vec![
                root.join("wp-content"),
                root.join("wp-content/uploads"),
                root.join("wp-content/cache"),
            ],
            entry_point: root.join("index.php"),
            needs_storage_dirs: false,
            needs_var_dirs: false,
        }
    }

    fn drupal(root: &Path) -> Self {
        Self {
            framework: Framework::Drupal,
            version: Self::detect_drupal_version(root),
            writable_dirs: vec![
                root.join("sites/default/files"),
                root.join("sites/default/settings.php"),
            ],
            entry_point: root.join("index.php"),
            needs_storage_dirs: false,
            needs_var_dirs: false,
        }
    }

    fn unknown(root: &Path) -> Self {
        Self {
            framework: Framework::Unknown,
            version: None,
            writable_dirs: vec![root.join("storage")],
            entry_point: root.join("index.php"),
            needs_storage_dirs: false,
            needs_var_dirs: false,
        }
    }

    fn detect_laravel_version(root: &Path) -> Option<String> {
        // Check composer.json
        let composer = root.join("composer.json");
        if composer.exists() {
            if let Ok(content) = std::fs::read_to_string(&composer) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Check require section for laravel/framework
                    if let Some(require) =
                        json.get("require").and_then(|r| r.get("laravel/framework"))
                    {
                        return Some(require.as_str().unwrap_or("unknown").to_string());
                    }
                }
            }
        }

        // Check bootstrap/app.php for version hints
        let bootstrap_app = root.join("bootstrap/app.php");
        if bootstrap_app.exists() {
            let content = std::fs::read_to_string(&bootstrap_app).ok()?;
            if content.contains("Application::VERSION") {
                return Some("11.x".to_string()); // Laravel 11 pattern
            }
        }

        None
    }

    fn detect_symfony_version(root: &Path) -> Option<String> {
        let composer = root.join("composer.json");
        if composer.exists() {
            if let Ok(content) = std::fs::read_to_string(&composer) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(require) = json
                        .get("require")
                        .and_then(|r| r.get("symfony/framework-bundle"))
                    {
                        return Some(require.as_str().unwrap_or("unknown").to_string());
                    }
                }
            }
        }
        None
    }

    fn detect_wordpress_version(root: &Path) -> Option<String> {
        let wp_version = root.join("wp-includes/version.php");
        if wp_version.exists() {
            if let Ok(content) = std::fs::read_to_string(&wp_version) {
                // Extract $wp_version
                for line in content.lines() {
                    if line.contains("$wp_version") && !line.contains("//") {
                        if let Some(start) = line.find('\'') {
                            if let Some(end) = line[start + 1..].find('\'') {
                                return Some(line[start + 1..start + 1 + end].to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn detect_drupal_version(root: &Path) -> Option<String> {
        let composer = root.join("composer.json");
        if composer.exists() {
            if let Ok(content) = std::fs::read_to_string(&composer) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(require) = json.get("require").and_then(|r| r.get("drupal/core")) {
                        return Some(require.as_str().unwrap_or("unknown").to_string());
                    }
                }
            }
        }
        None
    }

    /// Get PHP configuration adjustments for this framework
    pub fn get_php_config(&self) -> Vec<String> {
        match self.framework {
            Framework::Laravel => {
                vec![
                    "session.save_handler=file".to_string(),
                    "session.save_path=storage/framework/sessions".to_string(),
                    "upload_max_filesize=128M".to_string(),
                    "post_max_size=128M".to_string(),
                ]
            }
            Framework::Symfony => {
                vec![
                    "session.save_handler=native".to_string(),
                    "upload_max_filesize=128M".to_string(),
                    "post_max_size=128M".to_string(),
                ]
            }
            Framework::WordPress => {
                vec![
                    "memory_limit=256M".to_string(),
                    "upload_max_filesize=64M".to_string(),
                    "post_max_size=64M".to_string(),
                    "max_execution_time=300".to_string(),
                ]
            }
            _ => vec![],
        }
    }
}

/// Ensure writable directories exist with correct permissions
pub fn ensure_writable_dirs(info: &FrameworkInfo) -> Vec<std::io::Result<()>> {
    info.writable_dirs
        .iter()
        .map(|dir| {
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }

            // Try to make writable (Unix-style)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = dir.metadata() {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(dir, perms)?;
                }
            }

            Ok(())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_display() {
        assert_eq!(Framework::Laravel.to_string(), "Laravel");
        assert_eq!(Framework::Unknown.to_string(), "Unknown");
    }
}
