use crate::php_capabilities::is_composer_capable;
use crate::php_manager::{find_php_binary, PhpManager};
use crate::{cleanserve_home, CleanServeError, Result, VersionManifest};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

const PHP_RUNTIMES_REPO: &str = "https://github.com/LyeZinho/php-runtimes.git";

/// Extra configure flags applied to upstream php-runtimes before a local build.
const COMPOSER_CONFIGURE_INSERT: &str = r#"        "--enable-phar",
        "--enable-tokenizer",
        "--enable-ctype",
        "--enable-filter",
        "--enable-xml",
        "--with-libxml",
        "--enable-dom",
        "--enable-simplexml",
        "--enable-xmlwriter",
        "--enable-xmlreader",
        "--enable-session",
        "--enable-posix",
        "--with-zip",
        "--enable-opcache",
"#;

/// Flags that must exist in php-runtimes `get_configure_flags` for Yii/Laravel/Symfony.
const COMPOSER_BUILD_FLAGS: &[&str] = &[
    "--enable-phar",
    "--enable-tokenizer",
    "--enable-ctype",
    "--enable-filter",
    "--enable-xml",
    "--with-libxml",
    "--enable-dom",
    "--enable-simplexml",
    "--enable-xmlwriter",
    "--enable-xmlreader",
    "--enable-session",
    "--enable-posix",
    "--with-zip",
    "--enable-opcache",
];

