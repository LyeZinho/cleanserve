use crate::{CleanServeError, Result};
use std::path::{Path, PathBuf};

/// Locate managed PHP binaries.
///
/// Search order:
/// 1. Project-local `.cleanserve/php/`
/// 2. Global `~/.cleanserve/php/` (or `$CLEANSERVE_HOME/php`)
/// 3. Legacy `~/.cleanserve/bin/`
pub struct PhpManager {
    search_dirs: Vec<PathBuf>,
}

impl PhpManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            search_dirs: default_search_dirs()?,
        })
    }

    pub fn with_search_dirs(search_dirs: Vec<PathBuf>) -> Self {
        Self { search_dirs }
    }

    /// Directory used for new PHP installs (shared across projects).
    pub fn global_php_dir() -> Result<PathBuf> {
        Ok(cleanserve_home()?.join("php"))
    }

    pub fn project_php_dir() -> PathBuf {
        PathBuf::from(".cleanserve").join("php")
    }

    pub fn list_installed(&self) -> Vec<String> {
        let mut versions = Vec::new();
        for dir in &self.search_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = match entry.file_name().into_string() {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    if let Some(version) = name.strip_prefix("php-") {
                        if find_php_binary(&entry.path()).is_some()
                            && !versions.iter().any(|v| v == version)
                        {
                            versions.push(version.to_string());
                        }
                    }
                }
            }
        }
        versions.sort_by(|a, b| crate::version_manifest::version_cmp(a, b));
        versions
    }

    /// Resolve a PHP CLI binary for `version`.
    ///
    /// `"8.4"` matches the newest installed `8.4.x`. `"8.4.19"` matches exactly.
    pub fn get_path(&self, version: &str) -> Option<PathBuf> {
        let mut matches: Vec<(String, PathBuf)> = Vec::new();

        for dir in &self.search_dirs {
            if let Some(bin) = find_php_binary(&dir.join(format!("php-{}", version))) {
                if is_exact_version(version) {
                    return Some(bin);
                }
                matches.push((version.to_string(), bin));
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = match entry.file_name().into_string() {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    let Some(installed) = name.strip_prefix("php-") else {
                        continue;
                    };
                    if !version_matches(installed, version) {
                        continue;
                    }
                    if let Some(bin) = find_php_binary(&entry.path()) {
                        if installed == version && is_exact_version(version) {
                            return Some(bin);
                        }
                        matches.push((installed.to_string(), bin));
                    }
                }
            }
        }

        matches.sort_by(|a, b| crate::version_manifest::version_cmp(&a.0, &b.0));
        matches.pop().map(|(_, bin)| bin)
    }

    pub fn is_installed(&self, version: &str) -> bool {
        self.get_path(version).is_some()
    }

    /// Resolved patch version currently installed for `query`, if any.
    pub fn resolve_installed_version(&self, query: &str) -> Option<String> {
        let path = self.get_path(query)?;
        version_from_binary_path(&path).or_else(|| Some(query.to_string()))
    }
}

pub fn cleanserve_home() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("CLEANSERVE_HOME") {
        if !home.trim().is_empty() {
            return Ok(PathBuf::from(home));
        }
    }

    let home = dirs::home_dir()
        .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?;
    Ok(home.join(".cleanserve"))
}

pub fn find_php_binary(install_dir: &Path) -> Option<PathBuf> {
    if !install_dir.exists() {
        return None;
    }

    let version = install_dir
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("php-"));

    let mut candidates = vec![
        install_dir.join("php"),
        install_dir.join("bin").join("php"),
    ];

    #[cfg(windows)]
    {
        candidates.push(install_dir.join("php.exe"));
        candidates.push(install_dir.join("bin").join("php.exe"));
    }

    if let Some(v) = version {
        candidates.push(install_dir.join(format!("php-{}", v)));
        candidates.push(install_dir.join("linux-x64").join(format!("php-{}", v)));
        candidates.push(install_dir.join("linux-x64").join("php"));
        candidates.push(install_dir.join("bin").join(format!("php-{}", v)));
        #[cfg(windows)]
        {
            candidates.push(install_dir.join(format!("php-{}.exe", v)));
            candidates.push(install_dir.join("windows-x64").join(format!("php-{}.exe", v)));
        }
    }

    for candidate in &candidates {
        if is_php_cli_binary(candidate) {
            return Some(candidate.clone());
        }
    }

    walk_for_php(install_dir, 0, 4)
}

fn default_search_dirs() -> Result<Vec<PathBuf>> {
    let home = cleanserve_home()?;
    Ok(vec![
        PhpManager::project_php_dir(),
        home.join("php"),
        home.join("bin"),
    ])
}

fn is_exact_version(version: &str) -> bool {
    version.split('.').filter(|p| !p.is_empty()).count() >= 3
}

pub fn version_matches(installed: &str, query: &str) -> bool {
    installed == query || installed.starts_with(&format!("{}.", query))
}

fn is_php_cli_binary(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name = name.strip_suffix(".exe").unwrap_or(name);
    if name == "php" {
        return true;
    }
    if let Some(rest) = name.strip_prefix("php-") {
        return rest.chars().next().is_some_and(|c| c.is_ascii_digit());
    }
    false
}

fn walk_for_php(dir: &Path, depth: usize, max_depth: usize) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && is_php_cli_binary(&path) {
            return Some(path);
        }
        if path.is_dir() {
            dirs.push(path);
        }
    }
    for dir in dirs {
        if let Some(found) = walk_for_php(&dir, depth + 1, max_depth) {
            return Some(found);
        }
    }
    None
}

fn version_from_binary_path(path: &Path) -> Option<String> {
    for ancestor in path.ancestors() {
        if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if let Some(version) = name.strip_prefix("php-") {
                if version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"php").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn finds_php_runtimes_layout() {
        let tmp = TempDir::new().unwrap();
        let install = tmp.path().join("php-8.4.19");
        let binary = install.join("linux-x64").join("php-8.4.19");
        touch(&binary);

        assert_eq!(find_php_binary(&install).as_deref(), Some(binary.as_path()));

        let manager = PhpManager::with_search_dirs(vec![tmp.path().to_path_buf()]);
        assert!(manager.is_installed("8.4"));
        assert!(manager.is_installed("8.4.19"));
        assert!(!manager.is_installed("8.3"));
        assert_eq!(manager.get_path("8.4").as_deref(), Some(binary.as_path()));
    }

    #[test]
    fn finds_standard_bin_layout() {
        let tmp = TempDir::new().unwrap();
        let install = tmp.path().join("php-8.3.0");
        let binary = install.join("bin").join("php");
        touch(&binary);

        let manager = PhpManager::with_search_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(manager.get_path("8.3").as_deref(), Some(binary.as_path()));
    }

    #[test]
    fn prefers_newest_patch_for_minor_query() {
        let tmp = TempDir::new().unwrap();
        let older = tmp.path().join("php-8.4.1").join("php");
        let newer = tmp.path().join("php-8.4.19").join("php");
        touch(&older);
        touch(&newer);

        let manager = PhpManager::with_search_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(manager.get_path("8.4").as_deref(), Some(newer.as_path()));
        assert_eq!(manager.get_path("8.4.1").as_deref(), Some(older.as_path()));
    }

    #[test]
    fn version_matches_prefix_not_sibling() {
        assert!(version_matches("8.4.19", "8.4"));
        assert!(version_matches("8.4.19", "8.4.19"));
        assert!(!version_matches("8.41.0", "8.4"));
        assert!(!version_matches("8.3.0", "8.4"));
    }
}
