use anyhow::Context;
use cleanserve_core::{StackManager, ensure_docker};

pub async fn run_list() -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let stack = StackManager::new(&project_root).map_err(|e| anyhow::anyhow!(e))?;

    println!("Available stack services (install only what you need):\n");
    for (name, def) in stack.catalog() {
        println!("  {} — {} ({})", name, def.title, def.description);
    }
    println!("\nUsage: cleanserve stack add <name>");
    Ok(())
}

pub async fn run_add(name: &str) -> anyhow::Result<()> {
    ensure_docker().map_err(|e| anyhow::anyhow!(e))?;
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let stack = StackManager::new(&project_root).map_err(|e| anyhow::anyhow!(e))?;
    stack.add(name).map_err(|e| anyhow::anyhow!(e))?;
    println!("✓ Added stack service '{}'", name);
    println!("  Run: cleanserve stack up");
    Ok(())
}

pub async fn run_remove(name: &str) -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let stack = StackManager::new(&project_root).map_err(|e| anyhow::anyhow!(e))?;
    stack.remove(name).map_err(|e| anyhow::anyhow!(e))?;
    stack.stop().ok();
    println!("✓ Removed stack service '{}'", name);
    Ok(())
}

pub async fn run_up() -> anyhow::Result<()> {
    ensure_docker().map_err(|e| anyhow::anyhow!(e))?;
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let stack = StackManager::new(&project_root).map_err(|e| anyhow::anyhow!(e))?;
    stack.start().map_err(|e| anyhow::anyhow!(e))?;
    println!("✓ Stack services started");
    Ok(())
}

pub async fn run_down() -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let stack = StackManager::new(&project_root).map_err(|e| anyhow::anyhow!(e))?;
    stack.stop().map_err(|e| anyhow::anyhow!(e))?;
    println!("✓ Stack services stopped");
    Ok(())
}

pub async fn run_status() -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    let stack = StackManager::new(&project_root).map_err(|e| anyhow::anyhow!(e))?;
    stack.status().map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}
