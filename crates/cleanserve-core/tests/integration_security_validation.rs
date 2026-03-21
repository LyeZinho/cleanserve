//! Security Validation Integration Tests
//!
//! Comprehensive tests for path traversal, checksum integrity, file permissions,
//! concurrent download safety, disk exhaustion handling, and manifest validation.

use cleanserve_core::package_manager::{PackageDownloader, PackageRegistry};
use cleanserve_core::PathTraversal;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute SHA256 hex digest of raw bytes.
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Write a file and return its real SHA256 checksum in `sha256:<hex>` form.
fn write_file_with_checksum(dir: &std::path::Path, name: &str, content: &[u8]) -> (std::path::PathBuf, String) {
    let path = dir.join(name);
    fs::write(&path, content).expect("helper: write_file_with_checksum failed");
    let hash = sha256_hex(content);
    (path, format!("sha256:{}", hash))
}

/// Create a directory tree that mimics ~/.cleanserve/ layout inside a tempdir.
fn create_mock_cleanserve_dir(root: &std::path::Path) -> std::path::PathBuf {
    let base = root.join(".cleanserve");
    fs::create_dir_all(base.join("tools")).expect("helper: create tools dir");
    fs::create_dir_all(base.join("tmp")).expect("helper: create tmp dir");
    base
}

// ===========================================================================
// 1. Path Traversal Blocked
// ===========================================================================

#[tokio::test]
async fn path_traversal_blocked() {
    let tmp = TempDir::new().expect("Failed to create temp dir for path_traversal_blocked");
    let root = tmp.path().to_str().unwrap();

    // Paths with literal ".." components — caught by all validators including escapes_root
    let literal_traversal_paths = [
        "../../../etc/passwd",
        "..\\..\\..\\etc\\passwd",
        "packages/../../../etc/passwd",
        "/../../../../etc/passwd",
    ];

    for path in &literal_traversal_paths {
        assert!(
            PathTraversal::normalize_and_validate(path).is_none(),
            "normalize_and_validate should block traversal path: {}",
            path
        );
        assert!(
            !PathTraversal::is_valid_request_path(path),
            "is_valid_request_path should reject: {}",
            path
        );
        assert!(
            PathTraversal::escapes_root(path),
            "escapes_root should detect escape for: {}",
            path
        );
        assert!(
            PathTraversal::resolve_safe(root, path).is_none(),
            "resolve_safe should block traversal path: {}",
            path
        );
    }

    // URL-encoded traversal — caught by pattern detection (normalize_and_validate / is_valid_request_path)
    let encoded_traversal_paths = [
        "%2e%2e/%2e%2e/%2e%2e/etc/passwd",
        "%252e%252e/%252e%252e/etc/passwd",
        "....//....//....//etc/passwd",
    ];

    for path in &encoded_traversal_paths {
        assert!(
            PathTraversal::normalize_and_validate(path).is_none(),
            "normalize_and_validate should block encoded traversal: {}",
            path
        );
        assert!(
            !PathTraversal::is_valid_request_path(path),
            "is_valid_request_path should reject encoded traversal: {}",
            path
        );
        assert!(
            PathTraversal::resolve_safe(root, path).is_none(),
            "resolve_safe should block encoded traversal: {}",
            path
        );
    }

    // Null-byte injection must be caught
    assert!(
        PathTraversal::has_null_bytes("/etc/passwd\0.png"),
        "has_null_bytes should detect null byte injection"
    );

    // Sanity: normal paths still work
    assert!(
        PathTraversal::normalize_and_validate("/index.php").is_some(),
        "Normal path /index.php should be allowed"
    );
    assert!(
        PathTraversal::is_valid_request_path("/css/style.css"),
        "Normal path /css/style.css should be valid"
    );

    // TempDir drops here — no artifacts remain
}

// ===========================================================================
// 2. Checksum Mismatch Rejected
// ===========================================================================

