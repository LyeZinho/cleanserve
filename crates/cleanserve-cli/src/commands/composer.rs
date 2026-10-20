use anyhow::Context;
use cleanserve_core::{cleanserve_home, PhpDownloader, PhpManager};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const COMPOSER_PHAR_URL: &str = "https://getcomposer.org/download/latest-stable/composer.phar";
const DEFAULT_PHP: &str = "8.4";

pub async fn run(args: Vec<String>, php_override: Option<String>) -> anyhow::Result<()> {
    let config_path = Path::new("cleanserve.json");
    let config = if config_path.exists() {
        Some(
            cleanserve_core::CleanServeConfig::load(config_path)
                .context("Failed to load cleanserve.json")?,
        )
    } else {
        None
    };

    if config.is_none() && !is_create_project(&args) && Path::new("composer.json").exists() {
        println!("No cleanserve.json found. Run 'cleanserve init' to pin a PHP version.");
        println!("Using PHP {} for this command.\n", DEFAULT_PHP);
    } else if config.is_none() && !is_create_project(&args) && !Path::new("composer.json").exists() {
        anyhow::bail!(
            "No cleanserve.json found. Run 'cleanserve init' first, or use:\n  cleanserve composer create-project vendor/package dir"
        );
    }

    let php_version = php_override
        .or_else(|| config.as_ref().map(|c| c.engine.php.clone()))
        .unwrap_or_else(|| DEFAULT_PHP.to_string());

    let managed_php = ensure_php(&php_version).await?;
    let php_path = resolve_php_for_composer(&managed_php, &php_version)?;
    let composer_path = ensure_composer().await?;

    println!("🎼 Running Composer with PHP {}", php_version);
    println!("📦 Command: composer {}\n", args.join(" "));

    let php_bin_dir = php_path.parent().unwrap_or(Path::new("."));
    let path_env = prepend_path(php_bin_dir);

    let composer_home = cleanserve_home()
        .context("Failed to resolve CleanServe home")?
        .join("composer");
    std::fs::create_dir_all(&composer_home).ok();

    let mut cmd = Command::new(&php_path);
    if let Some(ini) = crate::commands::project_env::project_php_ini(&php_path) {
        cmd.arg("-c").arg(ini);
    }
    let status = cmd
        .arg(&composer_path)
        .args(&args)
        .env("PATH", path_env)
        .env("PHP_BINARY", &php_path)
        .env("COMPOSER_HOME", &composer_home)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to execute composer")?;

    if !status.success() {
        anyhow::bail!("Composer exited with code: {:?}", status.code());
    }

    if let Some(target) = create_project_target(&args) {
        maybe_init_created_project(&target, &php_version).await?;
    }

    Ok(())
}

async fn ensure_php(version: &str) -> anyhow::Result<PathBuf> {
    let manager = PhpManager::new().context("Failed to init PHP manager")?;
    if let Some(path) = manager.get_path(version) {
        return Ok(path);
    }

    println!("📦 PHP {} not found locally. Downloading...", version);
    let project_php = Path::new(".cleanserve").join("php");
    let downloader = if project_php.exists() {
        PhpDownloader::new(&project_php).context("Failed to initialize project PHP downloader")?
    } else {
        PhpDownloader::global().context("Failed to initialize PHP downloader")?
    };

    match downloader.ensure(version).await {
        Ok(path) => Ok(path),
        Err(e) => {
            anyhow::bail!(
                "PHP {} is not available ({}).\n\
                 Searched: .cleanserve/php/, ~/.cleanserve/php/, ~/.cleanserve/bin/\n\
                 Run: cleanserve update --version {}\n\
                 If you built from source, run: ./scripts/install-local.sh (your `cleanserve --version` should show a git hash, not only 0.3.0)",
                version,
                e,
                version
            );
        }
    }
}

async fn ensure_composer() -> anyhow::Result<PathBuf> {
    if let Some(existing) = find_composer() {
        return Ok(existing);
    }

    println!("📦 Composer not found. Downloading composer.phar...");
    download_composer_phar().await
}

fn find_composer() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from("composer.phar")];

    if let Ok(home) = cleanserve_home() {
        candidates.push(home.join("tools").join("composer.phar"));
    }

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".composer").join("composer.phar"));
        candidates.push(
            home.join(".composer")
                .join("vendor")
                .join("bin")
                .join("composer"),
        );
        candidates.push(home.join(".config").join("composer").join("composer.phar"));
    }

    for path in candidates {
        if path.is_file() {
            return Some(path);
        }
    }

    which("composer")
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let bat = dir.join(format!("{}.bat", name));
            if bat.is_file() {
                return Some(bat);
            }
            let exe = dir.join(format!("{}.exe", name));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

async fn download_composer_phar() -> anyhow::Result<PathBuf> {
    let dest = cleanserve_home()
        .context("Failed to resolve CleanServe home")?
        .join("tools")
        .join("composer.phar");
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(COMPOSER_PHAR_URL)
        .send()
        .await
        .context("Failed to download composer.phar")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download Composer (HTTP {}). Install it from https://getcomposer.org/",
            response.status()
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read composer.phar")?;

    if bytes.len() < 1024 || !looks_like_php(&bytes) {
        anyhow::bail!("Downloaded composer.phar looks invalid. Try again.");
    }

    std::fs::write(&dest, &bytes).context("Failed to save composer.phar")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }

    println!("✅ Composer downloaded to {}", dest.display());
    Ok(dest)
}

/// Composer needs these even for `--version`. php-runtimes currently builds
/// with `--disable-all` and omits several of them.
const COMPOSER_REQUIRED_EXTS: &[&str] = &[
    "json",
    "phar",
    "hash",
    "filter",
    "mbstring",
    "openssl",
    "tokenizer",
    "ctype",
];

