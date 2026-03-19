use crate::{CleanServeError, Result};
use std::path::PathBuf;
use tracing::info;

pub struct ExtensionManager {
    php_home: PathBuf,
}

impl ExtensionManager {
    pub fn new() -> Result<Self> {
        let php_home = dirs::home_dir()
            .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
            .join(".cleanserve")
            .join("bin");

        Ok(Self { php_home })
    }

    /// Generate php.ini with specified extensions
    pub fn generate_php_ini(
        &self,
        version: &str,
        extensions: &[String],
        display_errors: bool,
        memory_limit: Option<&str>,
    ) -> Result<String> {
        let php_version_dir = self.php_home.join(format!("php-{}", version));

        // Determine extension directory
        #[cfg(windows)]
        let ext_dir = php_version_dir.join("ext");
        #[cfg(not(windows))]
        let ext_dir = php_version_dir.join("lib").join("php").join("extensions");

        let mut ini_content = String::new();

        // PHP configuration
        ini_content.push_str("; CleanServe Generated php.ini\n");
        ini_content.push_str(&format!("; PHP Version: {}\n\n", version));

        // Core settings
        ini_content.push_str("[PHP]\n");
        ini_content.push_str("engine = On\n");
        ini_content.push_str("short_open_tag = Off\n");
        ini_content.push_str("precision = 14\n");
        ini_content.push_str("output_buffering = 4096\n");
        ini_content.push_str("zlib.output_compression = Off\n");
        ini_content.push_str("implicit_flush = Off\n");
        ini_content.push_str("serialize_callback = \n");
        ini_content.push_str("disable_functions = \n");
        ini_content.push_str("disable_classes = \n");
        ini_content.push_str("zend.enable_gc = On\n");
        ini_content.push_str("zend.exception_ignore_args = On\n");
        ini_content.push_str("expose_php = On\n");
        ini_content.push_str("max_execution_time = 30\n");
        ini_content.push_str("max_input_time = 60\n");
        ini_content.push_str(&format!(
            "memory_limit = {}\n",
            memory_limit.unwrap_or("128M")
        ));

        // Error display
        ini_content.push_str(&format!(
            "display_errors = {}\n",
            if display_errors { "On" } else { "Off" }
        ));
        ini_content.push_str("display_startup_errors = On\n");
        ini_content.push_str("log_errors = On\n");
        ini_content.push_str("error_log = php://stderr\n");
        ini_content.push_str("error_reporting = E_ALL\n");

        // Paths
        #[cfg(windows)]
        {
            let ext_dir_str = ext_dir.to_string_lossy().replace("\\", "\\\\");
            ini_content.push_str(&format!("extension_dir = \"{}\"\n", ext_dir_str));
        }
        #[cfg(not(windows))]
        {
            ini_content.push_str(&format!("extension_dir = \"{}\"\n", ext_dir.display()));
        }

        // Extensions
        if !extensions.is_empty() {
            ini_content.push_str("\n; Extensions\n");
            for ext in extensions {
                // Handle common extension names
                let ext_name = if ext.starts_with("php_") || ext.starts_with("extension=") {
                    ext.clone()
                } else {
                    format!("extension={}", ext)
                };
                ini_content.push_str(&format!("{}\n", ext_name));
            }
        }

        // Windows-specific settings
        #[cfg(windows)]
        {
            ini_content.push_str("\n[Windows Extensions]\n");
            ini_content.push_str("extension=curl\n");
            ini_content.push_str("extension=gd\n");
            ini_content.push_str("extension=intl\n");
            ini_content.push_str("extension=mbstring\n");
            ini_content.push_str("extension=mysqli\n");
            ini_content.push_str("extension=openssl\n");
            ini_content.push_str("extension=pdo_mysql\n");
            ini_content.push_str("extension=pdo_sqlite\n");
            ini_content.push_str("extension=sockets\n");
        }

        Ok(ini_content)
    }

    /// Write php.ini to a specific path
    pub fn write_php_ini(
        &self,
        version: &str,
        extensions: &[String],
        display_errors: bool,
        memory_limit: Option<&str>,
        output_path: &PathBuf,
    ) -> Result<()> {
        let ini_content =
            self.generate_php_ini(version, extensions, display_errors, memory_limit)?;

        std::fs::write(output_path, ini_content)
            .map_err(|e| CleanServeError::Config(format!("Failed to write php.ini: {}", e)))?;

        info!("Generated php.ini at {}", output_path.display());
        Ok(())
    }
}