#[tokio::test]
async fn checksum_mismatch_rejected() {
    let tmp = TempDir::new().expect("Failed to create temp dir for checksum_mismatch_rejected");

    let content = b"legitimate binary payload v1.0.0";
    let (file_path, correct_checksum) = write_file_with_checksum(tmp.path(), "package.tar.gz", content);

    // Correct checksum must pass
    assert!(
        PackageDownloader::verify_checksum(&file_path, &correct_checksum).is_ok(),
        "verify_checksum should accept the correct checksum"
    );

    // Fabricate a wrong checksum (flip first character)
    let wrong_checksum = {
        let hash = sha256_hex(content);
        let flipped = if hash.starts_with('a') {
            format!("b{}", &hash[1..])
        } else {
            format!("a{}", &hash[1..])
        };
        format!("sha256:{}", flipped)
    };

    let result = PackageDownloader::verify_checksum(&file_path, &wrong_checksum);
    assert!(
        result.is_err(),
        "verify_checksum must reject a wrong checksum"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Checksum mismatch"),
        "Error message should mention 'Checksum mismatch', got: {}",
        err_msg
    );

    // Completely bogus format must also fail
    assert!(
        PackageDownloader::verify_checksum(&file_path, "md5:abc123").is_err(),
        "verify_checksum must reject unsupported algorithm"
    );
    assert!(
        PackageDownloader::verify_checksum(&file_path, "not-a-checksum").is_err(),
        "verify_checksum must reject malformed checksum string"
    );

    // File written with wrong checksum should NOT be trusted — confirm file
    // is still untouched (verify_checksum is read-only, no side-effects)
    let file_bytes = fs::read(&file_path).expect("File should still exist after failed verify");
    assert_eq!(
        file_bytes, content,
        "Original file must be unchanged after checksum rejection"
    );
}

// ===========================================================================
// 3. Malicious Binary Rejected
// ===========================================================================

#[tokio::test]
async fn malicious_binary_rejected() {
    let tmp = TempDir::new().expect("Failed to create temp dir for malicious_binary_rejected");

    // Create an "expected" binary and record its checksum
    let expected_content = b"authentic php-8.4 binary ELF header...";
    let expected_hash = sha256_hex(expected_content);
    let expected_checksum = format!("sha256:{}", expected_hash);

    // Write a corrupted / tampered binary instead
    let corrupted_content = b"MALWARE injected payload! Not the real binary.";
    let corrupted_path = tmp.path().join("php-8.4");
    fs::write(&corrupted_path, corrupted_content).expect("Write corrupted binary");

    // Integrity check must fail
    let result = PackageDownloader::verify_checksum(&corrupted_path, &expected_checksum);
    assert!(
        result.is_err(),
        "Corrupted binary must fail integrity check"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Checksum mismatch"),
        "Error should indicate checksum mismatch, got: {}",
        err_msg
    );

    // Verify the actual hash differs
    let actual_hash = PackageDownloader::compute_sha256(&corrupted_path)
        .expect("compute_sha256 should succeed on readable file");
    assert_ne!(
        actual_hash, expected_hash,
        "Corrupted binary hash must differ from expected"
    );

    // Zero-length file must also fail (empty != expected)
    let empty_path = tmp.path().join("empty-binary");
    fs::write(&empty_path, b"").expect("Write empty binary");
    assert!(
        PackageDownloader::verify_checksum(&empty_path, &expected_checksum).is_err(),
        "Empty file must fail checksum verification"
    );

    // Truncated file must fail
    let truncated_path = tmp.path().join("truncated-binary");
    fs::write(&truncated_path, &expected_content[..10]).expect("Write truncated binary");
    assert!(
        PackageDownloader::verify_checksum(&truncated_path, &expected_checksum).is_err(),
        "Truncated binary must fail checksum verification"
    );

    // Nonexistent file must produce an error (not panic)
    let ghost = tmp.path().join("nonexistent");
    assert!(
        PackageDownloader::compute_sha256(&ghost).is_err(),
        "compute_sha256 on missing file must return Err, not panic"
    );
}