fn resolve_php_for_composer(managed: &Path, php_version: &str) -> anyhow::Result<PathBuf> {
    if php_has_composer_extensions(managed) {
        return Ok(managed.to_path_buf());
    }

    if let Ok(override_path) = std::env::var("CLEANSERVE_COMPOSER_PHP") {
        let path = PathBuf::from(override_path);
        if php_has_composer_extensions(&path) {
            println!("Using CLEANSERVE_COMPOSER_PHP={}", path.display());
            return Ok(path);
        }
    }

    if let Some(system_php) = which("php").filter(|p| php_has_composer_extensions(p)) {
        println!(
            "⚠️  CleanServe PHP ({}) is missing Composer extensions (phar, tokenizer, …).",
            managed.display()
        );
        println!(
            "   Using system PHP {} for this command.",
            system_php.display()
        );
        println!(
            "   After php-runtimes releases new binaries: cleanserve update --version {}",
            php_version
        );
        return Ok(system_php);
    }

    let missing = missing_composer_extensions(managed);
    anyhow::bail!(
        "This PHP build is missing extensions Composer needs: {}.\n\
         Install system PHP (e.g. pacman -S php) or set CLEANSERVE_COMPOSER_PHP=/usr/bin/php\n\
         Long-term: rebuild php-runtimes with phar enabled, then: cleanserve update --version {}",
        missing.join(", "),
        php_version
    )
}

fn php_has_composer_extensions(php_path: &Path) -> bool {
    missing_composer_extensions(php_path).is_empty()
}

fn missing_composer_extensions(php_path: &Path) -> Vec<&'static str> {
    let output = Command::new(php_path).arg("-m").output();
    let modules = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_ascii_lowercase(),
        _ => return COMPOSER_REQUIRED_EXTS.to_vec(),
    };
    COMPOSER_REQUIRED_EXTS
        .iter()
        .copied()
        .filter(|ext| !modules.lines().any(|line| line.trim() == *ext))
        .collect()
}

fn prepend_path(dir: &Path) -> std::ffi::OsString {
    let mut dirs = vec![dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(dirs).unwrap_or_else(|_| dir.as_os_str().to_os_string())
}

fn looks_like_php(bytes: &[u8]) -> bool {
    bytes.starts_with(b"<?php") || bytes.starts_with(b"#!/usr/bin/env php") || bytes.windows(5).take(8).any(|w| w == b"<?php")
}

fn is_create_project(args: &[String]) -> bool {
    args.first().map(|s| s.as_str()) == Some("create-project")
}

fn create_project_target(args: &[String]) -> Option<PathBuf> {
    if !is_create_project(args) {
        return None;
    }

    let positionals = positional_after_command(args);
    if positionals.len() >= 2 {
        return Some(PathBuf::from(&positionals[1]));
    }
    if positionals.len() == 1 {
        let package = &positionals[0];
        let dir = package.rsplit('/').next().unwrap_or(package);
        return Some(PathBuf::from(dir));
    }
    None
}

fn positional_after_command(args: &[String]) -> Vec<String> {
    let mut positionals = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            positionals.extend(args[i + 1..].iter().cloned());
            break;
        }
        if arg.starts_with('-') {
            if arg.contains('=') || is_composer_flag_without_value(arg) {
                i += 1;
                continue;
            }
            i += 2;
            continue;
        }
        positionals.push(arg.clone());
        i += 1;
    }
    positionals
}

fn is_composer_flag_without_value(arg: &str) -> bool {
    matches!(
        arg,
        "--prefer-dist"
            | "--prefer-source"
            | "--prefer-install"
            | "--no-dev"
            | "--dev"
            | "--no-install"
            | "--no-security-blocking"
            | "--no-plugins"
            | "--no-scripts"
            | "--no-progress"
            | "--no-interaction"
            | "-n"
            | "--quiet"
            | "-q"
            | "-v"
            | "-vv"
            | "-vvv"
            | "--ansi"
            | "--no-ansi"
            | "--ignore-platform-reqs"
            | "--secure-http"
            | "--no-secure-http"
            | "--keep-vcs"
            | "--remove-vcs"
            | "--add-repository"
    )
}

async fn maybe_init_created_project(target: &Path, php_version: &str) -> anyhow::Result<()> {
    if !target.is_dir() {
        return Ok(());
    }
    if target.join("cleanserve.json").exists() {
        return Ok(());
    }

    let original = std::env::current_dir().context("Failed to get current directory")?;
    std::env::set_current_dir(target).with_context(|| {
        format!("Failed to enter created project {}", target.display())
    })?;
    let result = crate::commands::init::run(None, Some(php_version.to_string()), false).await;
    let _ = std::env::set_current_dir(original);
    result.context("Project created, but failed to run 'cleanserve init' in the new directory")?;

    println!(
        "\n✓ Initialized CleanServe in {}. Run:\n  cd {} && cleanserve up",
        target.display(),
        target.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_create_project_directory() {
        let args = vec![
            "create-project".into(),
            "yiisoft/app".into(),
            "your_project".into(),
        ];
        assert_eq!(
            create_project_target(&args),
            Some(PathBuf::from("your_project"))
        );
    }

    #[test]
    fn parses_create_project_package_as_directory() {
        let args = vec!["create-project".into(), "yiisoft/app".into()];
        assert_eq!(
            create_project_target(&args),
            Some(PathBuf::from("app"))
        );
    }

    #[test]
    fn skips_composer_flags() {
        let args = vec![
            "create-project".into(),
            "--prefer-dist".into(),
            "-s".into(),
            "stable".into(),
            "yiisoft/app".into(),
            "blog".into(),
        ];
        assert_eq!(create_project_target(&args), Some(PathBuf::from("blog")));
    }
}
