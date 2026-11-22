use super::pidfile;
use std::time::Duration;

pub async fn run() -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;

    let Some(pid) = pidfile::read_pid(&project_root) else {
        println!("No running CleanServe server found in this project.");
        return Ok(());
    };

    if !pidfile::is_cleanserve_process(pid) {
        // Stale file (crash / reboot): clean it up so `up` starts fresh.
        let _ = std::fs::remove_file(pidfile::pid_path(&project_root));
        println!("No running CleanServe server found (removed stale PID file for {}).", pid);
        return Ok(());
    }

    println!("🛑 Stopping CleanServe server (pid {})...", pid);
    let stopped = tokio::task::spawn_blocking(move || pidfile::terminate(pid, Duration::from_secs(5)))
        .await??;

    if !stopped {
        anyhow::bail!("Could not stop CleanServe server (pid {})", pid);
    }

    let _ = std::fs::remove_file(pidfile::pid_path(&project_root));
    println!("✅ Server stopped");
    Ok(())
}
