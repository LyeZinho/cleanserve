use crate::find_available_port;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use tracing::info;

const DEFAULT_PHP_PORT: u16 = 9000;
const PORT_SCAN: u16 = 200;

pub struct PhpWorker {
    php_path: PathBuf,
    root: PathBuf,
    php_ini: Option<PathBuf>,
    port: u16,
    child: Option<Child>,
}

impl PhpWorker {
    pub fn new(php_path: PathBuf, root: PathBuf) -> Self {
        Self {
            php_path,
            root,
            php_ini: None,
            port: DEFAULT_PHP_PORT,
            child: None,
        }
    }

    pub fn with_php_ini(mut self, ini_path: PathBuf) -> Self {
        self.php_ini = Some(ini_path);
        self
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Start the built-in server, using `preferred_port` or the next free port.
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
            info!(
                "PHP port {} in use, using {} for built-in server",
                preferred_port,
                port
            );
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

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        std::thread::sleep(std::time::Duration::from_millis(250));

        if child.try_wait()?.is_some() {
            let mut stderr = String::new();
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_string(&mut stderr);
            }
            anyhow::bail!(
                "PHP worker exited immediately on port {}: {}",
                port,
                stderr.trim()
            );
        }

        self.port = port;
        self.child = Some(child);
        info!("✅ PHP worker started on port {}", port);
        Ok(port)
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.start_on_port(DEFAULT_PHP_PORT)?;
        Ok(())
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
