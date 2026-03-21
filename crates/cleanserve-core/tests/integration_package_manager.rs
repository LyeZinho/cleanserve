use cleanserve_core::package_manager::{PackageRegistry, PackageDownloader, PackageCache};

#[tokio::test]
async fn test_registry_loads_builtin() {
    let registry = PackageRegistry::with_builtin().expect("Failed to load registry");
    let packages = registry.list();
    assert!(!packages.is_empty(), "Registry should have packages");
}

#[tokio::test]
async fn test_registry_finds_mysql() {
    let registry = PackageRegistry::with_builtin().expect("Failed to load registry");
    assert!(registry.get("mysql").is_some(), "MySQL should be in registry");
}

#[tokio::test]
async fn test_registry_finds_redis() {
    let registry = PackageRegistry::with_builtin().expect("Failed to load registry");
    assert!(registry.get("redis").is_some(), "Redis should be in registry");
}

#[test]
fn test_cache_paths() {
    let root = PackageCache::root().expect("Cache root should exist");
    assert!(root.to_string_lossy().contains(".cleanserve"));

    let pkg_path = PackageCache::package_path("mysql", "8.0")
        .expect("Package path should be valid");
    assert!(pkg_path.to_string_lossy().contains("mysql"));
    assert!(pkg_path.to_string_lossy().contains("8.0"));
}

#[test]
fn test_checksum_validation_format() {
    let valid = PackageDownloader::validate_checksum_format(
        "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    );
    assert!(valid.is_ok());

    let invalid = PackageDownloader::validate_checksum_format("invalid:format");
    assert!(invalid.is_err());
}

#[test]
fn test_platform_detection() {
    let platform = PackageDownloader::get_platform();
    assert!(!platform.is_empty());
    assert!(platform.contains('-'));
}
