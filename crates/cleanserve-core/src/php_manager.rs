use crate::{CleanServeError, Result};
use std::path::PathBuf;

pub struct PhpManager {
    cache_dir: PathBuf,
}

impl PhpManager {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
            .join(".cleanserve")
            .join("bin");

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir })
    }

    pub fn list_installed(&self) -> Vec<String> {
        std::fs::read_dir(&self.cache_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                if name.starts_with("php-") {
                    Some(name.strip_prefix("php-")?.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_path(&self, version: &str) -> Option<PathBuf> {
        let path = self.cache_dir.join(format!("php-{}", version));
        #[cfg(windows)]
        let exe = path.join("php.exe");
        #[cfg(not(windows))]
        let exe = path.join("bin").join("php");

        if exe.exists() {
            Some(exe)
        } else {
            None
        }
    }

    pub fn is_installed(&self, version: &str) -> bool {
        self.get_path(version).is_some()
    }
}
