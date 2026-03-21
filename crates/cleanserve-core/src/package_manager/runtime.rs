use super::{PackageManagerError, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PackageRuntime {
    package_name: String,
    version: String,
    pid: Option<u32>,
    port: u16,
    status: RuntimeStatus,
    install_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

impl PackageRuntime {
    pub fn new(package_name: String, version: String, port: u16, install_path: PathBuf) -> Self {
        Self {
            package_name,
            version,
            pid: None,
            port,
            status: RuntimeStatus::Stopped,
            install_path,
        }
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn status(&self) -> &RuntimeStatus {
        &self.status
    }

    pub fn is_running(&self) -> bool {
        self.status == RuntimeStatus::Running
    }

    pub fn set_running(&mut self, pid: u32) {
        self.pid = Some(pid);
        self.status = RuntimeStatus::Running;
    }

    pub fn set_stopped(&mut self) {
        self.pid = None;
        self.status = RuntimeStatus::Stopped;
    }

    pub fn set_error(&mut self, error: String) {
        self.status = RuntimeStatus::Error(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        assert_eq!(rt.package_name(), "mysql");
        assert_eq!(rt.version(), "8.0");
        assert_eq!(rt.port(), 3306);
        assert!(!rt.is_running());
    }

    #[test]
    fn test_runtime_transitions() {
        let mut rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );

        rt.set_running(1234);
        assert!(rt.is_running());

        rt.set_stopped();
        assert!(!rt.is_running());
    }
}
