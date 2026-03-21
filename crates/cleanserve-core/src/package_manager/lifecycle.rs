use super::{Result, PackageManagerError, PackageRuntime, RuntimeStatus};
use std::collections::HashMap;

pub struct PackageLifecycle {
    runtimes: HashMap<String, PackageRuntime>,
}

impl PackageLifecycle {
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    pub fn register(&mut self, package_name: String, runtime: PackageRuntime) -> Result<()> {
        if self.runtimes.contains_key(&package_name) {
            return Err(PackageManagerError {
                message: format!("Package '{}' already registered", package_name),
            });
        }
        self.runtimes.insert(package_name, runtime);
        Ok(())
    }

    pub fn get_runtime(&self, package_name: &str) -> Option<&PackageRuntime> {
        self.runtimes.get(package_name)
    }

    pub fn get_runtime_mut(&mut self, package_name: &str) -> Option<&mut PackageRuntime> {
        self.runtimes.get_mut(package_name)
    }

    pub fn list_runtimes(&self) -> Vec<&PackageRuntime> {
        self.runtimes.values().collect()
    }

    pub async fn start_package(&mut self, package_name: &str) -> Result<()> {
        let runtime = self.get_runtime_mut(package_name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not registered", package_name),
            })?;

        if runtime.is_running() {
            return Err(PackageManagerError {
                message: format!("Package '{}' is already running", package_name),
            });
        }

        runtime.set_status(RuntimeStatus::Starting);
        runtime.set_running(1234);
        
        Ok(())
    }

    pub async fn stop_package(&mut self, package_name: &str) -> Result<()> {
        let runtime = self.get_runtime_mut(package_name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not registered", package_name),
            })?;

        if !runtime.is_running() {
            return Err(PackageManagerError {
                message: format!("Package '{}' is not running", package_name),
            });
        }

        runtime.set_status(RuntimeStatus::Stopping);
        runtime.set_stopped();

        Ok(())
    }

    pub fn get_status(&self, package_name: &str) -> Result<RuntimeStatus> {
        let runtime = self.get_runtime(package_name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not registered", package_name),
            })?;

        Ok(runtime.status().clone())
    }
}

impl Default for PackageLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_lifecycle_register_and_start() {
        let mut lc = PackageLifecycle::new();
        let rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        
        lc.register("mysql".to_string(), rt).unwrap();
        lc.start_package("mysql").await.unwrap();
        
        let status = lc.get_status("mysql").unwrap();
        assert_eq!(status, RuntimeStatus::Running);
    }

    #[tokio::test]
    async fn test_lifecycle_start_stop() {
        let mut lc = PackageLifecycle::new();
        let rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        
        lc.register("mysql".to_string(), rt).unwrap();
        lc.start_package("mysql").await.unwrap();
        lc.stop_package("mysql").await.unwrap();
        
        let status = lc.get_status("mysql").unwrap();
        assert_eq!(status, RuntimeStatus::Stopped);
    }
}
