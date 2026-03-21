# Phase 5: Comprehensive Testing + Documentation Plan

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete comprehensive integration testing and documentation for the package manager + auto-updater system.

**Deliverables:**
1. E2E integration tests (realistic multi-step workflows)
2. Security validation tests (edge cases, path traversal, checksum failures)
3. User documentation (CLI guide, examples, troubleshooting)
4. API documentation (module references, type docs)

**Tech Stack:** Rust tests (tokio async, tempfile for isolation), markdown documentation

---

## Pre-Implementation Setup

**Verify Phase 4 Complete:**
```bash
cargo build --release 2>&1 | tail -5
cargo test --lib 2>&1 | grep "test result"
# Expected: All tests pass, release build succeeds
```

**Files to Review (Context):**
- `crates/cleanserve-core/tests/` - Existing integration test patterns
- `docs/guide/` - Documentation structure and style
- `src/main.rs` - CLI commands layout

---

## Task 1: E2E Integration Tests (Multi-Step Workflows)

**Files:**
- Modify: `crates/cleanserve-core/tests/integration_package_manager.rs` (add scenarios)
- Create: `crates/cleanserve-core/tests/integration_e2e_workflows.rs` (new comprehensive tests)

**Scope:**
- Scenario 1: Add package → verify manifest → download → checksum validation → cache hit
- Scenario 2: Start package → check runtime status → verify port allocation → stop
- Scenario 3: Check update → verify version comparison → simulate download
- Scenario 4: Multiple packages lifecycle (add redis + mysql, start both, verify isolation)
- Scenario 5: Proxy integration (package running → health check → status reporting)

**Test Structure:**
```rust
#[tokio::test]
async fn e2e_package_add_download_cache() { }

#[tokio::test]
async fn e2e_package_lifecycle_start_stop() { }

#[tokio::test]
async fn e2e_auto_updater_check_version() { }

#[tokio::test]
async fn e2e_multiple_packages_concurrent() { }

#[tokio::test]
async fn e2e_proxy_integration_health_check() { }
```

**Expected Output:**
- 5 new E2E tests
- All passing
- ~300 lines of test code

---

## Task 2: Security Validation Tests (Edge Cases & Threats)

**Files:**
- Create: `crates/cleanserve-core/tests/integration_security_validation.rs`

**Test Scenarios:**

1. **Path Traversal Protection:**
   - Attempt: `../../../etc/passwd` in package path
   - Expected: Blocked, error returned

2. **Checksum Mismatch:**
   - Download with wrong SHA256
   - Expected: Rejection, rollback, cleanup

3. **Malicious Binary Download:**
   - Simulate corrupted binary
   - Expected: Integrity check fails, not executed

4. **Permission Restrictions:**
   - Verify ~/.cleanserve/ created with 0700
   - Verify binaries created with 0755
   - Expected: Only owner can read/write

5. **Race Condition (Concurrent Downloads):**
   - Start 3 concurrent package downloads
   - Expected: Mutual exclusion, single write, cache reuse

6. **Disk Space Exhaustion:**
   - Simulate low disk space during download
   - Expected: Graceful failure, cleanup

7. **Invalid Manifest URLs:**
   - Attempt to load from malicious source
   - Expected: Certificate validation, safe rejection

**Test Structure:**
```rust
#[tokio::test]
async fn security_path_traversal_blocked() { }

#[tokio::test]
async fn security_checksum_mismatch_rejected() { }

#[tokio::test]
async fn security_malicious_binary_rejected() { }

#[tokio::test]
async fn security_permissions_enforced() { }

#[tokio::test]
async fn security_concurrent_downloads_safe() { }

#[tokio::test]
async fn security_disk_exhaustion_graceful() { }

#[tokio::test]
async fn security_invalid_manifest_rejected() { }
```

**Expected Output:**
- 7 security tests
- All passing
- ~400 lines of test code
- No regressions in existing tests

---

## Task 3: User Documentation (CLI Guide + Examples)

**Files:**
- Create: `docs/guide/package-manager.md` (user-facing CLI guide)
- Create: `docs/guide/auto-updater.md` (update workflow)
- Modify: `docs/guide/cli-commands.md` (add package/update commands)

**Content for `docs/guide/package-manager.md`:**

