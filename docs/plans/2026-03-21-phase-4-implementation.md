# Phase 4: Auto-Updater Implementation Plan

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement self-updating functionality so CleanServe can check for and install the latest release from GitHub.

**Architecture:** Phase 4 adds auto-update capability: (1) UpdateChecker queries GitHub API for latest release; (2) BinaryDownloader fetches + validates binary; (3) Installation handler manages backup/restore; (4) install.sh updated for unattended upgrades.

**Tech Stack:** Rust (reqwest for HTTP, semver for version comparison), GitHub API integration, atomic file operations.

---

## Pre-Implementation Setup

**Verify Phase 3 Complete:**
```bash
cargo test --lib package_manager 2>&1 | grep "test result" | head -1
# Expected: ok. 6 passed (runtime + lifecycle tests)
```

**Files to Review (Context):**
- `crates/cleanserve-cli/src/main.rs` - Where to hook update command
- `Cargo.toml` - Dependencies (reqwest, semver already present)
- `install.sh` - Shell script to update

---

## Task 1: Create UpdateChecker for GitHub API Integration

**Files:**
- Create: `crates/cleanserve-core/src/auto_updater/mod.rs`
- Create: `crates/cleanserve-core/src/auto_updater/checker.rs`
- Modify: `crates/cleanserve-core/src/lib.rs` (add auto_updater module)

**Step 1: Create module structure**

Create `crates/cleanserve-core/src/auto_updater/mod.rs`:

```rust
pub mod checker;

pub use checker::UpdateChecker;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub checksum_url: String,
    pub needs_update: bool,
}

#[derive(Debug)]
pub struct UpdateCheckerError {
    pub message: String,
}

impl std::fmt::Display for UpdateCheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Update error: {}", self.message)
    }
}

impl std::error::Error for UpdateCheckerError {}

pub type Result<T> = std::result::Result<T, UpdateCheckerError>;
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

**Step 2: Create checker.rs**

Create `crates/cleanserve-core/src/auto_updater/checker.rs`:

```rust
use super::{Result, UpdateInfo, UpdateCheckerError};
use semver::Version;

pub struct UpdateChecker;

