//! Download and run Composer with the project PHP binary.

use crate::package_manager::cache::PackageCache;
use crate::{CleanServeError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::info;

const COMPOSER_VERSION: &str = "2.8";
const COMPOSER_DOWNLOAD_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar";
const COMPOSER_CHECKSUM_URL: &str =
    "https://getcomposer.org/download/latest-stable/composer.phar.sha256sum";

pub struct ComposerTool;

/// How Composer will be executed, in order of preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerBackend {
    /// `composer.phar` run by the project's PHP (right PHP version, no extra requirements).
    Phar,
    /// A `composer` executable found on `PATH` (uses the system PHP, which may differ).
    System(PathBuf),
    /// The official `composer:2` Docker image (uses the image's PHP, which may differ).
    Docker,
}

impl ComposerBackend {
    pub fn describe(&self) -> String {
        match self {
            ComposerBackend::Phar => "composer.phar with the project PHP".to_string(),
            ComposerBackend::System(p) => format!("system composer ({})", p.display()),
            ComposerBackend::Docker => "Docker image composer:2".to_string(),
        }
    }
}

/// Find an executable called `name` in `dirs` (the entries of `PATH`).
pub fn find_executable_in(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let candidates: Vec<String> = if cfg!(windows) {
        vec![format!("{name}.exe"), format!("{name}.bat"), format!("{name}.cmd")]
    } else {
        vec![name.to_string()]
    };
    dirs.iter()
        .flat_map(|d| candidates.iter().map(move |c| d.join(c)))
        .find(|p| p.is_file())
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    find_executable_in(name, &dirs)
}

