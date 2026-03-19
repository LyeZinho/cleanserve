//! Static File Blacklist Module
//!
//! Prevents access to sensitive files like .env, .git, configuration files,
//! and uploaded PHP files. Returns 403 Forbidden for blacklisted paths.

use std::path::Path;

/// Files and directories that should never be served
const BLACKLISTED_FILES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.*.local",
    ".git",
    ".gitignore",
    ".gitattributes",
    ".github",
    ".gitlab-ci.yml",
    ".circleci",
    ".travis.yml",
    ".htaccess",
    "composer.json",
    "composer.lock",
    "package.json",
    "package-lock.json",
    "yarn.lock",
    ".npmrc",
    ".yarnrc",
    "webpack.config.js",
    "vite.config.js",
    "tsconfig.json",
    "dockerfile",
    ".dockerignore",
    "docker-compose.yml",
    ".editorconfig",
    ".prettierrc",
    ".eslintrc",
    "phpunit.xml",
    ".phpunit.xml",
    "pytest.ini",
    "setup.py",
    "setup.cfg",
    "makefile",
    "gemfile",
    "rakefile",
    ".env.example",
    "readme.md",
    "license",
    "changelog.md",
    ".well-known",
];

/// Extensions that should not be served from upload directories
const FORBIDDEN_UPLOAD_EXTENSIONS: &[&str] = &[
    "php", "php3", "php4", "php5", "php7", "php8", "phtml", "phar", "inc", "pl", "py", "jsp",
    "asp", "aspx", "cgi", "exe", "sh", "bat", "cmd", "com", "bin", "app", "jar", "war", "class",
    "o", "so", "dll", "so", "dylib",
];

pub struct StaticBlacklist;

impl StaticBlacklist {
    /// Check if a path should be blocked from serving
    ///
    /// Returns true if the path contains blacklisted files or directories
    pub fn is_blocked(path: &str) -> bool {
        let path_lower = path.to_lowercase();

        for blacklisted in BLACKLISTED_FILES {
            if path_lower.ends_with(blacklisted)
                || path_lower.contains(&format!("/{}/", blacklisted))
                || path_lower.contains(&format!("/{}?", blacklisted))
                || path_lower == *blacklisted
                || path_lower.contains(&format!("/{}", blacklisted))
            {
                return true;
            }
        }

        false
    }

    /// Check if a file in an upload directory should be blocked
    ///
    /// Returns true if the file has a dangerous extension
    pub fn is_dangerous_upload(file_path: &str) -> bool {
        let path = Path::new(file_path);

        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                let ext_lower = ext_str.to_lowercase();
                return FORBIDDEN_UPLOAD_EXTENSIONS.contains(&ext_lower.as_str());
            }
        }

        false
    }

    /// Check if path is under an upload directory
    pub fn is_in_upload_dir(path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.contains("/uploads/")
            || path_lower.contains("/upload/")
            || path_lower.contains("/tmp/")
            || path_lower.contains("/temp/")
            || path_lower.contains("/user_files/")
            || path_lower.contains("/files/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_env_file() {
        assert!(StaticBlacklist::is_blocked("/.env"));
        assert!(StaticBlacklist::is_blocked(".env"));
        assert!(StaticBlacklist::is_blocked("path/to/.env"));
    }

    #[test]
    fn test_blocks_git_directory() {
        assert!(StaticBlacklist::is_blocked("/.git"));
        assert!(StaticBlacklist::is_blocked(".git"));
        assert!(StaticBlacklist::is_blocked("/.git/config"));
    }

    #[test]
    fn test_blocks_composer_files() {
        assert!(StaticBlacklist::is_blocked("/composer.json"));
        assert!(StaticBlacklist::is_blocked("/composer.lock"));
    }

    #[test]
    fn test_allows_normal_files() {
        assert!(!StaticBlacklist::is_blocked("/index.php"));
        assert!(!StaticBlacklist::is_blocked("/style.css"));
        assert!(!StaticBlacklist::is_blocked("/app.js"));
        assert!(!StaticBlacklist::is_blocked("/image.png"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(StaticBlacklist::is_blocked("/.ENV"));
        assert!(StaticBlacklist::is_blocked("/.Git"));
        assert!(StaticBlacklist::is_blocked("/.GIT/config"));
    }

    #[test]
    fn test_dangerous_upload_extensions() {
        assert!(StaticBlacklist::is_dangerous_upload("shell.php"));
        assert!(StaticBlacklist::is_dangerous_upload("shell.php5"));
        assert!(StaticBlacklist::is_dangerous_upload("script.asp"));
        assert!(StaticBlacklist::is_dangerous_upload("script.jsp"));
    }

    #[test]
    fn test_safe_upload_extensions() {
        assert!(!StaticBlacklist::is_dangerous_upload("document.pdf"));
        assert!(!StaticBlacklist::is_dangerous_upload("image.png"));
        assert!(!StaticBlacklist::is_dangerous_upload("image.jpg"));
        assert!(!StaticBlacklist::is_dangerous_upload("archive.zip"));
    }

    #[test]
    fn test_upload_directory_detection() {
        assert!(StaticBlacklist::is_in_upload_dir("/uploads/file.txt"));
        assert!(StaticBlacklist::is_in_upload_dir("/upload/file.txt"));
        assert!(StaticBlacklist::is_in_upload_dir("/tmp/file.txt"));
        assert!(StaticBlacklist::is_in_upload_dir("/user_files/file.txt"));
        assert!(!StaticBlacklist::is_in_upload_dir("/public/index.php"));
    }

    #[test]
    fn test_blocks_docker_files() {
        assert!(StaticBlacklist::is_blocked("/Dockerfile"));
        assert!(StaticBlacklist::is_blocked("/docker-compose.yml"));
    }

    #[test]
    fn test_blocks_readme_and_license() {
        assert!(StaticBlacklist::is_blocked("/README.md"));
        assert!(StaticBlacklist::is_blocked("/LICENSE"));
        assert!(StaticBlacklist::is_blocked("/CHANGELOG.md"));
    }
}