impl UpdateChecker {
    pub async fn check_for_updates(current_version: &str) -> Result<UpdateInfo> {
        let latest = Self::fetch_latest_release().await?;
        
        let current = Version::parse(current_version)
            .map_err(|e| UpdateCheckerError {
                message: format!("Invalid current version: {}", e),
            })?;
        
        let latest_version = Version::parse(&latest)
            .map_err(|e| UpdateCheckerError {
                message: format!("Invalid latest version from API: {}", e),
            })?;
        
        let needs_update = latest_version > current;
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest.clone(),
            download_url: format!(
                "https://github.com/LyeZinho/cleanserve/releases/download/v{}/cleanserve",
                latest
            ),
            checksum_url: format!(
                "https://github.com/LyeZinho/cleanserve/releases/download/v{}/SHA256SUMS",
                latest
            ),
            needs_update,
        })
    }

    async fn fetch_latest_release() -> Result<String> {
        // Placeholder: In Phase 4b, integrate with GitHub API
        // For now, return mock version
        Ok("0.3.1".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version_comparison() {
        let info = UpdateChecker::check_for_updates("0.3.0").await.unwrap();
        assert!(info.needs_update);
        assert_eq!(info.latest_version, "0.3.1");
    }

    #[tokio::test]
    async fn test_no_update_needed() {
        let info = UpdateChecker::check_for_updates("0.3.1").await.unwrap();
        assert!(!info.needs_update);
    }
}
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

**Step 3: Add module to lib.rs**

Edit `crates/cleanserve-core/src/lib.rs`:

Find the modules section and add:
```rust
pub mod auto_updater;
```

Run: `cargo test --lib auto_updater 2>&1 | grep "test result"`
Expected: `ok. 2 passed`.

**Step 4: Commit**

```bash
git add crates/cleanserve-core/src/auto_updater/ crates/cleanserve-core/src/lib.rs
git commit -m "feat: add UpdateChecker for GitHub API version comparison"
```

---

## Task 2: Create BinaryDownloader for Release Artifacts

**Files:**
- Create: `crates/cleanserve-core/src/auto_updater/downloader.rs`
- Modify: `crates/cleanserve-core/src/auto_updater/mod.rs` (add mod declaration)

**Step 1: Create downloader.rs**

Create `crates/cleanserve-core/src/auto_updater/downloader.rs`:

```rust
use super::{Result, UpdateCheckerError};
use std::path::Path;

pub struct BinaryDownloader;

impl BinaryDownloader {
    pub async fn download_release(
        download_url: &str,
        checksum_url: &str,
        dest_path: &Path,
    ) -> Result<()> {
        // Placeholder: In Phase 4b, implement actual download
        // For now, return success
        println!("Would download from: {}", download_url);
        println!("Would validate with: {}", checksum_url);
        Ok(())
    }

    pub fn validate_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
        // Placeholder: In Phase 4b, implement SHA256 validation
        println!("Would validate: {:?} against {}", file_path, expected_checksum);
        Ok(())
    }

    pub fn get_platform() -> String {
        // Return platform identifier for download URL
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        format!("{}-{}", os, arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = BinaryDownloader::get_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains('-'));
    }

    #[tokio::test]
    async fn test_download_placeholder() {
        let result = BinaryDownloader::download_release(
            "https://example.com/binary",
            "https://example.com/checksums",
            Path::new("/tmp/test"),
        ).await;
        assert!(result.is_ok());
    }
}
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

**Step 2: Add module to mod.rs**

Edit `crates/cleanserve-core/src/auto_updater/mod.rs`:

Add after `pub mod checker;`:
```rust
pub mod downloader;

pub use downloader::BinaryDownloader;
```

Run: `cargo test --lib auto_updater::downloader 2>&1 | grep "test result"`
Expected: `ok. 2 passed`.

**Step 3: Commit**

```bash
git add crates/cleanserve-core/src/auto_updater/downloader.rs crates/cleanserve-core/src/auto_updater/mod.rs
git commit -m "feat: add BinaryDownloader for release artifact management"
```

---

## Task 3: Create UpdateManager for Installation & Rollback

**Files:**
- Create: `crates/cleanserve-core/src/auto_updater/installer.rs`
- Modify: `crates/cleanserve-core/src/auto_updater/mod.rs` (add mod declaration)

**Step 1: Create installer.rs**

Create `crates/cleanserve-core/src/auto_updater/installer.rs`:

```rust
use super::{Result, UpdateCheckerError};
use std::path::{Path, PathBuf};

pub struct UpdateInstaller;

impl UpdateInstaller {
    pub fn backup_current_binary(binary_path: &Path) -> Result<PathBuf> {
        // Placeholder: In Phase 4b, implement backup logic
        let backup_dir = dirs::home_dir()
            .ok_or_else(|| UpdateCheckerError {
                message: "Cannot determine home directory".to_string(),
            })?
            .join(".cleanserve")
            .join("backups");
        
        std::fs::create_dir_all(&backup_dir).map_err(|e| UpdateCheckerError {
            message: format!("Failed to create backup dir: {}", e),
        })?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| UpdateCheckerError {
                message: format!("Time error: {}", e),
            })?
            .as_secs();

        let backup_path = backup_dir.join(format!("cleanserve.{}", timestamp));
        
        println!("Would backup {} to {}", binary_path.display(), backup_path.display());
        
        Ok(backup_path)
    }

    pub async fn install_binary(new_binary: &Path, target_location: &Path) -> Result<()> {
        // Placeholder: In Phase 4b, implement installation
        println!("Would install {} to {}", new_binary.display(), target_location.display());
        Ok(())
    }

    pub async fn verify_installation(binary_path: &Path) -> Result<()> {
        // Placeholder: In Phase 4b, run --version check
        println!("Would verify {}", binary_path.display());
        Ok(())
    }

    pub fn cleanup_old_backups() -> Result<()> {
        // Placeholder: In Phase 4b, keep last 3 backups
        println!("Would cleanup old backups");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let os = std::env::consts::OS;
        assert!(!os.is_empty());
    }

    #[tokio::test]
    async fn test_install_placeholder() {
        let result = UpdateInstaller::install_binary(
            Path::new("/tmp/new"),
            Path::new("/tmp/target"),
        ).await;
        assert!(result.is_ok());
    }
}
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors (may need to add `dirs` dependency).

If dirs dependency missing, add to `crates/cleanserve-core/Cargo.toml`:
```toml
dirs = "5.0"
```

**Step 2: Add module to mod.rs**

Edit `crates/cleanserve-core/src/auto_updater/mod.rs`:

Add after `pub mod downloader;`:
```rust
pub mod installer;

pub use installer::UpdateInstaller;
```

