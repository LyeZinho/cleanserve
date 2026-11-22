use crate::find_available_port;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};
use tracing::info;

/// First port tried for the PHP built-in server.
pub const DEFAULT_PHP_PORT: u16 = 9000;
const PORT_SCAN: u16 = 200;

const READY_TIMEOUT: Duration = Duration::from_secs(10);
const READY_POLL: Duration = Duration::from_millis(50);

pub struct PhpWorker {
    php_path: PathBuf,
    root: PathBuf,
    router: Option<PathBuf>,
    php_ini: Option<PathBuf>,
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
            php_ini: None,
            log_path: None,
            port: DEFAULT_PHP_PORT,
            child: None,
        }
    }

    /// Use `router` as the front controller script of the built-in server.
    pub fn with_router(php_path: PathBuf, root: PathBuf, router: PathBuf) -> Self {
        let mut worker = Self::new(php_path, root);
        worker.router = Some(router);
        worker
    }

    /// Run PHP with this `php.ini` (`php -c`).
    pub fn with_php_ini(mut self, ini_path: PathBuf) -> Self {
        self.php_ini = Some(ini_path);
        self
    }

    /// Append the PHP built-in server output (request log, errors) to this file.
    ///
    /// Without a log file the output is discarded. It is never piped back to us:
    /// an unread pipe fills up (~64 KiB) and blocks the PHP server.
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_path = Some(path);
        self
    }

    /// Port the PHP built-in server listens on (valid after a successful start).
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.start_on_port(DEFAULT_PHP_PORT)?;
        Ok(())
    }

    /// Start the built-in server on `preferred_port`, or the next free port. Returns the port used.
    pub fn start_on_port(&mut self, preferred_port: u16) -> anyhow::Result<u16> {
        self.stop();

        let port = find_available_port(preferred_port, PORT_SCAN).ok_or_else(|| {
            anyhow::anyhow!(
                "No free TCP port found for PHP worker (tried {}..{})",
                preferred_port,
                preferred_port.saturating_add(PORT_SCAN - 1)
            )
        })?;
        if port != preferred_port {
            info!("PHP port {} in use, using {} for built-in server", preferred_port, port);
        }

        info!("🔄 Starting PHP worker: {} on 127.0.0.1:{}", self.php_path.display(), port);

        let mut cmd = std::process::Command::new(&self.php_path);
        if let Some(ini) = &self.php_ini {
            cmd.arg("-c").arg(ini);
        }
        cmd.args([
            "-S",
            &format!("127.0.0.1:{}", port),
            "-t",
            self.root.to_str().unwrap_or("."),
        ]);
        if self.php_ini.is_none() {
            // A generated php.ini already sets these; without one, keep the same behaviour.
            cmd.args(["-d", "variables_order=EGPCS", "-d", "cgi.fix_pathinfo=1"]);
        }
        if let Some(router) = &self.router {
            cmd.arg(router);
            info!("Using front-controller router: {}", router.display());
        }

        let stderr = match &self.log_path {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
                Stdio::from(file)
            }
            None => Stdio::null(),
        };

        let child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()?;

        self.port = port;
        self.child = Some(child);

        if let Err(e) = self.wait_until_ready() {
            self.stop();
            return Err(e);
        }

        info!("✅ PHP worker started on port {}", port);
        Ok(port)
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
                // Something answers, but is it *our* server? If another process grabbed the port
                // after we picked it, PHP exits at once with "address in use" while the connect
                // above succeeded against the other process. Give it a moment, then confirm
                // our child is still alive.
                std::thread::sleep(Duration::from_millis(40));
                if let Some(child) = self.child.as_mut() {
                    if let Some(status) = child.try_wait()? {
                        anyhow::bail!(
                            "PHP server exited right after startup ({}); port {} may have been taken by another process.{}",
                            status,
                            self.port,
                            log_hint
                        );
                    }
                }
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


#[cfg(test)]
mod tests {
    use super::*;

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