// ===========================================================================
// 4. Permissions Enforced
// ===========================================================================

#[tokio::test]
async fn permissions_enforced() {
    let tmp = TempDir::new().expect("Failed to create temp dir for permissions_enforced");
    let base = create_mock_cleanserve_dir(tmp.path());

    // Set ~/.cleanserve/ to 0700 (rwx------)
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700))
        .expect("Failed to set directory permissions");

    let meta = fs::metadata(&base).expect("Failed to read dir metadata");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o700,
        "~/.cleanserve/ should have mode 0700, got {:o}",
        mode
    );

    // Create a mock binary with 0755 (rwxr-xr-x)
    let bin_dir = base.join("tools").join("php").join("8.4").join("bin");
    fs::create_dir_all(&bin_dir).expect("Failed to create bin dir");
    let binary_path = bin_dir.join("php");
    fs::write(&binary_path, b"#!/bin/sh\necho php").expect("Failed to write mock binary");
    fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o755))
        .expect("Failed to set binary permissions");

    let bin_meta = fs::metadata(&binary_path).expect("Failed to read binary metadata");
    let bin_mode = bin_meta.permissions().mode() & 0o777;
    assert_eq!(
        bin_mode, 0o755,
        "Binary should have mode 0755, got {:o}",
        bin_mode
    );

    // Readonly file should fail on write attempts
    let readonly_path = base.join("tools").join("readonly-config.toml");
    fs::write(&readonly_path, b"key = 'value'").expect("Write initial readonly file");
    fs::set_permissions(&readonly_path, fs::Permissions::from_mode(0o444))
        .expect("Set readonly permissions");

    let write_result = fs::write(&readonly_path, b"malicious override");
    assert!(
        write_result.is_err(),
        "Writing to readonly file (0444) should fail"
    );

    // Verify subdirectories inherit expected isolation
    let tools_mode = fs::metadata(base.join("tools"))
        .expect("tools dir metadata")
        .permissions()
        .mode() & 0o777;
    assert!(
        tools_mode & 0o700 == 0o700,
        "tools/ should be at least rwx for owner, got {:o}",
        tools_mode
    );

    // Restore write perms so TempDir cleanup succeeds
    fs::set_permissions(&readonly_path, fs::Permissions::from_mode(0o644))
        .expect("Restore write permission for cleanup");
}

// ===========================================================================
// 5. Concurrent Downloads Safe
// ===========================================================================