Run: `cargo test --lib auto_updater::installer 2>&1 | grep "test result"`
Expected: `ok. 2 passed`.

**Step 3: Commit**

```bash
git add crates/cleanserve-core/src/auto_updater/installer.rs crates/cleanserve-core/src/auto_updater/mod.rs crates/cleanserve-core/Cargo.toml
git commit -m "feat: add UpdateInstaller for binary backup and installation"
```

---

## Task 4: Add Update CLI Command

**Files:**
- Modify: `crates/cleanserve-cli/src/main.rs` (add Update command)
- Modify: `crates/cleanserve-cli/src/commands/update.rs` (create or update)

**Step 1: Add Update command to CLI**

Edit `crates/cleanserve-cli/src/main.rs`:

Find the Commands enum and add:

```rust
    /// Check for and install CleanServe updates
    Update {
        /// Only check without installing
        #[arg(long)]
        check: bool,
        /// Force update even if versions match
        #[arg(long)]
        force: bool,
    },
```

**Step 2: Add match handler**

In the match statement after other commands, add:

```rust
        Commands::Update { check, force } => {
            commands::update::run(check, force).await?;
        }
```

**Step 3: Create/update commands/update.rs**

Create or modify `crates/cleanserve-cli/src/commands/update.rs`:

```rust
use cleanserve_core::auto_updater::UpdateChecker;

pub async fn run(check_only: bool, force: bool) -> anyhow::Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    
    println!("Checking for updates...");
    
    let info = UpdateChecker::check_for_updates(current_version).await
        .map_err(|e| anyhow::anyhow!(e))?;
    
    println!("Current version: {}", info.current_version);
    println!("Latest version: {}", info.latest_version);
    
    if !info.needs_update && !force {
        println!("✓ Already up to date!");
        return Ok(());
    }
    
    if check_only {
        if info.needs_update {
            println!("✓ Update available: {} → {}", info.current_version, info.latest_version);
        }
        return Ok(());
    }
    
    // Placeholder: In Phase 4b, implement actual update
    println!("✓ Update would install: {}", info.latest_version);
    
    Ok(())
}
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

**Step 4: Commit**

```bash
git add crates/cleanserve-cli/src/main.rs crates/cleanserve-cli/src/commands/update.rs
git commit -m "feat: add update CLI command with version checking"
```

---

## Task 5: Integration Tests for Auto-Updater

**Files:**
- Create: `crates/cleanserve-core/tests/integration_auto_updater.rs`

**Step 1: Write integration tests**

Create `crates/cleanserve-core/tests/integration_auto_updater.rs`:

```rust
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
```

Run: `cargo test --test integration_auto_updater 2>&1 | grep "test result"`
Expected: `ok. 4 passed`.

**Step 2: Commit**

```bash
git add crates/cleanserve-core/tests/integration_auto_updater.rs
git commit -m "test: add integration tests for auto-updater"
```

---

## Task 6: Full Verification & Summary

**Step 1: Run all tests**

```bash
cargo test --lib 2>&1 | grep "test result" | tail -1
# Expected: all pass with auto_updater tests included
```

**Step 2: Verify release build**

```bash
cargo build --release 2>&1 | tail -3
# Expected: Finished successfully
```

**Step 3: Test new CLI command**

```bash
cargo run --release -- update --check 2>&1 | tail -3
# Expected: Shows version check output
```

**Step 4: Verify git history**

```bash
git log --oneline | head -10
# Expected: 5 new commits for Phase 4
```

---

## Phase 4b (Future): Full Implementation

These tasks are placeholders for Phase 4b:

1. **Implement GitHub API integration** in UpdateChecker
   - Use GitHub releases API
   - Add retries and error handling
   - Cache version info locally

2. **Implement actual binary download** in BinaryDownloader
   - Download from GitHub releases
   - Stream to temp file
   - Validate SHA256 checksum

3. **Implement installation** in UpdateInstaller
   - Atomic file operations
   - Backup current binary
   - Set executable permissions
   - Verify post-install

4. **Update install.sh**
   - Add check for existing binary
   - Backup before removing
   - Update to use new auto-update mechanism

5. **Production testing**
   - Test actual GitHub API integration
   - Test rollback scenarios
   - Test permission handling

---

## Execution Handoff

**Plan complete and saved to `/home/pedro/repo/cleanserve/docs/plans/2026-03-21-phase-4-implementation.md`.**

**Ready to execute: Direct implementation approach (5 tasks, atomic commits expected).**
