use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use tracing::info;

pub struct PhpWorker {
    php_path: PathBuf,
    root: PathBuf,
    router: Option<PathBuf>,
    child: Option<Child>,
}

impl PhpWorker {
    pub fn new(php_path: PathBuf, root: PathBuf) -> Self {
        Self {
            php_path,
            root,
            router: None,
            child: None,
        }
    }

    pub fn with_router(php_path: PathBuf, root: PathBuf, router: PathBuf) -> Self {
        Self {
            php_path,
            root,
            router: Some(router),
            child: None,
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let router = self.router.clone();
        self.start_with_router(router.as_deref())
    }

    pub fn start_with_router(&mut self, router: Option<&Path>) -> anyhow::Result<()> {
        self.stop();
        
        info!("🔄 Starting PHP worker: {}", self.php_path.display());
        
        let mut cmd = std::process::Command::new(&self.php_path);
        cmd.args([
                "-S", "127.0.0.1:9000",
                "-t", self.root.to_str().unwrap_or("."),
                "-d", "variables_order=EGPCS",
                "-d", "cgi.fix_pathinfo=1",
            ]);

        if let Some(router) = router {
            cmd.arg(router);
            info!("Using front-controller router: {}", router.display());
        }

        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        self.child = Some(child);
        std::thread::sleep(std::time::Duration::from_millis(200));
        info!("✅ PHP worker started on port 9000");
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
