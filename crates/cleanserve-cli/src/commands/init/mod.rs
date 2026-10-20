use anyhow::Context;
use cleanserve_core::{ensure_writable_dirs, Framework, FrameworkInfo};
use std::path::{Path, PathBuf};

pub mod html_pages;

pub async fn run(
    name: Option<String>,
    php: Option<String>,
    quickstart: bool,
) -> anyhow::Result<()> {
    let hints = composer_hints();
    let framework = FrameworkInfo::detect(Path::new("."));

    let project_name = name
        .or(hints.name)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "my-project".to_string());

    let php = php
        .or(hints.php)
        .unwrap_or_else(|| "8.4".to_string());

    let config_path = Path::new("cleanserve.json");
    if config_path.exists() {
        anyhow::bail!("cleanserve.json already exists. Remove it first.");
    }

    let mut server = cleanserve_core::ServerConfig::default();
    if framework.framework != Framework::Unknown {
        if let Some(root) = document_root_for(&framework) {
            server.root = root;
        }
    } else if Path::new("public/index.php").exists() {
        server.root = "public/".to_string();
    } else if Path::new("index.php").exists() {
        server.root = "./".to_string();
    }

    let config = cleanserve_core::CleanServeConfig {
        name: project_name.clone(),
        engine: cleanserve_core::EngineConfig {
            php: php.clone(),
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server,
        packages: None,
    };

    config
        .save(config_path)
        .context("Failed to save cleanserve.json")?;

    println!("✓ Created cleanserve.json");
    if framework.framework != Framework::Unknown {
        println!(
            "✓ Detected {}{}",
            framework.framework,
            framework
                .version
                .as_deref()
                .map(|v| format!(" ({v})"))
                .unwrap_or_default()
        );
    }

    let cleanserve_dir = Path::new(".cleanserve");
    let cleanserve_pages_dir = cleanserve_dir.join("pages");
    std::fs::create_dir_all(&cleanserve_pages_dir)
        .context("Failed to create .cleanserve/pages/ directory")?;

    println!("✓ Created .cleanserve/pages/ directory");

    let has_app = Path::new("composer.json").exists()
        || framework.framework != Framework::Unknown
        || Path::new("public/index.php").exists();

    let public_dir = Path::new("public");
    if !public_dir.exists() && !has_app {
        std::fs::create_dir_all(public_dir).context("Failed to create public/ directory")?;

        if quickstart {
            html_pages::write_quickstart_pages(&cleanserve_pages_dir, &project_name)
                .context("Failed to write quickstart pages")?;
        } else {
            html_pages::write_default_pages(&cleanserve_pages_dir, &project_name, &php)
                .context("Failed to write default pages")?;
        }

        println!("✓ Created public/ directory with HTML pages");
    }

    for result in ensure_writable_dirs(&framework) {
        if let Err(e) = result {
            println!("⚠️ Could not prepare framework directory: {}", e);
        }
    }

    let gitignore_path = Path::new(".gitignore");
    let cleanserve_ignore = ".cleanserve/";
    if gitignore_path.exists() {
        let content =
            std::fs::read_to_string(gitignore_path).context("Failed to read .gitignore")?;
        if !content.contains(cleanserve_ignore) {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(gitignore_path)
                .context("Failed to open .gitignore")?;
            use std::io::Write;
            writeln!(f, "\n# CleanServe (standalone PHP runtime)")?;
            writeln!(f, "{}", cleanserve_ignore)?;
            println!("✓ Added .cleanserve/ to .gitignore");
        }
    } else {
        std::fs::write(
            gitignore_path,
            format!("# CleanServe\n{}\n", cleanserve_ignore),
        )
        .context("Failed to create .gitignore")?;
        println!("✓ Created .gitignore with .cleanserve/");
    }

    println!();
    println!("Next steps:");
    if has_app {
        println!("  1. Run 'cleanserve composer install' if dependencies are missing");
        println!("  2. Run 'cleanserve up' to start the server");
    } else {
        println!("  1. Edit public/index.php with your application code");
        println!("  2. Run 'cleanserve up' to start the server");
    }
    println!("  3. Open http://localhost:8080 in your browser");

    Ok(())
}

struct ComposerHints {
    name: Option<String>,
    php: Option<String>,
}

fn composer_hints() -> ComposerHints {
    let path = Path::new("composer.json");
    let Ok(content) = std::fs::read_to_string(path) else {
        return ComposerHints {
            name: None,
            php: None,
        };
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return ComposerHints {
            name: None,
            php: None,
        };
    };

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.rsplit('/').next().unwrap_or(s).to_string());

    let php = json
        .get("require")
        .and_then(|r| r.get("php"))
        .and_then(|v| v.as_str())
        .and_then(php_constraint_to_version);

    ComposerHints { name, php }
}

/// Pull a usable X.Y version from a Composer PHP constraint (`^8.2`, `>=8.3.0`, `~8.4.0`).
fn php_constraint_to_version(constraint: &str) -> Option<String> {
    let mut best: Option<(u32, u32)> = None;
    let bytes = constraint.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let token = &constraint[start..i];
            let mut parts = token.split('.');
            if let (Some(major), Some(minor)) = (parts.next(), parts.next()) {
                if let (Ok(maj), Ok(min)) = (major.parse::<u32>(), minor.parse::<u32>()) {
                    best = Some(match best {
                        Some(current) if current >= (maj, min) => current,
                        _ => (maj, min),
                    });
                }
            }
        } else {
            i += 1;
        }
    }

    let (major, minor) = best?;
    let detected = format!("{}.{}", major, minor);
    // Prefer current default when it still satisfies a lower floor (e.g. ^8.2 -> 8.4).
    if major == 8 && minor <= 4 {
        Some("8.4".to_string())
    } else {
        Some(detected)
    }
}

fn document_root_for(info: &FrameworkInfo) -> Option<String> {
    let parent = info.entry_point.parent()?;
    let root = if parent.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        parent.to_path_buf()
    };
    let mut root = root.to_string_lossy().into_owned();
    if root != "." && !root.ends_with('/') && !root.ends_with('\\') {
        root.push('/');
    }
    Some(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn php_constraint_prefers_8_4_when_compatible() {
        assert_eq!(php_constraint_to_version("^8.2").as_deref(), Some("8.4"));
        assert_eq!(php_constraint_to_version(">=8.3.0").as_deref(), Some("8.4"));
        assert_eq!(php_constraint_to_version("^8.5").as_deref(), Some("8.5"));
        assert_eq!(php_constraint_to_version("~8.4.0").as_deref(), Some("8.4"));
    }
}
