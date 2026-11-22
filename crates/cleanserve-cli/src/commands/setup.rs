use anyhow::Context;
use cleanserve_core::php_runtime::{missing_modules, shared_runtimes_dir, DEV_BASELINE_MODULES};
use cleanserve_core::{docker_available, validate_project_php, ComposerTool, PhpDownloader};
use std::process::Command;

/// Embedded at compile time: a released binary has no `scripts/` directory next to it.
const INSTALL_DOCKER_SH: &str = include_str!("../../../../scripts/install-docker.sh");

/// Report on the setup. Only things CleanServe truly needs are errors; Docker is optional.
pub fn run() -> anyhow::Result<()> {
    println!("CleanServe doctor\n");
    println!("✓ cleanserve {}", env!("CARGO_PKG_VERSION"));

    let mut usable_php = None;
    if let Ok(dir) = shared_runtimes_dir() {
        let mut versions: Vec<String> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter_map(|n| n.strip_prefix("php-").map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        versions.sort();

        match versions.last() {
            None => println!("• PHP: none installed yet (downloaded automatically on `cleanserve up`, or run `cleanserve update --version 8.4`)"),
            Some(v) => {
                let exe = PhpDownloader::new(&dir).ok().and_then(|d| d.get_php_exe(v));
                match exe {
                    None => println!("✗ PHP {}: binary not found in {}", v, dir.display()),
                    // Same resolution `cleanserve up` uses, including the full-runtime fallback.
                    Some(exe) => match validate_project_php(&exe, v) {
                        Ok(path) => {
                            println!("✓ PHP {} ({})", v, path.display());
                            usable_php = Some(path);
                        }
                        Err(_) => {
                            let missing = missing_modules(&exe, DEV_BASELINE_MODULES);
                            println!(
                                "✗ PHP {} is a minimal build: missing {}",
                                v,
                                missing.join(", ")
                            );
                            println!(
                                "  Place a full runtime in ~/.cleanserve/bin/php-{}/ (or rebuild the runtime with these extensions).",
                                v.split('.').take(2).collect::<Vec<_>>().join(".")
                            );
                        }
                    },
                }
            }
        }
    }

    match usable_php.as_deref().and_then(ComposerTool::detect_backend) {
        Some(backend) => println!("✓ Composer: {}", backend.describe()),
        None if usable_php.is_some() => println!(
            "✗ Composer unavailable (PHP has no phar, no `composer` on PATH, Docker not running)"
        ),
        None => println!("• Composer: checked once a usable PHP is available"),
    }

    if docker_available() {
        println!("✓ Docker is running (optional: stack services and Composer fallback)");
    } else {
        println!("• Docker not available (optional: only needed for `cleanserve stack` and as a Composer fallback)");
        println!("  Install it with: cleanserve setup docker");
    }

    Ok(())
}

pub fn run_setup_docker() -> anyhow::Result<()> {
    if docker_available() {
        println!("✓ Docker is already installed and running");
        return Ok(());
    }

    println!("Running Docker installer (may ask for your sudo password)...");
    // Run from a file, not via stdin: the installer's own commands may read stdin (prompts).
    let dir = tempfile::tempdir().context("Cannot create temporary directory")?;
    let script = dir.path().join("install-docker.sh");
    std::fs::write(&script, INSTALL_DOCKER_SH).context("Cannot write installer script")?;
    let status = Command::new("sh")
        .arg(&script)
        .status()
        .context("Failed to run the Docker installer")?;

    if !status.success() {
        anyhow::bail!(
            "Docker installation script failed. Install Docker manually: https://docs.docker.com/engine/install/"
        );
    }

    if docker_available() {
        println!("✓ Docker is ready");
    } else {
        println!("Docker was installed but the daemon is not reachable.");
        println!("Log out/in (docker group) or start the service, then run: cleanserve doctor");
    }

    Ok(())
}
