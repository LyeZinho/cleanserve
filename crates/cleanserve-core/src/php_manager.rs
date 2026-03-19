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
        // Check multiple possible locations for the PHP binary
        let candidates = vec![
            path.join("php"),             // static-php-cli direct
            path.join("bin").join("php"), // standard layout
        ];

        #[cfg(windows)]
        let candidates = vec![path.join("php.exe"), path.join("bin").join("php.exe")];

        for exe in candidates {
            if exe.exists() {
                return Some(exe);
            }
        }
        None
    }

    pub fn is_installed(&self, version: &str) -> bool {
        self.get_path(version).is_some()
    }
}