```markdown
# Package Manager Guide

## Overview
The package manager allows you to download and manage development tools like MySQL, Redis, and phpMyAdmin.

## Quick Start

### List Available Packages
```bash
cleanserve package list
```

### Get Package Info
```bash
cleanserve package info mysql
```

### Add a Package to Your Project
```bash
cleanserve package add mysql 8.0
```

### Start a Package
```bash
cleanserve package start mysql
```

### Check Package Status
```bash
cleanserve package status mysql
```

### Stop a Package
```bash
cleanserve package stop mysql
```

## Available Packages

- **mysql** (versions: 8.0.x, 8.1.x) - MySQL database server
- **redis** (versions: 7.0.x, 7.2.x) - In-memory data store
- **phpmyadmin** (versions: 5.2.x) - MySQL web interface

## Examples

### Setup a Full Stack
```bash
# Add packages
cleanserve package add mysql 8.0
cleanserve package add redis 7.0
cleanserve package add phpmyadmin 5.2

# Start services
cleanserve package start mysql
cleanserve package start redis

# Verify they're running
cleanserve package status
```

### Verify Downloaded Tools
```bash
ls ~/.cleanserve/tools/
# mysql/
# redis/
# phpmyadmin/
```

## Troubleshooting

### Package Won't Start
- Check if port is already in use: `lsof -i :3306`
- View logs: `cleanserve package status mysql` (shows error details)
- Verify binary: `~/.cleanserve/tools/mysql/bin/mysqld --version`

### Checksum Validation Failed
- Network issue during download? Retry: `cleanserve package add mysql 8.0 --force`
- Or delete cached version: `rm -rf ~/.cleanserve/tools/mysql/`

### Permission Denied
- Verify ~/.cleanserve/ ownership: `ls -la ~/ | grep cleanserve`
- Fix permissions: `chmod 0700 ~/.cleanserve/`
```

**Content for `docs/guide/auto-updater.md`:**

```markdown
# Auto-Updater Guide

## Overview
CleanServe can automatically check for and install the latest releases from GitHub.

## Commands

### Check for Updates
```bash
cleanserve update --check
```

Output:
```
Current version: 0.1.0
Latest version: 0.2.0
Update available! Run: cleanserve update --force
```

### Install Update
```bash
cleanserve update --force
```

This will:
1. Backup current binary to ~/.cleanserve/backups/cleanserve-0.1.0
2. Download latest release
3. Verify checksum
4. Install new binary
5. Verify success

### Force Update (Same Version)
```bash
cleanserve update --force
```

Useful for reinstalling or testing the update process.

## What Happens During Update

1. **Check Phase:** Fetch latest release from GitHub
2. **Download Phase:** Stream binary with progress indicator
3. **Verify Phase:** Checksum validation (SHA256)
4. **Backup Phase:** Save current binary to ~/.cleanserve/backups/
5. **Install Phase:** Replace binary atomically
6. **Verify Phase:** Test new binary works

## Rollback

If something goes wrong, the previous binary is backed up:

```bash
ls ~/.cleanserve/backups/
# cleanserve-0.1.0
# cleanserve-0.1.1

# Manually restore if needed
cp ~/.cleanserve/backups/cleanserve-0.1.0 $(which cleanserve)
```

## Automated Updates (install.sh)

The installer supports unattended updates:

```bash
# Install with auto-update enabled
curl -fsSL https://raw.githubusercontent.com/LyeZinho/cleanserve/main/install.sh | \
  CLEANSERVE_AUTO_UPDATE=1 sh
```

This will check for updates on each `cleanserve` invocation and prompt for installation.

## Troubleshooting

### Update Stuck/Slow
- Network issue. Retry: `cleanserve update --check`
- Check internet connection

### Checksum Mismatch
- Corrupted download. Try again: `cleanserve update --force`
- If persistent, report issue on GitHub

### Permission Denied During Install
- Binary location not writable. Check: `ls -la $(which cleanserve)`
- May need `sudo`: `sudo cleanserve update --force`
```

**Expected Output:**
- 3 new markdown files
- ~600 lines of documentation
- CLI command reference updated
- Examples with expected output

---

## Task 4: API Documentation (Module Docs + Type References)

**Files:**
- Modify: `crates/cleanserve-core/src/package_manager/mod.rs` (add doc comments)
- Modify: `crates/cleanserve-core/src/auto_updater/mod.rs` (add doc comments)
- Create: `docs/api/package-manager-api.md` (reference)
- Create: `docs/api/auto-updater-api.md` (reference)

**Rust Doc Comments (add to each public type):**

```rust
/// Package metadata from the manifest.
///
/// # Example
/// ```rust
/// let pkg = Package::new("mysql", "8.0.34");
/// ```
pub struct Package { }

/// Download information for a specific platform.
///
/// Contains checksum validation and URL information.
pub struct DownloadInfo { }

/// Runtime state of a running package.
///
/// Tracks process ID, port allocation, and status transitions.
pub struct PackageRuntime { }
```

**Content for `docs/api/package-manager-api.md`:**

```markdown
# Package Manager API Reference

