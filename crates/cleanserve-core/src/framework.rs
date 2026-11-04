//! Framework Detection Module
//!
//! Automatically detects PHP frameworks (Laravel, Symfony, etc.)
//! and adjusts configuration accordingly. RF-P04

use crate::config::CleanServeConfig;
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
    /// Yii / Yii 3 (yiisoft)
    Yii,
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
            Framework::Yii => write!(f, "Yii"),
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

        // Yii / Yii 3 detection
        if Self::is_yii(root) {
            return Self::yii(root);
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

    fn is_yii(root: &Path) -> bool {
        let composer = root.join("composer.json");
        if composer.exists() {
            if let Ok(content) = std::fs::read_to_string(&composer) {
                if content.contains("yiisoft/") || content.contains("yiisoft/yii2") {
                    return true;
                }
            }
        }
        root.join("yii").exists() && composer.exists()
    }

    fn yii(root: &Path) -> Self {
        let entry_point = if root.join("frontend/web/index.php").exists() {
            root.join("frontend/web/index.php")
        } else if root.join("backend/web/index.php").exists() {
            root.join("backend/web/index.php")
        } else if root.join("web/index.php").exists() {
            root.join("web/index.php")
        } else if root.join("public/index.php").exists() {
            root.join("public/index.php")
        } else {
            root.join("index.php")
        };

        Self {
            framework: Framework::Yii,
            version: Self::detect_yii_version(root),
            writable_dirs: vec![
                root.join("runtime"),
                root.join("console/runtime"),
                root.join("frontend/runtime"),
                root.join("frontend/web/assets"),
                root.join("backend/runtime"),
                root.join("backend/web/assets"),
                root.join("public/assets"),
                root.join("web/assets"),
            ],
            entry_point,
            needs_storage_dirs: false,
            needs_var_dirs: false,
        }
    }

    fn detect_yii_version(root: &Path) -> Option<String> {
        let composer = root.join("composer.json");
        if composer.exists() {
            if let Ok(content) = std::fs::read_to_string(&composer) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(require) = json.get("require") {
                        for key in ["yiisoft/yii-core", "yiisoft/yii-console", "yiisoft/yii2"] {
                            if let Some(ver) = require.get(key).and_then(|v| v.as_str()) {
                                return Some(ver.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
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
            Framework::Yii => {
                vec![
                    "upload_max_filesize=128M".to_string(),
                    "post_max_size=128M".to_string(),
                ]
            }
            _ => vec![],
        }
    }
}

/// An application discovered in a project (one per document root).
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    /// Document root relative to the project root.
    pub root: PathBuf,
    pub entry_point: PathBuf,
}

/// An application ready to be served, with its public hostname.
#[derive(Debug, Clone)]
pub struct ServedApp {
    pub name: String,
    pub root: PathBuf,
    pub entry_point: PathBuf,
    pub domain: String,
    /// Front controller passed to PHP's built-in server, when present.
    pub router: Option<PathBuf>,
}

/// Discover the applications of a project.
///
/// Yii2 Advanced exposes two independent apps (`frontend/web` and
/// `backend/web`); Yii2 Basic and generic projects expose one. The order is
/// deterministic so the first app can be used as the default document root.
pub fn discover_apps(project_root: &Path) -> Vec<AppInfo> {
    let frontend = project_root.join("frontend/web/index.php");
    let backend = project_root.join("backend/web/index.php");
    if frontend.is_file() && backend.is_file() {
        return vec![
            AppInfo {
                name: "frontend".to_string(),
                root: PathBuf::from("frontend/web"),
                entry_point: frontend,
            },
            AppInfo {
                name: "backend".to_string(),
                root: PathBuf::from("backend/web"),
                entry_point: backend,
            },
        ];
    }

    let candidates: &[(&str, &str)] = &[
        ("app", "web"),
        ("app", "public"),
        ("app", "."),
    ];
    for (name, root) in candidates {
        let dir = project_root.join(root);
        let entry = dir.join("index.php");
        if entry.is_file() {
            return vec![AppInfo {
                name: (*name).to_string(),
                root: PathBuf::from(*root),
                entry_point: entry,
            }];
        }
    }

    Vec::new()
}

/// Merge configured apps with auto-discovery.
///
/// `cleanserve.json` wins when it has an `apps` map. Otherwise apps are
/// detected and named dynamically. A single app is served on `localhost`;
/// multi-app projects get `<name>.<domain_suffix>`.
pub fn resolve_apps(config: &CleanServeConfig, project_root: &Path) -> Vec<ServedApp> {
    let suffix = config.server.domain_suffix.trim_start_matches('.');
    let mut apps = Vec::new();

    if !config.apps.is_empty() {
        let mut names: Vec<&String> = config.apps.keys().collect();
        names.sort();
        let single = names.len() == 1;
        for name in names {
            let app = &config.apps[name];
            let root = project_root.join(&app.root);
            let router_rel = app.router.as_deref().unwrap_or("index.php");
            let entry = root.join(router_rel);
            let domain = app.domain.clone().unwrap_or_else(|| {
                if single {
                    "localhost".to_string()
                } else {
                    format!("{}.{}", name, suffix)
                }
            });
            apps.push(ServedApp {
                name: name.clone(),
                root,
                router: entry.is_file().then_some(entry.clone()),
                entry_point: entry,
                domain,
            });
        }
        return apps;
    }

    let discovered = discover_apps(project_root);
    if discovered.is_empty() {
        let root = project_root.join(config.server.root.trim_start_matches("./"));
        let entry = root.join("index.php");
        return vec![ServedApp {
            name: "app".to_string(),
            router: entry.is_file().then_some(entry.clone()),
            entry_point: entry,
            root,
            domain: "localhost".to_string(),
        }];
    }

    let single = discovered.len() == 1;
    for app in discovered {
        let domain = if single {
            "localhost".to_string()
        } else {
            format!("{}.{}", app.name, suffix)
        };
        apps.push(ServedApp {
            name: app.name,
            router: app.entry_point.is_file().then_some(app.entry_point.clone()),
            entry_point: app.entry_point,
            root: app.root,
            domain,
        });
    }
    apps
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

    fn temp_project(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cleanserve-apps-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discovers_yii2_advanced_apps() {
        let root = temp_project("advanced");
        std::fs::create_dir_all(root.join("frontend/web")).unwrap();
        std::fs::create_dir_all(root.join("backend/web")).unwrap();
        std::fs::write(root.join("frontend/web/index.php"), "<?php").unwrap();
        std::fs::write(root.join("backend/web/index.php"), "<?php").unwrap();

        let apps = discover_apps(&root);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "frontend");
        assert_eq!(apps[0].root, PathBuf::from("frontend/web"));
        assert_eq!(apps[1].name, "backend");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_apps_names_multi_app_domains() {
        let root = temp_project("domains");
        std::fs::create_dir_all(root.join("frontend/web")).unwrap();
        std::fs::create_dir_all(root.join("backend/web")).unwrap();
        std::fs::write(root.join("frontend/web/index.php"), "<?php").unwrap();
        std::fs::write(root.join("backend/web/index.php"), "<?php").unwrap();

        let config = CleanServeConfig {
            name: "demo".to_string(),
            engine: crate::EngineConfig {
                php: "8.4".to_string(),
                extensions: vec![],
                display_errors: true,
                memory_limit: None,
            },
            server: crate::ServerConfig::default(),
            packages: None,
            apps: Default::default(),
            stack: Default::default(),
        };

        let apps = resolve_apps(&config, &root);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].domain, "frontend.serve");
        assert_eq!(apps[1].domain, "backend.serve");
        assert!(apps[0].router.is_some());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_single_app_uses_localhost() {
        let root = temp_project("single");
        std::fs::create_dir_all(root.join("public")).unwrap();
        std::fs::write(root.join("public/index.php"), "<?php").unwrap();

        let config = CleanServeConfig {
            name: "demo".to_string(),
            engine: crate::EngineConfig {
                php: "8.4".to_string(),
                extensions: vec![],
                display_errors: true,
                memory_limit: None,
            },
            server: crate::ServerConfig::default(),
            packages: None,
            apps: Default::default(),
            stack: Default::default(),
        };

        let apps = resolve_apps(&config, &root);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].domain, "localhost");

        let _ = std::fs::remove_dir_all(&root);
    }
}
