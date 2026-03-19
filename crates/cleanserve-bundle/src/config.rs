use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleConfig {
    pub name: String,
    pub version: String,
    pub php_version: String,
    pub php_extensions: Vec<String>,
    pub entry_point: String,
    pub compression: CompressionType,
    pub preload_script: Option<String>,
    pub env_inlined: bool,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    Gzip,
    Xz,
    Zstd,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            name: "app".to_string(),
            version: "1.0.0".to_string(),
            php_version: "8.4".to_string(),
            php_extensions: vec![
                "phar".to_string(),
                "opcache".to_string(),
                "pdo".to_string(),
                "mbstring".to_string(),
            ],
            entry_point: "public/index.php".to_string(),
            compression: CompressionType::Gzip,
            preload_script: None,
            env_inlined: false,
            exclude_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                ".env".to_string(),
                ".env.*".to_string(),
                "*.log".to_string(),
                "var/".to_string(),
                "storage/".to_string(),
            ],
        }
    }
}

impl BundleConfig {
    pub fn new(name: &str) -> Self {
        let mut config = Self::default();
        config.name = name.to_string();
        config
    }

    pub fn with_php_version(mut self, version: &str) -> Self {
        self.php_version = version.to_string();
        self
    }

    pub fn with_entry_point(mut self, entry: &str) -> Self {
        self.entry_point = entry.to_string();
        self
    }

    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_preload_script(mut self, script: &str) -> Self {
        self.preload_script = Some(script.to_string());
        self
    }

    pub fn with_env_inlined(mut self) -> Self {
        self.env_inlined = true;
        self
    }

    pub fn add_extension(mut self, ext: &str) -> Self {
        self.php_extensions.push(ext.to_string());
        self
    }

    pub fn exclude(mut self, pattern: &str) -> Self {
        self.exclude_patterns.push(pattern.to_string());
        self
    }
}

impl Default for CompressionType {
    fn default() -> Self {
        CompressionType::Gzip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BundleConfig::default();
        assert_eq!(config.name, "app");
        assert_eq!(config.php_version, "8.4");
        assert!(config.php_extensions.contains(&"opcache".to_string()));
    }

    #[test]
    fn test_builder_pattern() {
        let config = BundleConfig::new("myapp")
            .with_php_version("8.3")
            .with_entry_point("index.php")
            .with_env_inlined()
            .add_extension("gd");

        assert_eq!(config.name, "myapp");
        assert_eq!(config.php_version, "8.3");
        assert_eq!(config.entry_point, "index.php");
        assert!(config.env_inlined);
        assert!(config.php_extensions.contains(&"gd".to_string()));
    }
}
