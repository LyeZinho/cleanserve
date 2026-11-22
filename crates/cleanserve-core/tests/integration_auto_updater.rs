//! Integration tests for the auto-updater. They must never touch the network:
//! the release metadata is a fixture fed to the same code path the CLI uses.

use cleanserve_core::auto_updater::{
    archive, BinaryDownloader, GitHubRelease, UpdateChecker, UpdateInstaller,
};

fn fixture_release(tag: &str) -> GitHubRelease {
    let asset = UpdateChecker::asset_name().unwrap();
    serde_json::from_value(serde_json::json!({
        "tag_name": tag,
        "assets": [
            { "name": asset, "browser_download_url": "https://example.invalid/archive" },
            { "name": "SHA256SUMS", "browser_download_url": "https://example.invalid/SHA256SUMS" }
        ]
    }))
    .unwrap()
}

#[test]
fn test_update_checker_version_comparison() {
    let info = UpdateChecker::evaluate("0.3.0", &fixture_release("v0.3.1")).unwrap();
    assert_eq!(info.current_version, "0.3.0");
    assert_eq!(info.latest_version, "0.3.1");
    assert!(info.needs_update);
    assert!(info.download_url.is_some());
    assert!(info.checksum_url.is_some());
}

#[test]
fn test_update_checker_no_update_needed() {
    let info = UpdateChecker::evaluate("0.3.1", &fixture_release("v0.3.1")).unwrap();
    assert!(!info.needs_update);
}

#[test]
fn test_binary_downloader_platform_detection() {
    let platform = BinaryDownloader::get_platform();
    assert!(!platform.is_empty());
    assert!(platform.contains('-'));
}

#[cfg(unix)]
#[tokio::test]
async fn test_full_update_flow_offline() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::TempDir::new().unwrap();

    // "Downloaded" release archive containing a runnable fake binary.
    let archive_path = dir.path().join(UpdateChecker::asset_name().unwrap());
    let gz = flate2::write::GzEncoder::new(
        std::fs::File::create(&archive_path).unwrap(),
        flate2::Compression::default(),
    );
    let mut builder = tar::Builder::new(gz);
    let script = b"#!/bin/sh\necho cleanserve 9.9.9\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(script.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, archive::binary_name(), &script[..])
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap();

    // Extract -> backup -> install -> verify, like `cleanserve update`.
    let extracted = dir.path().join("extracted");
    archive::extract_binary(&archive_path, &extracted).unwrap();

    let installed = dir.path().join("bin").join("cleanserve");
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, "old binary").unwrap();

    let backup_dir = dir.path().join("backups");
    let backup = UpdateInstaller::backup_current_binary(&installed, Some(&backup_dir)).unwrap();
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "old binary");

    UpdateInstaller::install_binary(&extracted, &installed).await.unwrap();
    let mode = std::fs::metadata(&installed).unwrap().permissions().mode();
    assert_eq!(mode & 0o111, 0o111, "installed binary must be executable");
    UpdateInstaller::verify_installation(&installed).await.unwrap();

    // Rollback restores the previous binary and keeps the backup.
    UpdateInstaller::rollback(&backup, &installed).unwrap();
    assert_eq!(std::fs::read_to_string(&installed).unwrap(), "old binary");
    assert!(backup.exists());
}
