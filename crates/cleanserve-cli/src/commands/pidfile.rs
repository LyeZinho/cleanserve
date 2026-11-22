//! PID file handling so `cleanserve down` can stop the server started by `cleanserve up`.

use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn pid_path(project_root: &Path) -> PathBuf {
    project_root.join(".cleanserve").join("cleanserve.pid")
}

pub fn read_pid(project_root: &Path) -> Option<u32> {
    std::fs::read_to_string(pid_path(project_root))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// True when `pid` is alive *and* looks like a cleanserve process, so a stale PID file
/// whose number was recycled by the OS never makes us signal an unrelated program.
pub fn is_cleanserve_process(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .stderr(Stdio::null())
            .output()
            .map(|o| {
                o.status.success() && String::from_utf8_lossy(&o.stdout).contains("cleanserve")
            })
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("cleanserve"))
            .unwrap_or(false)
    }
}

fn signal(pid: u32, force: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    let status = Command::new("kill")
        .arg(if force { "-KILL" } else { "-TERM" })
        .arg(pid.to_string())
        .stderr(Stdio::null())
        .status()?;
    #[cfg(windows)]
    let status = {
        let _ = force;
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
    };
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("failed to signal process {}", pid)))
    }
}

/// Ask `pid` to terminate, escalating to a forced kill after `grace`.
/// Returns true if the process is gone.
pub fn terminate(pid: u32, grace: Duration) -> std::io::Result<bool> {
    signal(pid, false)?;
    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        if !is_cleanserve_process(pid) {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    signal(pid, true)?;
    std::thread::sleep(Duration::from_millis(200));
    Ok(!is_cleanserve_process(pid))
}

/// Owns the PID file for the lifetime of a running server.
pub struct PidGuard {
    path: PathBuf,
    pid: u32,
}

impl PidGuard {
    /// Claim the PID file. Fails if another cleanserve server already runs in this project.
    pub fn acquire(project_root: &Path) -> anyhow::Result<Self> {
        if let Some(existing) = read_pid(project_root) {
            if existing != std::process::id() && is_cleanserve_process(existing) {
                anyhow::bail!(
                    "CleanServe is already running in this project (pid {}). Run `cleanserve down` first.",
                    existing
                );
            }
        }
        let path = pid_path(project_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pid = std::process::id();
        std::fs::write(&path, pid.to_string()).context("Failed to write PID file")?;
        Ok(Self { path, pid })
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        // Only remove the file if it is still ours.
        if std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            == Some(self.pid)
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_writes_and_drop_removes_pid_file() {
        let dir = tempfile_dir();
        {
            let _guard = PidGuard::acquire(&dir).unwrap();
            assert_eq!(read_pid(&dir), Some(std::process::id()));
        }
        assert_eq!(read_pid(&dir), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_pid_file_is_overwritten() {
        let dir = tempfile_dir();
        std::fs::create_dir_all(dir.join(".cleanserve")).unwrap();
        // u32::MAX is never a live process.
        std::fs::write(pid_path(&dir), u32::MAX.to_string()).unwrap();
        let guard = PidGuard::acquire(&dir).unwrap();
        assert_eq!(read_pid(&dir), Some(std::process::id()));
        drop(guard);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unrelated_process_is_not_cleanserve() {
        // The test binary is named `cleanserve-<hash>`, so use a PID that cannot exist.
        assert!(!is_cleanserve_process(u32::MAX));
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cleanserve-pid-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
