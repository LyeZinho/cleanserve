use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};
use tracing::info;

/// Default upstream port used before a worker picks a free one.
pub const DEFAULT_PHP_PORT: u16 = 9000;

const READY_TIMEOUT: Duration = Duration::from_secs(10);
const READY_POLL: Duration = Duration::from_millis(50);

pub struct PhpWorker {
    php_path: PathBuf,
    root: PathBuf,
    router: Option<PathBuf>,
    log_path: Option<PathBuf>,
    port: u16,
    child: Option<Child>,
}

impl PhpWorker {
    pub fn new(php_path: PathBuf, root: PathBuf) -> Self {
        Self {
            php_path,
            root,
            router: None,
            log_path: None,
            port: 0,
            child: None,
        }
    }

    pub fn with_router(php_path: PathBuf, root: PathBuf, router: PathBuf) -> Self {
        let mut worker = Self::new(php_path, root);
        worker.router = Some(router);
        worker
    }

    /// Append the PHP built-in server output (request log, errors) to this file.
    ///
    /// Without a log file the output is discarded. It is never piped back to us:
    /// an unread pipe fills up (~64 KiB) and blocks the PHP server.
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_path = Some(path);
        self
    }

    /// Port the PHP built-in server listens on (valid after [`start`](Self::start)).
    pub fn port(&self) -> u16 {
        if self.port == 0 {
            DEFAULT_PHP_PORT
        } else {
            self.port
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let router = self.router.clone();
        self.start_with_router(router.as_deref())
    }

    pub fn start_with_router(&mut self, router: Option<&Path>) -> anyhow::Result<()> {
        self.stop();

        if self.port == 0 {
            self.port = pick_free_port()?;
        }
        let addr = format!("127.0.0.1:{}", self.port);

        info!("🔄 Starting PHP worker: {}", self.php_path.display());

        let mut cmd = std::process::Command::new(&self.php_path);
        cmd.args([
            "-S",
            addr.as_str(),
            "-t",
            self.root.to_str().unwrap_or("."),
            "-d",
            "variables_order=EGPCS",
            "-d",
            "cgi.fix_pathinfo=1",
        ]);

        if let Some(router) = router {
            cmd.arg(router);
            info!("Using front-controller router: {}", router.display());
        }

        let stderr = match &self.log_path {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                Stdio::from(file)
            }
            None => Stdio::null(),
        };

        let child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()?;

        self.child = Some(child);

        if let Err(e) = self.wait_until_ready() {
            self.stop();
            return Err(e);
        }

        info!("✅ PHP worker started on {}", addr);
        Ok(())
    }

    /// Block until the PHP server accepts connections, or fail if it exited / timed out.
    fn wait_until_ready(&mut self) -> anyhow::Result<()> {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        let deadline = Instant::now() + READY_TIMEOUT;
        let log_hint = self
            .log_path
            .as_ref()
            .map(|p| format!(" See {} for details.", p.display()))
            .unwrap_or_default();

        loop {
            if let Some(child) = self.child.as_mut() {
                if let Some(status) = child.try_wait()? {
                    anyhow::bail!(
                        "PHP server exited during startup ({}). Is the PHP binary valid?{}",
                        status,
                        log_hint
                    );
                }
            }

            if TcpStream::connect_timeout(&addr, READY_POLL).is_ok() {
                return Ok(());
            }

            if Instant::now() >= deadline {
                anyhow::bail!(
                    "PHP server did not become ready on {} within {}s.{}",
                    addr,
                    READY_TIMEOUT.as_secs(),
                    log_hint
                );
            }
            std::thread::sleep(READY_POLL);
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            info!("🛑 PHP worker stopped");
        }
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            child.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    }
}

impl Drop for PhpWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Ask the OS for a currently free loopback port.
fn pick_free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_free_port_returns_bindable_port() {
        let port = pick_free_port().unwrap();
        assert_ne!(port, 0);
        TcpListener::bind(("127.0.0.1", port)).expect("port should be free again");
    }

    #[test]
    fn port_defaults_before_start() {
        let w = PhpWorker::new(PathBuf::from("php"), PathBuf::from("."));
        assert_eq!(w.port(), DEFAULT_PHP_PORT);
    }

    #[cfg(unix)]
    #[test]
    fn start_fails_fast_when_binary_exits() {
        // `false` exits immediately: start must report an error, not hang or claim success.
        let mut w = PhpWorker::new(PathBuf::from("false"), PathBuf::from("."));
        let started = Instant::now();
        let err = w.start().expect_err("worker that exits must fail to start");
        assert!(err.to_string().contains("exited during startup"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!w.is_running());
    }

    #[test]
    fn start_fails_when_binary_missing() {
        let mut w = PhpWorker::new(PathBuf::from("/nonexistent/php"), PathBuf::from("."));
        assert!(w.start().is_err());
    }
}
