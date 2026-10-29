use cleanserve_core::{docker_available, ensure_docker};
use anyhow::Context;

pub fn run() -> anyhow::Result<()> {
    println!("CleanServe doctor\n");

    if docker_available() {
        println!("✓ Docker is available");
        ensure_docker()?;
    } else {
        println!("✗ Docker is not available (required for Composer, stack services, and optional PHP images)");
        println!("  Install: cleanserve setup docker");
        println!("  Or run: curl -fsSL https://raw.githubusercontent.com/LyeZinho/cleanserve/main/scripts/install-docker.sh | sh");
        anyhow::bail!("Docker prerequisite missing");
    }

    Ok(())
}

pub fn run_setup_docker() -> anyhow::Result<()> {
    if docker_available() {
        println!("✓ Docker is already installed and running");
        return Ok(());
    }

    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/install-docker.sh");
    if !std::path::Path::new(script).exists() {
        anyhow::bail!(
            "install-docker.sh not found. Install Docker manually: https://docs.docker.com/engine/install/"
        );
    }

    println!("Running Docker installer (may require sudo)...");
    let status = std::process::Command::new("sh")
        .arg(script)
        .status()
        .context("Failed to run install-docker.sh")?;

    if !status.success() {
        anyhow::bail!("Docker installation script failed");
    }

    if docker_available() {
        println!("✓ Docker is ready");
    } else {
        println!("Docker was installed but the daemon is not reachable.");
        println!("Add your user to the docker group and start the service, then re-run: cleanserve doctor");
    }

    Ok(())
}