pub fn discover_php_runtimes_repo() -> Option<PathBuf> {
    if let Ok(src) = std::env::var("CLEANSERVE_PHP_RUNTIMES_SRC") {
        let path = PathBuf::from(src);
        if is_php_runtimes_repo(&path) {
            return Some(path);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            let candidate = dir.join("php-runtimes");
            if is_php_runtimes_repo(&candidate) {
                return Some(candidate);
            }
            // ~/proj/my-app → ~/proj/cleanserve/php-runtimes
            let sibling = dir.join("cleanserve").join("php-runtimes");
            if is_php_runtimes_repo(&sibling) {
                return Some(sibling);
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors() {
            let candidate = dir.join("php-runtimes");
            if is_php_runtimes_repo(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

fn is_php_runtimes_repo(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("src/curator/build.rs").is_file()
}

fn default_clone_dir() -> Result<PathBuf> {
    Ok(cleanserve_home()?.join("src").join("php-runtimes"))
}

pub fn ensure_php_runtimes_repo() -> Result<PathBuf> {
    if let Some(repo) = discover_php_runtimes_repo() {
        return Ok(repo);
    }

    let dest = default_clone_dir()?;
    if is_php_runtimes_repo(&dest) {
        return Ok(dest);
    }

    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| {
            CleanServeError::Config(format!("Cannot reset {}: {}", dest.display(), e))
        })?;
    }

    std::fs::create_dir_all(dest.parent().unwrap_or(Path::new("."))).map_err(|e| {
        CleanServeError::Config(format!("Cannot create clone parent: {}", e))
    })?;

    info!("Cloning php-runtimes into {}", dest.display());
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            "main",
            PHP_RUNTIMES_REPO,
            dest.to_str().unwrap_or_default(),
        ])
        .status()
        .map_err(|e| CleanServeError::Config(format!("git clone failed: {}", e)))?;

    if !status.success() {
        return Err(CleanServeError::Config(
            "Failed to clone php-runtimes. Install git or set CLEANSERVE_PHP_RUNTIMES_SRC.".into(),
        ));
    }

    Ok(dest)
}

pub fn patch_build_for_composer(repo: &Path) -> Result<()> {
    let build_rs = repo.join("src/curator/build.rs");
    let mut content = std::fs::read_to_string(&build_rs).map_err(|e| {
        CleanServeError::Config(format!("Cannot read {}: {}", build_rs.display(), e))
    })?;

    let missing: Vec<&str> = COMPOSER_BUILD_FLAGS
        .iter()
        .copied()
        .filter(|flag| !content.contains(*flag))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    if !content.contains("--enable-phar") {
        let needle = "\"--enable-cli\",";
        let Some(pos) = content.find(needle) else {
            return Err(CleanServeError::Config(
                "php-runtimes build.rs format changed; cannot inject Composer extensions".into(),
            ));
        };
        let insert_at = pos + needle.len();
        let mut patched = String::with_capacity(content.len() + COMPOSER_CONFIGURE_INSERT.len() + 2);
        patched.push_str(&content[..insert_at]);
        patched.push('\n');
        patched.push_str(COMPOSER_CONFIGURE_INSERT);
        patched.push_str(&content[insert_at..]);
        content = patched;
    } else {
        for flag in missing {
            content = insert_configure_flag(&content, flag)?;
        }
    }

    std::fs::write(&build_rs, content).map_err(|e| {
        CleanServeError::Config(format!("Cannot patch {}: {}", build_rs.display(), e))
    })?;

    Ok(())
}

fn insert_configure_flag(content: &str, flag: &str) -> Result<String> {
    let line = format!("\n        \"{}\",", flag);
    let anchors = ["\"--with-libxml\",", "\"--enable-xml\",", "\"--enable-cli\","];
    for anchor in anchors {
        if let Some(pos) = content.find(anchor) {
            let insert_at = pos + anchor.len();
            let mut patched = String::with_capacity(content.len() + line.len());
            patched.push_str(&content[..insert_at]);
            patched.push_str(&line);
            patched.push_str(&content[insert_at..]);
            return Ok(patched);
        }
    }
    Err(CleanServeError::Config(format!(
        "Cannot insert {} into php-runtimes build.rs",
        flag
    )))
}

pub fn install_built_binary(src: &Path, base_dir: &Path, version: &str) -> Result<PathBuf> {
    let install_path = base_dir.join(format!("php-{}", version));
    let dest_dir = install_path.join("linux-x64");
    std::fs::create_dir_all(&dest_dir).map_err(|e| {
        CleanServeError::Config(format!("Cannot create {}: {}", dest_dir.display(), e))
    })?;

    let dest = dest_dir.join(format!("php-{}", version));
    std::fs::copy(src, &dest).map_err(|e| {
        CleanServeError::Config(format!(
            "Cannot install PHP binary to {}: {}",
            dest.display(),
            e
        ))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
        let stable = install_path.join("php");
        if !stable.exists() {
            let _ = std::os::unix::fs::symlink(&dest, &stable);
        }
    }

    find_php_binary(&install_path).ok_or_else(|| {
        CleanServeError::Config(format!(
            "Installed PHP {} but binary was not found under {}",
            version,
            install_path.display()
        ))
    })
}

pub async fn build_and_install(version_query: &str, base_dir: &Path) -> Result<PathBuf> {
    let manifest = VersionManifest::fetch(true).await?;
    let php_version = manifest
        .find_version(version_query)
        .ok_or_else(|| {
            CleanServeError::Download(format!(
                "PHP {} not found in manifest for local build",
                version_query
            ))
        })?;
    let resolved = php_version.version.clone();

    let repo = ensure_php_runtimes_repo()?;
    patch_build_for_composer(&repo)?;

    let platform = crate::php_downloader::current_platform();
    let built = repo
        .join("builds")
        .join(platform)
        .join(format!("php-{}", resolved));

    if built.is_file() && is_composer_capable(&built) {
        return install_built_binary(&built, base_dir, &resolved);
    }

    println!(
        "🔨 Building PHP {} locally (Composer extensions). This can take several minutes…",
        resolved
    );
    println!("   Source: {}", repo.display());

    let manifest = repo.join("Cargo.toml");
    let cargo_target = cleanserve_home()?
        .join("cache")
        .join("php-curator-target");
    std::fs::create_dir_all(&cargo_target).ok();

    let status = Command::new("cargo")
        .current_dir(&repo)
        .env("CARGO_TARGET_DIR", &cargo_target)
        .env("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "gcc")
        .args([
            "run",
            "--release",
            "--manifest-path",
            manifest.to_str().unwrap_or_default(),
            "--",
            "build",
            "--version",
            &resolved,
            "--platform",
            platform,
            "--path",
            ".",
        ])
        .status()
        .map_err(|e| CleanServeError::Config(format!("Failed to run php-curator build: {}", e)))?;

    if !status.success() {
        return Err(CleanServeError::Config(
            "Local PHP build failed. Install build dependencies (gcc, libxml2, openssl, …) or set CLEANSERVE_COMPOSER_PHP=/usr/bin/php.".into(),
        ));
    }

    if !built.is_file() {
        return Err(CleanServeError::Config(format!(
            "Build finished but binary missing at {}",
            built.display()
        )));
    }

    if !is_composer_capable(&built) {
        return Err(CleanServeError::Config(
            "Locally built PHP still lacks Composer extensions".into(),
        ));
    }

    let installed = install_built_binary(&built, base_dir, &resolved)?;
    println!("✅ Installed Composer-capable PHP {} at {}", resolved, installed.display());
    Ok(installed)
}

pub fn remove_matching_installs(version_query: &str) -> Result<()> {
    PhpManager::new()?.remove_matching_installs(version_query)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every configure flag the patch injects must also be part of the
    /// capability contract, otherwise a patched build can still be rejected.
    #[test]
    fn configure_insert_matches_required_build_flags() {
        for flag in COMPOSER_BUILD_FLAGS {
            assert!(
                COMPOSER_CONFIGURE_INSERT.contains(flag),
                "{} missing from COMPOSER_CONFIGURE_INSERT",
                flag
            );
        }
    }

    #[test]
    fn patch_injects_xmlwriter_and_xmlreader_into_build_rs() {
        let repo = std::env::temp_dir().join(format!("cleanserve-php-patch-test-{}", std::process::id()));
        let curator = repo.join("src").join("curator");
        let build_rs = curator.join("build.rs");
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&curator).unwrap();
        std::fs::write(
            &build_rs,
            "fn get_configure_flags() -> Vec<&'static str> {\n    vec![\n        \"--disable-all\",\n        \"--enable-cli\",\n    ]\n}\n",
        )
        .unwrap();

        patch_build_for_composer(&repo).unwrap();
        let patched = std::fs::read_to_string(&build_rs).unwrap();
        assert!(patched.contains("\"--enable-phar\","));
        assert!(patched.contains("\"--enable-xmlwriter\","));
        assert!(patched.contains("\"--enable-xmlreader\","));

        // Re-running the patch must be a no-op.
        patch_build_for_composer(&repo).unwrap();
        let again = std::fs::read_to_string(&build_rs).unwrap();
        assert_eq!(patched, again);

        let _ = std::fs::remove_dir_all(&repo);
    }
}