/// Extract the hash from `sha256sum` output (`<hash>  composer.phar`).
fn parse_sha256(text: &str) -> Option<String> {
    let hash = text.split_whitespace().next()?;
    (hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| hash.to_ascii_lowercase())
}

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
                    .any(|line| line.trim().eq_ignore_ascii_case("phar"))
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

    /// Pick the best available way to run Composer, or `None` if there is none.
    pub fn detect_backend(php_bin: &Path) -> Option<ComposerBackend> {
        if Self::php_has_phar(php_bin) {
            return Some(ComposerBackend::Phar);
        }
        if let Some(path) = find_in_path("composer") {
            return Some(ComposerBackend::System(path));
        }
        if Self::docker_available() {
            return Some(ComposerBackend::Docker);
        }
        None
    }

    /// A Composer that does not need the project PHP: a `composer` on `PATH`, else the
    /// `composer:2` Docker image. Used when no available PHP has the extensions Composer needs.
    pub fn detect_external_backend() -> Option<ComposerBackend> {
        if let Some(path) = find_in_path("composer") {
            return Some(ComposerBackend::System(path));
        }
        if Self::docker_available() {
            return Some(ComposerBackend::Docker);
        }
        None
    }

    /// Run Composer through a backend from [`detect_external_backend`](Self::detect_external_backend).
    pub fn run_external(
        backend: &ComposerBackend,
        project_root: &Path,
        args: &[String],
    ) -> Result<i32> {
        match backend {
            ComposerBackend::System(path) => Self::run_via_system(path, project_root, args),
            ComposerBackend::Docker => Self::run_via_docker(project_root, args),
            ComposerBackend::Phar => Err(CleanServeError::Config(
                "the phar backend needs a project PHP".into(),
            )),
        }
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

        info!("Downloading Composer phar...");

        let bytes = download_bytes(COMPOSER_DOWNLOAD_URL).await?;
        let checksum = String::from_utf8_lossy(&download_bytes(COMPOSER_CHECKSUM_URL).await?)
            .to_string();
        let expected = parse_sha256(&checksum)
            .ok_or_else(|| CleanServeError::Download("Invalid Composer checksum file".into()))?;
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != expected {
            return Err(CleanServeError::Download(format!(
                "Composer download is corrupt or tampered with (sha256 {} != {})",
                actual, expected
            )));
        }

        // Write to a temp file and rename, so an interrupted download is never cached as valid.
        let tmp = dest.with_extension("phar.part");
        std::fs::write(&tmp, &bytes).map_err(|e| {
            CleanServeError::Download(format!("Failed to write {}: {}", tmp.display(), e))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        }

        std::fs::rename(&tmp, &dest).map_err(|e| {
            CleanServeError::Download(format!("Failed to install {}: {}", dest.display(), e))
        })?;

        Ok(dest)
    }

    pub async fn run(php_bin: &Path, project_root: &Path, args: &[String]) -> Result<i32> {
        match Self::detect_backend(php_bin) {
            Some(ComposerBackend::Phar) => Self::run_via_phar(php_bin, project_root, args).await,
            Some(ComposerBackend::System(path)) => {
                info!("Project PHP has no phar extension — using system composer");
                Self::run_via_system(&path, project_root, args)
            }
            Some(ComposerBackend::Docker) => {
                info!("Project PHP has no phar extension — running Composer via Docker (composer:2)");
                Self::run_via_docker(project_root, args)
            }
            None => Err(CleanServeError::Config(
                "Composer is not available: the project PHP has no phar extension, there is no \
`composer` on PATH and Docker is not running. Install Composer (https://getcomposer.org/download/) \
or Docker (`cleanserve setup docker`), or pick a PHP build that includes phar."
                    .into(),
            )),
        }
    }

    fn run_via_system(composer: &Path, project_root: &Path, args: &[String]) -> Result<i32> {
        let status = Command::new(composer)
            .args(args)
            .current_dir(project_root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| CleanServeError::Config(format!("Failed to run composer: {}", e)))?;
        Ok(status.code().unwrap_or(1))
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
        use std::io::IsTerminal;

        let root = project_root
            .canonicalize()
            .map_err(|e| CleanServeError::Config(format!("Invalid project path: {}", e)))?;

        // Keep Composer's cache/home inside the project so a non-root container user can write it
        // and downloads are reused between runs.
        let composer_home = root.join(".cleanserve").join("composer-home");
        std::fs::create_dir_all(&composer_home).map_err(|e| {
            CleanServeError::Config(format!("Cannot create composer home: {}", e))
        })?;

        let mut cmd = Command::new("docker");
        cmd.args(["run", "--rm", "-i"]);
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            cmd.arg("-t");
        }
        #[cfg(unix)]
        {
            let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
            cmd.args(["-u", &format!("{}:{}", uid, gid)]);
        }
        cmd.args([
            "-e",
            "COMPOSER_HOME=/composer-home",
            "-v",
            &format!("{}:/composer-home", composer_home.display()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sha256_accepts_sha256sum_output() {
        let h = "dd158c0afd41d15fac89e1b4e47b5d42911fe316c3687cd5edac98319b25d028";
        assert_eq!(parse_sha256(&format!("{h}  composer.phar\n")), Some(h.to_string()));
        assert_eq!(parse_sha256(&format!("{}\n", h.to_uppercase())), Some(h.to_string()));
        assert_eq!(parse_sha256("<html>not found</html>"), None);
        assert_eq!(parse_sha256(""), None);
    }

    #[cfg(unix)]
    #[test]
    fn find_executable_in_searches_given_dirs_in_order() {
        use std::os::unix::fs::PermissionsExt;
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let bin_b = b.path().join("composer");
        std::fs::write(&bin_b, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&bin_b, std::fs::Permissions::from_mode(0o755)).unwrap();

        let dirs = vec![a.path().to_path_buf(), b.path().to_path_buf()];
        assert_eq!(find_executable_in("composer", &dirs), Some(bin_b));
        assert_eq!(find_executable_in("missing-tool", &dirs), None);
    }

    #[test]
    fn backend_descriptions_name_the_tool() {
        assert!(ComposerBackend::Phar.describe().contains("phar"));
        assert!(ComposerBackend::Docker.describe().contains("Docker"));
        assert!(ComposerBackend::System(PathBuf::from("/usr/bin/composer"))
            .describe()
            .contains("/usr/bin/composer"));
    }
}
