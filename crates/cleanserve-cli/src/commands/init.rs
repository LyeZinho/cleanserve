use anyhow::Context;
use std::path::Path;

pub async fn run(name: Option<String>, php: String) -> anyhow::Result<()> {
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
    
    let config = cleanserve_core::CleanServeConfig {
        name: project_name,
        engine: cleanserve_core::EngineConfig {
            php,
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server: cleanserve_core::ServerConfig::default(),
    };
    
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;

    println!("✓ Created cleanserve.json");

    let public_dir = Path::new("public");
    if !public_dir.exists() {
        std::fs::create_dir_all(public_dir)
            .context("Failed to create public/ directory")?;
        
        let index_php = public_dir.join("index.php");
        std::fs::write(&index_php, r#"<?php
/**
 * CleanServe - Zero Config PHP Development Server
 * This is your application entry point.
 */

echo "Hello from CleanServe!\n";
phpinfo();
"#)
            .context("Failed to create public/index.php")?;
        
        println!("✓ Created public/ directory with index.php");
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
    println!("  1. Edit public/index.php with your application code");
    println!("  2. Run 'cleanserve up' to start the server");
    println!("  3. Open http://localhost:8080 in your browser");
    
    Ok(())
}
