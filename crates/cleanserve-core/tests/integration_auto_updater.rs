use cleanserve_core::auto_updater::{UpdateChecker, BinaryDownloader};

#[tokio::test]
async fn test_update_checker_version_comparison() {
    let info = UpdateChecker::check_for_updates("0.3.0").await.unwrap();
    assert_eq!(info.current_version, "0.3.0");
    // Live GitHub data: only assert relative to the real latest release.
    assert!(!info.latest_version.is_empty());
    assert!(info.needs_update);
}

#[tokio::test]
async fn test_update_checker_no_update_needed() {
    let latest = UpdateChecker::check_for_updates("0.0.1").await.unwrap().latest_version;
    let info = UpdateChecker::check_for_updates(&latest).await.unwrap();
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