## Core Types

### Package
```rust
pub struct Package {
    pub name: String,
    pub description: String,
    pub versions: HashMap<String, PackageVersion>,
}
```

### PackageVersion
```rust
pub struct PackageVersion {
    pub downloads: HashMap<String, DownloadInfo>,
    pub executable: Option<String>,
    pub requires: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub default_port: Option<u16>,
    pub health_check: Option<String>,
    pub proxy_path: Option<String>,
    pub server_type: Option<String>,
}
```

### DownloadInfo
```rust
pub struct DownloadInfo {
    pub url: String,
    pub checksum: String,
    pub checksum_type: String,
}
```

### PackageRuntime
```rust
pub struct PackageRuntime {
    pub package_name: String,
    pub version: String,
    pub pid: Option<u32>,
    pub port: u16,
    pub status: RuntimeStatus,
}

pub enum RuntimeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}
```

## Modules

### registry
Load built-in and custom package manifests.

### downloader
Download packages with checksum validation.

### cache
Manage local package cache (~/.cleanserve/tools/).

### lifecycle
Start/stop/monitor running packages.

## Usage Example

```rust
use cleanserve_core::package_manager::{PackageRegistry, PackageDownloader};

// Load packages
let registry = PackageRegistry::load()?;

// Find package
let mysql = registry.get_package("mysql")?;

// Download
let downloader = PackageDownloader::new();
downloader.download(&mysql.versions["8.0.34"], "/path/to/cache").await?;
```
```

**Content for `docs/api/auto-updater-api.md`:**

```markdown
# Auto-Updater API Reference

## Core Types

### UpdateInfo
```rust
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub checksum_url: String,
    pub needs_update: bool,
}
```

### UpdateCheckerError
```rust
pub enum UpdateCheckerError {
    NetworkError(String),
    InvalidVersion(String),
    ChecksumMismatch,
    InstallationFailed(String),
}
```

## Modules

### checker
Check GitHub for latest releases and compare versions.

### downloader
Download release binaries with platform detection.

### installer
Manage backup, installation, and rollback operations.

## Usage Example

```rust
use cleanserve_core::auto_updater::{UpdateChecker, BinaryDownloader, UpdateInstaller};

// Check for updates
let checker = UpdateChecker::new();
let update_info = checker.check_for_updates("0.1.0").await?;

if update_info.needs_update {
    // Download new binary
    let downloader = BinaryDownloader::new();
    let binary_path = downloader.download(&update_info).await?;
    
    // Install it
    let installer = UpdateInstaller::new();
    installer.install(&binary_path).await?;
}
```
```

**Expected Output:**
- 4 documentation files
- ~500 lines of reference docs
- Full type/method signatures
- Usage examples in each module
- `cargo doc` builds cleanly

---

## Task 5: Final Verification & Integration

**Files:**
- No new files; verify all changes

**Steps:**

1. **Run All Tests:**
   ```bash
   cargo test --lib 2>&1 | grep "test result"
   cargo test --test integration_* 2>&1 | tail -20
   ```
   Expected: All tests pass

2. **Build Release:**
   ```bash
   cargo build --release 2>&1 | tail -5
   ```
   Expected: No errors

3. **Generate Docs:**
   ```bash
   cargo doc --no-deps 2>&1 | tail -5
   ```
   Expected: Documentation generated

4. **CLI Smoke Tests:**
   ```bash
   ./target/release/cleanserve package list
   ./target/release/cleanserve package info mysql
   ./target/release/cleanserve update --check
   ```
   Expected: All commands work

5. **Create Summary Commit:**
   ```bash
   git add -A
   git commit -m "Phase 5: Add comprehensive testing + documentation

   - E2E integration tests (5 scenarios)
   - Security validation tests (7 edge cases)
   - User documentation (package manager, auto-updater guides)
   - API reference documentation
   - All tests passing, release build successful"
   ```

**Expected Output:**
- All tests pass (150+ total)
- Release build succeeds
- Documentation builds without warnings
- CLI commands functional
- Summary commit created

---

## Success Criteria

- [ ] 5 E2E integration tests passing
- [ ] 7 security validation tests passing
- [ ] 3 user documentation files created
- [ ] 2 API reference files created
- [ ] All existing tests still passing
- [ ] Release build successful
- [ ] `cargo doc` clean
- [ ] Summary commit created

---

## Post-Phase 5

Ready for Phase 4b: Full GitHub API Auto-Updater Implementation
- Real GitHub API integration
- Actual binary downloads
- Production-grade error handling
- Full rollback/recovery
