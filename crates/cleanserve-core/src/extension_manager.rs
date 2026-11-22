use crate::php_manager::cleanserve_home;
use crate::{CleanServeError, Result};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct ExtensionManager {
    php_home: PathBuf,
}

impl ExtensionManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            php_home: cleanserve_home()?.join("php"),
        })
    }

    pub fn with_install_root(install_root: PathBuf) -> Self {
        Self {
            php_home: install_root,
        }
    }

    /// Generate php.ini with specified extensions (shared/global layout).
    pub fn generate_php_ini(
        &self,
        version: &str,
        extensions: &[String],
        display_errors: bool,
        memory_limit: Option<&str>,
    ) -> Result<String> {
        let php_version_dir = self.php_home.join(format!("php-{}", version));
        Self::generate_php_ini_at(&php_version_dir, version, extensions, display_errors, memory_limit)
    }

    pub fn generate_php_ini_at(
        install_root: &Path,
        version: &str,
        extensions: &[String],
        display_errors: bool,
        memory_limit: Option<&str>,
    ) -> Result<String> {
        #[cfg(windows)]
        let ext_dir = install_root.join("ext");
        #[cfg(not(windows))]
        let ext_dir = install_root.join("lib").join("php").join("extensions");

        let mut ini_content = String::new();
        ini_content.push_str("; CleanServe Generated php.ini\n");
        ini_content.push_str(&format!("; PHP Version: {}\n\n", version));
        ini_content.push_str("[PHP]\n");
        ini_content.push_str(&format!(
            "display_errors = {}\n",
            if display_errors { "On" } else { "Off" }
        ));
        ini_content.push_str(&format!(
            "memory_limit = {}\n",
            memory_limit.unwrap_or("128M")
        ));

        if ext_dir.is_dir() {
            #[cfg(windows)]
            let ext_dir_str = ext_dir.to_string_lossy().replace('\\', "\\\\");
            #[cfg(not(windows))]
            let ext_dir_str = ext_dir.display().to_string();
            ini_content.push_str(&format!("extension_dir = \"{}\"\n", ext_dir_str));

            if !extensions.is_empty() {
                ini_content.push_str("\n; Extensions\n");
                for ext in extensions {
                    let ext_name = if ext.starts_with("extension=") {
                        ext.clone()
                    } else {
                        format!("extension={}", ext)
                    };
                    ini_content.push_str(&format!("{}\n", ext_name));
                }
            }
        }

        Ok(ini_content)
    }

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
