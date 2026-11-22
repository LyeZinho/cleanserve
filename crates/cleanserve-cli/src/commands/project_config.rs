use anyhow::Context;
use std::path::Path;

/// Remove CleanServe from a project: its Docker stack, `cleanserve.json` and `.cleanserve/`.
pub fn uninstall_project(project_root: &Path) -> anyhow::Result<()> {
    if cleanserve_core::docker_available() {
        if let Err(e) = cleanserve_core::teardown_all_for_project(project_root, true) {
            eprintln!("Warning: stack cleanup: {}", e);
        } else {
            println!("✓ Removed Docker stack (containers, volumes, local images)");
        }
    }

    let config_path = project_root.join("cleanserve.json");
    let cleanserve_dir = project_root.join(".cleanserve");

    if config_path.exists() {
        std::fs::remove_file(&config_path).context("Failed to remove cleanserve.json")?;
        println!("✓ Removed cleanserve.json");
    }

    if cleanserve_dir.exists() {
        std::fs::remove_dir_all(&cleanserve_dir).context("Failed to remove .cleanserve/")?;
        println!("✓ Removed .cleanserve/");
    }

    if !config_path.exists() && !cleanserve_dir.exists() {
        println!("Nothing to uninstall (no cleanserve.json or .cleanserve/)");
    }

    Ok(())
}
