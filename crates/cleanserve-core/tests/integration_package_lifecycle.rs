use cleanserve_core::package_manager::{PackageRuntime, PackageLifecycle, RuntimeStatus};
use std::path::PathBuf;

#[tokio::test]
async fn test_package_lifecycle_full_cycle() {
    let mut lc = PackageLifecycle::new();
    let rt = PackageRuntime::new(
        "mysql".to_string(),
        "8.0".to_string(),
        3306,
        PathBuf::from("/tmp/mysql-8.0"),
    );
    
    lc.register("mysql".to_string(), rt).unwrap();
    
    lc.start_package("mysql").await.unwrap();
    assert_eq!(lc.get_status("mysql").unwrap(), RuntimeStatus::Running);
    
    lc.stop_package("mysql").await.unwrap();
    assert_eq!(lc.get_status("mysql").unwrap(), RuntimeStatus::Stopped);
}

#[tokio::test]
async fn test_package_lifecycle_multiple_packages() {
    let mut lc = PackageLifecycle::new();
    
    let rt1 = PackageRuntime::new("mysql".to_string(), "8.0".to_string(), 3306, PathBuf::from("/tmp/mysql"));
    let rt2 = PackageRuntime::new("redis".to_string(), "7.0".to_string(), 6379, PathBuf::from("/tmp/redis"));
    
    lc.register("mysql".to_string(), rt1).unwrap();
    lc.register("redis".to_string(), rt2).unwrap();
    
    lc.start_package("mysql").await.unwrap();
    lc.start_package("redis").await.unwrap();
    
    assert_eq!(lc.list_runtimes().len(), 2);
    assert!(lc.get_runtime("mysql").unwrap().is_running());
    assert!(lc.get_runtime("redis").unwrap().is_running());
}

#[tokio::test]
async fn test_package_lifecycle_errors() {
    let mut lc = PackageLifecycle::new();
    
    let result = lc.start_package("nonexistent").await;
    assert!(result.is_err());
}

#[test]
fn test_runtime_status_transitions() {
    let mut rt = PackageRuntime::new(
        "mysql".to_string(),
        "8.0".to_string(),
        3306,
        PathBuf::from("/tmp/mysql"),
    );
    
    assert!(!rt.is_running());
    rt.set_running(1234);
    assert!(rt.is_running());
    rt.set_stopped();
    assert!(!rt.is_running());
}
