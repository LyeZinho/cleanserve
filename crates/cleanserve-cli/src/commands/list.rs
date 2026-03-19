use anyhow::Context;

pub async fn run() -> anyhow::Result<()> {
    let manager = cleanserve_core::PhpManager::new()
        .context("Failed to initialize PHP manager")?;
    
    let installed = manager.list_installed();
    
    if installed.is_empty() {
        println!("No PHP versions installed.");
        println!("Run 'cleanserve update --version 8.4' to install PHP.");
    } else {
        println!("Installed PHP versions:");
        for version in installed {
            println!("  • {}", version);
        }
    }
    
    Ok(())
}