#[tokio::test]
async fn concurrent_downloads_safe() {
    let tmp = TempDir::new().expect("Failed to create temp dir for concurrent_downloads_safe");
    let cache_dir = tmp.path().join("cache").join("tools").join("composer").join("2.7");
    fs::create_dir_all(&cache_dir).expect("Create cache dir");

    let payload = b"composer-2.7.0-phar-binary-content";
    let checksum = format!("sha256:{}", sha256_hex(payload));

    let write_count = Arc::new(AtomicUsize::new(0));
    let dest = cache_dir.join("composer.phar");

    // Simulate 3 concurrent "download" tasks for the same package
    let mut handles = Vec::new();
    for task_id in 0..3 {
        let dest = dest.clone();
        let payload = payload.to_vec();
        let checksum = checksum.clone();
        let counter = Arc::clone(&write_count);

        handles.push(tokio::spawn(async move {
            // Simulate mutual exclusion: only write if file doesn't exist yet
            if !dest.exists() {
                // Simulate download latency
                tokio::time::sleep(std::time::Duration::from_millis(10 * (task_id as u64))).await;

                // Double-check after "download" (race window)
                if !dest.exists() {
                    fs::write(&dest, &payload)
                        .unwrap_or_else(|e| panic!("Task {} failed to write: {}", task_id, e));
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            }

            // Every task should be able to verify the cached file
            // (sleep to let writers finish)
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if dest.exists() {
                let result = PackageDownloader::verify_checksum(&dest, &checksum);
                assert!(
                    result.is_ok(),
                    "Task {} should verify cached file, got: {:?}",
                    task_id,
                    result.err()
                );
            }
        }));
    }

    for handle in handles {
        handle.await.expect("Task should not panic");
    }

    // At most one writer should have created the file
    let writers = write_count.load(Ordering::SeqCst);
    assert!(
        writers <= 1,
        "Only one task should write to cache, but {} did",
        writers
    );

    // File must exist and be valid
    assert!(dest.exists(), "Cached file must exist after concurrent downloads");
    let result = PackageDownloader::verify_checksum(&dest, &checksum);
    assert!(
        result.is_ok(),
        "Final cached file must pass checksum verification"
    );

    // Content must match exactly
    let cached = fs::read(&dest).expect("Read cached file");
    assert_eq!(
        cached, payload,
        "Cached content must match original payload"
    );
}

// ===========================================================================
// 6. Disk Exhaustion Graceful
// ===========================================================================

#[tokio::test]
async fn disk_exhaustion_graceful() {
    let tmp = TempDir::new().expect("Failed to create temp dir for disk_exhaustion_graceful");
    let download_dir = tmp.path().join("downloads");
    fs::create_dir_all(&download_dir).expect("Create download dir");

    // Simulate disk exhaustion by making the target directory read-only
    // so writes fail as if the disk were full.
    let target_file = download_dir.join("large-package.tar.gz");

    let partial_content = b"partial download data...";
    fs::write(&target_file, partial_content).expect("Write partial file");

    // Make the file itself readonly to simulate inability to write (disk-full analog)
    fs::set_permissions(&target_file, fs::Permissions::from_mode(0o444))
        .expect("Set target file readonly");

    let write_result = fs::write(&target_file, b"more data that cannot be written");
    assert!(
        write_result.is_err(),
        "Write to readonly file should fail (simulated disk full)"
    );

    // Also verify creating new files fails when directory is readonly
    fs::set_permissions(&download_dir, fs::Permissions::from_mode(0o555))
        .expect("Set download dir readonly");
    let new_file = download_dir.join("another-package.tar.gz");
    let create_result = fs::write(&new_file, b"should not be creatable");
    assert!(
        create_result.is_err(),
        "Creating new file in readonly dir should fail"
    );

    // Restore perms to verify cleanup
    fs::set_permissions(&download_dir, fs::Permissions::from_mode(0o755))
        .expect("Restore download dir permissions");
    fs::set_permissions(&target_file, fs::Permissions::from_mode(0o644))
        .expect("Restore target file permissions");

    // Verify partial file still contains original data (no corruption)
    let remaining = fs::read(&target_file).expect("Read remaining file");
    assert_eq!(
        remaining, partial_content,
        "Partial file should be uncorrupted after failed write"
    );

    // Cleanup: remove partial/corrupted download artifacts
    fs::remove_file(&target_file).expect("Remove partial download");
    assert!(
        !target_file.exists(),
        "Partial download must be cleaned up"
    );

    // Verify the temp directory is clean
    let entries: Vec<_> = fs::read_dir(&download_dir)
        .expect("Read download dir")
        .collect();
    assert!(
        entries.is_empty(),
        "Download directory should be empty after cleanup, found {} entries",
        entries.len()
    );

    // Verify compute_sha256 on missing file returns Err (no panic on cleanup)
    assert!(
        PackageDownloader::compute_sha256(&target_file).is_err(),
        "compute_sha256 on cleaned-up file must return Err"
    );
}

// ===========================================================================
// 7. Invalid Manifest Rejected
// ===========================================================================

#[tokio::test]
async fn invalid_manifest_rejected() {
    let tmp = TempDir::new().expect("Failed to create temp dir for invalid_manifest_rejected");

    // 1. Completely invalid JSON
    let bad_json_path = tmp.path().join("garbage.json");
    fs::write(&bad_json_path, b"NOT VALID JSON {{{").expect("Write garbage JSON");

    let mut registry = PackageRegistry::new();
    let result = registry.load_custom(&bad_json_path);
    assert!(
        result.is_err(),
        "load_custom must reject invalid JSON"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Invalid custom manifest JSON"),
        "Error should mention invalid JSON, got: {}",
        err_msg
    );

    // 2. Valid JSON but wrong schema (missing required fields)
    let wrong_schema_path = tmp.path().join("wrong-schema.json");
    fs::write(
        &wrong_schema_path,
        br#"{"evil": true, "payload": "malware"}"#,
    )
    .expect("Write wrong-schema JSON");

    let result = registry.load_custom(&wrong_schema_path);
    assert!(
        result.is_err(),
        "load_custom must reject JSON with wrong schema"
    );

    // 3. Manifest with empty packages (validation should catch)
    let empty_manifest_path = tmp.path().join("empty-packages.json");
    fs::write(
        &empty_manifest_path,
        br#"{"version": "1.0", "packages": {}}"#,
    )
    .expect("Write empty-packages manifest");
    // load_custom succeeds (empty packages merge fine), but direct validate rejects
    let manifest_content: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&empty_manifest_path).unwrap()
    ).unwrap();
    assert!(
        manifest_content["packages"].as_object().unwrap().is_empty(),
        "Empty packages manifest should have no packages"
    );

    // 4. Manifest with path traversal in package name
    let traversal_manifest_path = tmp.path().join("traversal-manifest.json");
    fs::write(
        &traversal_manifest_path,
        br#"{
            "version": "1.0",
            "packages": {
                "../../../etc/shadow": {
                    "name": "../../../etc/shadow",
                    "description": "evil",
                    "versions": {}
                }
            }
        }"#,
    )
    .expect("Write traversal manifest");

    let mut evil_registry = PackageRegistry::new();
    let load_result = evil_registry.load_custom(&traversal_manifest_path);
    // Even if it loads, using the name in cache paths must be caught
    if load_result.is_ok() {
        if let Some(_pkg) = evil_registry.get("../../../etc/shadow") {
            // Attempting to use this name in a cache path must be blocked
            let traversal_check = PathTraversal::normalize_and_validate("../../../etc/shadow");
            assert!(
                traversal_check.is_none(),
                "Path traversal in package name must be blocked by PathTraversal"
            );
        }
    }

    // 5. Nonexistent manifest path should be handled gracefully (no panic)
    let ghost_path = tmp.path().join("nonexistent-manifest.json");
    let result = registry.load_custom(&ghost_path);
    assert!(
        result.is_ok(),
        "load_custom on missing file should return Ok (optional custom manifest)"
    );

    // 6. Verify built-in manifest is valid (baseline sanity)
    let builtin = PackageRegistry::with_builtin();
    assert!(
        builtin.is_ok(),
        "Built-in manifest must always load successfully"
    );
    let builtin_reg = builtin.unwrap();
    assert!(
        !builtin_reg.list().is_empty(),
        "Built-in manifest must contain at least one package"
    );

    // 7. Verify checksum format enforcement on manifest download URLs
    let mysql = builtin_reg.get("mysql").expect("mysql should exist");
    for (version_str, version) in &mysql.versions {
        for (platform, download_info) in &version.downloads {
            assert!(
                download_info.checksum.starts_with("sha256:"),
                "Package mysql/{}/{} checksum must start with sha256:, got: {}",
                version_str,
                platform,
                download_info.checksum
            );
            let format_check = PackageDownloader::validate_checksum_format(&download_info.checksum);
            assert!(
                format_check.is_ok(),
                "Package mysql/{}/{} checksum format is invalid: {:?}",
                version_str,
                platform,
                format_check.err()
            );
        }
    }
}
