use anyhow::Context;
use std::path::Path;

pub mod html_pages;

pub async fn run(name: Option<String>, php: String, quickstart: bool) -> anyhow::Result<()> {
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "my-project".to_string())
    });
    
    let config_path = Path::new("cleanserve.json");
    if config_path.exists() {
        anyhow::bail!("cleanserve.json already exists. Remove it first.");
    }
    
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let layout = cleanserve_core::ProjectLayout::detect(&project_root);
    let php_version = layout.suggested_php.clone().unwrap_or_else(|| php.clone());

    let mut server = cleanserve_core::ServerConfig::default();
    if Path::new("composer.json").exists() {
        server.root = layout.document_root_for_config(&project_root);
        println!(
            "✓ Detected {} — document root: {}",
            layout.framework,
            server.root
        );
    }

    let server_port = server.port;

    let config = cleanserve_core::CleanServeConfig {
        name: project_name.clone(),
        engine: cleanserve_core::EngineConfig {
            php: php_version.clone(),
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server,
        packages: None,
    };
    
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;

    println!("✓ Created cleanserve.json");

    let cleanserve_dir = Path::new(".cleanserve");
    let cleanserve_pages_dir = cleanserve_dir.join("pages");
    std::fs::create_dir_all(&cleanserve_pages_dir)
        .context("Failed to create .cleanserve/pages/ directory")?;
    
    println!("✓ Created .cleanserve/pages/ directory");

    let skip_public_scaffold = layout.framework == cleanserve_core::framework::Framework::Yii2
        || Path::new("composer.json").exists();

    let public_dir = Path::new("public");
    if !skip_public_scaffold && !public_dir.exists() {
        std::fs::create_dir_all(public_dir)
            .context("Failed to create public/ directory")?;
        
        // Generate HTML pages based on quickstart flag
        if quickstart {
            html_pages::write_quickstart_pages(&cleanserve_pages_dir, &project_name)
                .context("Failed to write quickstart pages")?;
        } else {
            html_pages::write_default_pages(&cleanserve_pages_dir, &project_name, &php_version)
                .context("Failed to write default pages")?;
        }
        
        println!("✓ Created public/ directory with HTML pages");
    }

    let gitignore_path = Path::new(".gitignore");
    let cleanserve_ignore = ".cleanserve/";
    if gitignore_path.exists() {
        let content = std::fs::read_to_string(gitignore_path)
            .context("Failed to read .gitignore")?;
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
        std::fs::write(gitignore_path, format!("# CleanServe\n{}\n", cleanserve_ignore))
            .context("Failed to create .gitignore")?;
        println!("✓ Created .gitignore with .cleanserve/");
    }

    println!();
    println!("Next steps:");
    if layout.framework == cleanserve_core::framework::Framework::Yii2 {
        println!("  1. Run `cleanserve php init` (Yii environment setup)");
        println!("  2. Run `cleanserve composer install`");
        println!("  3. Run `cleanserve up` and open https://localhost:{}", server_port);
    } else {
        println!("  1. Edit {}/index.php with your application code", config.server.root.trim_end_matches('/'));
        println!("  2. Run `cleanserve up` to start the server");
        println!("  3. Open https://localhost:{}", server_port);
    }
    println!();
    println!("Tip: use the binary from this repo: cargo build --release && ./target/release/cleanserve");
    
    Ok(())
}
