use cleanserve_core::auto_updater::{UpdateChecker, BinaryDownloader};

#[tokio::test]
async fn test_update_checker_version_comparison() {
    let info = UpdateChecker::check_for_updates("0.3.0").await.unwrap();
    assert_eq!(info.current_version, "0.3.0");
    assert_eq!(info.latest_version, "0.3.1");
    assert!(info.needs_update);
}

#[tokio::test]
async fn test_update_checker_no_update_needed() {
    let info = UpdateChecker::check_for_updates("0.3.1").await.unwrap();
    assert!(!info.needs_update);
}

#[test]
fn test_binary_downloader_platform_detection() {
    let platform = BinaryDownloader::get_platform();
    assert!(!platform.is_empty());
    assert!(platform.contains('-'));
}

#[tokio::test]
async fn test_full_update_flow_placeholder() {
    let info = UpdateChecker::check_for_updates("0.3.0").await.unwrap();
    assert!(info.needs_update);
    
    // Placeholder: More detailed flow testing in Phase 4b
}
