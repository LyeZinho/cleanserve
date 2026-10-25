//! Download and run Composer with the project PHP binary.

use crate::package_manager::cache::PackageCache;
use crate::{CleanServeError, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::info;

const COMPOSER_VERSION: &str = "2.8";
const COMPOSER_DOWNLOAD_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar";

pub struct ComposerTool;

impl ComposerTool {
    pub fn phar_path() -> Result<PathBuf> {
        PackageCache::package_path("composer", COMPOSER_VERSION)
            .map(|p| p.join("composer.phar"))
            .map_err(|e| CleanServeError::Config(e.message))
    }

    fn php_has_phar(php_bin: &Path) -> bool {
        Command::new(php_bin)
            .arg("-m")
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|line| line.trim() == "phar")
            })
            .unwrap_or(false)
    }

    fn docker_available() -> bool {
        Command::new("docker")
            .arg("info")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub async fn ensure_phar() -> Result<PathBuf> {
        let dest = Self::phar_path()?;
        if dest.exists() {
            return Ok(dest);
        }

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CleanServeError::Download(format!("Cannot create Composer cache dir: {}", e))
            })?;
        }

        info!("Downloading Composer {} phar...", COMPOSER_VERSION);

        let bytes = download_bytes(COMPOSER_DOWNLOAD_URL).await?;
        std::fs::write(&dest, &bytes).map_err(|e| {
            CleanServeError::Download(format!("Failed to write {}: {}", dest.display(), e))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms)?;
        }

        Ok(dest)
    }

    pub async fn run(php_bin: &Path, project_root: &Path, args: &[String]) -> Result<i32> {
        if Self::php_has_phar(php_bin) {
            return Self::run_via_phar(php_bin, project_root, args).await;
        }

        if Self::docker_available() {
            info!("Project PHP has no phar extension — running Composer via Docker (composer:2)");
            return Self::run_via_docker(project_root, args);
        }

        Err(CleanServeError::Config(
            "Composer needs the PHP phar extension, which the bundled PHP build does not include. \
Install Docker, or use a system PHP with phar for `cleanserve composer`."
                .into(),
        ))
    }

    async fn run_via_phar(
        php_bin: &Path,
        project_root: &Path,
        args: &[String],
    ) -> Result<i32> {
        let phar = Self::ensure_phar().await?;
        let composer_home = project_root.join(".cleanserve").join("composer-home");
        std::fs::create_dir_all(&composer_home).map_err(|e| {
            CleanServeError::Config(format!("Cannot create composer home: {}", e))
        })?;

        let status = Command::new(php_bin)
            .arg(&phar)
            .args(args)
            .current_dir(project_root)
            .env("COMPOSER_HOME", &composer_home)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| CleanServeError::Config(format!("Failed to run composer: {}", e)))?;

        Ok(status.code().unwrap_or(1))
    }

    fn run_via_docker(project_root: &Path, args: &[String]) -> Result<i32> {
        let root = project_root
            .canonicalize()
            .map_err(|e| CleanServeError::Config(format!("Invalid project path: {}", e)))?;

        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        let mut cmd = Command::new("docker");
        cmd.args([
            "run",
            "--rm",
            "-u",
            &format!("{}:{}", uid, gid),
            "-v",
            &format!("{}:/app", root.display()),
            "-w",
            "/app",
            "composer:2",
        ])
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

        let status = cmd
            .status()
            .map_err(|e| CleanServeError::Config(format!("Failed to run docker composer: {}", e)))?;

        Ok(status.code().unwrap_or(1))
    }

    pub async fn ensure_vendor(php_bin: &Path, project_root: &Path) -> Result<()> {
        let composer_json = project_root.join("composer.json");
        let vendor = project_root.join("vendor");
        if !composer_json.exists() || vendor.exists() {
            return Ok(());
        }

        info!("vendor/ missing — running composer install");
        let code = Self::run(php_bin, project_root, &["install".to_string()]).await?;
        if code != 0 {
            return Err(CleanServeError::Config(format!(
                "composer install failed with exit code {}",
                code
            )));
        }
        Ok(())
    }
}

async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| CleanServeError::Download(format!("HTTP client error: {}", e)))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| CleanServeError::Download(format!("Download failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(CleanServeError::Download(format!(
            "Download failed: HTTP {} for {}",
            response.status(),
            url
        )));
    }

    response
        .bytes()
        .await
        .map_err(|e| CleanServeError::Download(format!("Failed to read response: {}", e)))
        .map(|b| b.to_vec())
}
