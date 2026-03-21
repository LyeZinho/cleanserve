# Version Management Redesign

## Context

CleanServe's PHP version management currently scrapes HTML from `dl.static-php.dev` to discover versions and constructs download URLs manually. This is fragile and the binaries are large.

The new approach uses a curated manifest (`versions.json`) from `LyeZinho/php-runtimes` with pre-built ultra-lightweight PHP binaries (~4-5MB). Linux-only for now.

## Source

- **Manifest URL**: `https://raw.githubusercontent.com/LyeZinho/php-runtimes/main/manifests/versions.json`
- **Versions available**: 37 (PHP 8.3.19-8.3.30, 8.4.0-8.4.19, 8.5.0-8.5.4)
- **Binary sizes**: ~4.1MB (8.3/8.4), ~5.5MB (8.5)
- **Integrity**: SHA256 checksums per binary

## Design

### 1. New module: `version_manifest.rs`

Handles fetch, cache, and query of the remote manifest.

**Structs:**

```rust
pub struct VersionManifest {
    pub schema_version: String,
    pub updated_at: String,
    pub versions: Vec<PhpVersion>,
}

pub struct PhpVersion {
    pub version: String,       // "8.4.19"
    pub tag: String,           // "v8.4.19"
    pub published_at: String,
    pub platforms: Vec<PlatformBinary>,
}

pub struct PlatformBinary {
    pub platform: String,      // "linux"
    pub filename: String,
    pub download_url: String,
    pub size_bytes: u64,
    pub sha256: String,
}
```

**Cache behavior:**

- Stored at `~/.cleanserve/cache/versions.json` + `~/.cleanserve/cache/manifest.meta` (timestamp)
- TTL: 1 hour. After that, re-fetch on next access
- `--refresh` flag forces immediate re-fetch
- Offline fallback: if fetch fails and cache exists, use cache with warning

**Query helpers:**

- `find_version("8.4")` - resolves minor to latest patch (e.g., 8.4.19)
- `find_exact("8.4.19")` - exact match
- `list_available()` - all versions
- `get_platform_binary("8.4.19", "linux")` - returns PlatformBinary for download

### 2. Rewrite: `php_downloader.rs`

Replace HTML scraping and hardcoded URLs with manifest-driven downloads.

**New download flow:**

1. `manifest.fetch_or_cache()`
2. `manifest.find_version(version)` resolves "8.4" to "8.4.19"
3. `manifest.get_platform_binary(resolved, current_platform)` gets URL + SHA256
4. Early return if already installed
5. Download from `binary.download_url`
6. **Mandatory SHA256 verification** - fail if mismatch
7. Extract tar.gz to `.cleanserve/php/php-{version}/`
8. `chmod +x` on the binary
9. Cleanup temp file

**API preserved** (no breaking changes to callers):

- `PhpDownloader::new(base_dir)` - unchanged
- `download(&self, version: &str)` - same signature, new behavior
- `is_installed()`, `get_php_exe()`, `get_install_path()` - unchanged

**Removed:**

- `find_latest_patch_version()` - replaced by manifest query
- Hardcoded `dl.static-php.dev` URL
- Hardcoded `windows.php.net` URL
- Windows download block (`#[cfg(windows)]` on download method)

### 3. Update: `list.rs` CLI command

**New output format:**

```
Available PHP versions:
  8.5.4        (5.5 MB)
  8.5.3        (5.5 MB)
  8.4.19       (4.1 MB)  * installed
  8.3.30       (3.9 MB)

Installed: 1 | Available: 37
```

- Shows all available versions from manifest (newest first)
- Marks installed versions
- Shows download size in human-readable format
- `cleanserve list --refresh` forces manifest re-fetch
- `cleanserve list --installed` filters to installed only

### 4. Minor update: `use_.rs`

If version not installed, auto-download it instead of just telling the user to run `cleanserve update`.

## Files Changed

| File | Action |
|------|--------|
| `crates/cleanserve-core/src/version_manifest.rs` | **NEW** - manifest fetch/cache/query |
| `crates/cleanserve-core/src/php_downloader.rs` | **REWRITE** - manifest-driven downloads |
| `crates/cleanserve-core/src/lib.rs` | **EDIT** - export new module |
| `crates/cleanserve-core/Cargo.toml` | **EDIT** - add sha2 dependency |
| `crates/cleanserve-cli/src/commands/list.rs` | **REWRITE** - show remote + installed |
| `crates/cleanserve-cli/src/commands/use_.rs` | **EDIT** - auto-download if missing |
| `crates/cleanserve-cli/src/main.rs` | **EDIT** - add --refresh/--installed flags to List |

## Non-Goals

- macOS/Windows support (manifest only has Linux binaries currently)
- Multiple download sources/fallbacks
- Version pinning beyond cleanserve.json
