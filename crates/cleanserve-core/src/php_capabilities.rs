use std::path::Path;
use std::process::Command;

/// Extensions Composer needs even for `create-project`.
/// `xmlwriter` is required by phpunit/phpunit (a dependency of yiisoft/app,
/// laravel/laravel, …); `xmlreader` ships alongside it in every normal build.
pub const COMPOSER_REQUIRED_EXTENSIONS: &[&str] = &[
    "json",
    "phar",
    "hash",
    "filter",
    "mbstring",
    "openssl",
    "tokenizer",
    "ctype",
    "dom",
    "xml",
    "xmlwriter",
    "xmlreader",
];

pub fn is_composer_capable(php_path: &Path) -> bool {
    missing_composer_extensions(php_path).is_empty()
}

pub fn missing_composer_extensions(php_path: &Path) -> Vec<&'static str> {
    let output = Command::new(php_path).arg("-m").output();
    let modules = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_ascii_lowercase(),
        _ => return COMPOSER_REQUIRED_EXTENSIONS.to_vec(),
    };
    COMPOSER_REQUIRED_EXTENSIONS
        .iter()
        .copied()
        .filter(|ext| !modules.lines().any(|line| line.trim() == *ext))
        .collect()
}
