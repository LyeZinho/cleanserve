use super::DockerError;
use std::path::Path;
use std::process::Command;

pub fn docker_available() -> bool {
    Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn ensure_docker() -> Result<(), DockerError> {
    if docker_available() {
        Ok(())
    } else {
        Err(DockerError::NotAvailable)
    }
}

pub fn compose_up(compose_file: &Path, project_dir: &Path) -> Result<(), DockerError> {
    let status = compose_command(compose_file, project_dir)
        .args(["up", "-d"])
        .status()
        .map_err(|e| DockerError::Command(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(DockerError::Command(format!(
            "docker compose up failed (exit {:?})",
            status.code()
        )))
    }
}

pub fn compose_down(compose_file: &Path, project_dir: &Path, prune: bool) -> Result<(), DockerError> {
    let mut cmd = compose_command(compose_file, project_dir);
    cmd.args(["down", "--remove-orphans"]);
    if prune {
        cmd.args(["-v", "--rmi", "local"]);
    }
    let status = cmd
        .status()
        .map_err(|e| DockerError::Command(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(DockerError::Command(format!(
            "docker compose down failed (exit {:?})",
            status.code()
        )))
    }
}

pub fn compose_ps(compose_file: &Path, project_dir: &Path) -> Result<(), DockerError> {
    let status = compose_command(compose_file, project_dir)
        .args(["ps"])
        .status()
        .map_err(|e| DockerError::Command(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(DockerError::Command("docker compose ps failed".into()))
    }
}

fn compose_command(compose_file: &Path, project_dir: &Path) -> Command {
    let cmd = if Command::new("docker")
        .args(["compose", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        let mut c = Command::new("docker");
        c.args(["compose", "-f"]);
        c.arg(compose_file);
        c.args([
            "--project-directory",
            project_dir.to_str().unwrap_or("."),
        ]);
        c
    } else {
        let mut c = Command::new("docker-compose");
        c.args(["-f"]);
        c.arg(compose_file);
        c.current_dir(project_dir);
        c
    };
    cmd
}
