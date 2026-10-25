//! Detect document root, router, and PHP version from the project tree.

use crate::framework::{Framework, FrameworkInfo};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProjectLayout {
    pub framework: Framework,
    pub document_root: PathBuf,
    pub router_script: Option<PathBuf>,
    pub suggested_php: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ComposerRequire {
    #[serde(default)]
    require: std::collections::HashMap<String, serde_json::Value>,
}

impl ProjectLayout {
    pub fn detect(project_root: &Path) -> Self {
        let info = FrameworkInfo::detect(project_root);
        let composer_php = read_composer_php_constraint(project_root);

        let (document_root, router_script) = match info.framework {
            Framework::Yii2 => yii2_layout(project_root),
            Framework::Laravel | Framework::Symfony => {
                let root = project_root.join("public");
                let router = root.join("index.php");
                let router = router.exists().then_some(router);
                (root, router)
            }
            _ => {
                let root = if project_root.join("public").join("index.php").exists() {
                    project_root.join("public")
                } else if project_root.join("frontend/web/index.php").exists() {
                    project_root.join("frontend/web")
                } else if project_root.join("web/index.php").exists() {
                    project_root.join("web")
                } else {
                    project_root.join("public")
                };
                let router = root.join("index.php");
                let router = router.exists().then_some(router);
                (root, router)
            }
        };

        Self {
            framework: info.framework,
            document_root,
            router_script,
            suggested_php: composer_php,
        }
    }

    pub fn document_root_for_config(&self, project_root: &Path) -> String {
        self.document_root
            .strip_prefix(project_root)
            .map(|p| {
                let s = p.to_string_lossy();
                if s.ends_with('/') {
                    s.to_string()
                } else {
                    format!("{}/", s)
                }
            })
            .unwrap_or_else(|_| "public/".to_string())
    }
}

fn yii2_layout(project_root: &Path) -> (PathBuf, Option<PathBuf>) {
    let root = project_root.join("frontend/web");
    let router = root.join("index.php");
    let router = router.exists().then_some(router);
    (root, router)
}

fn read_composer_php_constraint(project_root: &Path) -> Option<String> {
    let path = project_root.join("composer.json");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let parsed: ComposerRequire = serde_json::from_str(&content).ok()?;
    parsed
        .require
        .get("php")
        .and_then(|v| v.as_str())
        .map(suggest_php_minor_from_constraint)
}

/// Map composer constraints like ">=8.2" to a CleanServe minor version string.
pub fn suggest_php_minor_from_constraint(constraint: &str) -> String {
    let digits: String = constraint
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let parts: Vec<&str> = digits.split('.').filter(|p| !p.is_empty()).collect();
    match parts.len() {
        0 => "8.4".to_string(),
        1 => parts[0].to_string(),
        _ => {
            let major = parts[0].parse::<u32>().unwrap_or(8);
            let minor = parts[1].parse::<u32>().unwrap_or(4);
            // Prefer a line that is still published in php-runtimes (8.2 is EOL in the manifest).
            if major == 8 && minor <= 2 {
                "8.4".to_string()
            } else {
                format!("{}.{}", parts[0], parts[1])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_php_from_composer_constraint() {
        assert_eq!(suggest_php_minor_from_constraint(">=8.2"), "8.4");
        assert_eq!(suggest_php_minor_from_constraint("^8.4.0"), "8.4");
    }

    #[test]
    fn test_detect_vollink_style_layout() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"php":">=8.2","yiisoft/yii2":"~2.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("common/config")).unwrap();
        std::fs::create_dir_all(dir.path().join("frontend/web")).unwrap();
        std::fs::write(dir.path().join("frontend/web/index.php"), "<?php").unwrap();

        let layout = ProjectLayout::detect(dir.path());
        assert_eq!(layout.framework, Framework::Yii2);
        assert_eq!(layout.suggested_php.as_deref(), Some("8.4"));
        assert!(layout.document_root.ends_with("frontend/web"));
    }
}
