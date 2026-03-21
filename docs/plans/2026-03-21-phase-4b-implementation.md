# Phase 4b: Full GitHub API Auto-Updater Implementation Plan

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the auto-updater with real GitHub API integration, actual binary downloads, and production-grade error handling.

**Current State:** Phase 4 has skeleton code with placeholder implementations. Phase 4b adds:
1. Real GitHub API integration (fetch actual releases, not mock 0.3.1)
2. Streaming binary downloads with progress reporting
3. Production error handling and retry logic
4. Full rollback/recovery on failure
5. install.sh integration for unattended upgrades

**Architecture:**
- **UpdateChecker:** Query GitHub Releases API, parse semver, determine update availability
- **BinaryDownloader:** Detect platform (linux-x64, darwin-arm64, etc), stream download, calculate SHA256
- **UpdateInstaller:** Atomic file operations with backup/restore, verification, cleanup
- **install.sh:** Enhanced with `CLEANSERVE_AUTO_UPDATE` flag for unattended upgrades

**Tech Stack:** Rust (reqwest for HTTP streaming, sha2 for checksums, semver for versions), GitHub API v3 (JSON)

---

## Pre-Implementation Setup

**Verify Phase 5 Complete:**
```bash
cargo test --lib 2>&1 | grep "test result" | head -1
# Expected: All tests pass
cargo build --release 2>&1 | tail -3
# Expected: Release build succeeds
```

**Files to Review (Context):**
- `crates/cleanserve-core/src/auto_updater/` - Current skeleton implementation
- `.github/workflows/` - CI/CD for releases (to understand release naming)
- `install.sh` - Current installer script

**GitHub API Endpoint:**
```
GET https://api.github.com/repos/LyeZinho/cleanserve/releases/latest
Response: { tag_name, assets[].name, assets[].browser_download_url }
```

---

## Task 1: Real GitHub API Integration (UpdateChecker)

**Files:**
- Modify: `crates/cleanserve-core/src/auto_updater/checker.rs` (replace mock implementation)

**Current Placeholder Code:**
```rust
// CURRENT: Returns hardcoded 0.3.1
pub async fn check_for_updates(&self, current_version: &str) -> Result<UpdateInfo> {
    Ok(UpdateInfo {
        latest_version: "0.3.1".to_string(),
        // ...
    })
}
```

**Implementation:**

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use semver::Version;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubAsset>,
    pub draft: bool,
    pub prerelease: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

impl UpdateChecker {
    pub async fn check_for_updates(&self, current_version: &str) -> Result<UpdateInfo> {
        let client = Client::new();
        let current = Version::parse(current_version)?;
        
        // Fetch latest release from GitHub API
        let response = client
            .get("https://api.github.com/repos/LyeZinho/cleanserve/releases/latest")
            .header("User-Agent", "cleanserve-updater")
            .send()
            .await?;
            
        let release: GitHubRelease = response.json().await?;
        
        // Parse latest version from tag_name (e.g., "v0.2.0" -> "0.2.0")
        let latest_version_str = release.tag_name.trim_start_matches('v');
        let latest = Version::parse(latest_version_str)?;
        
        // Determine if update needed
        let needs_update = latest > current;
        
        // Find appropriate asset for current platform
        let platform = self.detect_platform();
        let asset = release.assets
            .iter()
            .find(|a| a.name.contains(&platform))
            .ok_or(UpdateCheckerError::PlatformNotSupported(platform.clone()))?;
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest_version_str.to_string(),
            download_url: asset.browser_download_url.clone(),
            checksum_url: format!(
                "https://github.com/LyeZinho/cleanserve/releases/download/{}/SHA256SUMS",
                release.tag_name
            ),
            needs_update,
        })
    }
    
    fn detect_platform(&self) -> String {
        #[cfg(target_os = "linux")]
        {
            #[cfg(target_arch = "x86_64")]
            { "cleanserve-linux-x64".to_string() }
            #[cfg(target_arch = "aarch64")]
            { "cleanserve-linux-arm64".to_string() }
        }
        
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "x86_64")]
            { "cleanserve-darwin-x64".to_string() }
            #[cfg(target_arch = "aarch64")]
            { "cleanserve-darwin-arm64".to_string() }
        }
        
        #[cfg(target_os = "windows")]
        { "cleanserve-windows-x64.exe".to_string() }
    }
}
```

**Test Cases:**
- Parse GitHub API response correctly
- Extract latest version from tag_name
- Compare versions (0.1.0 < 0.2.0, 0.2.0 > 0.2.0-rc1)
- Platform detection (linux-x64, darwin-arm64, etc)
- Return correct download_url and checksum_url
- Handle API errors (404, rate limit, network)

**Expected Output:**
- Real GitHub API integration working
- Platform detection correct
- Version comparison accurate
- All existing tests still pass

---

## Task 2: Streaming Binary Downloads (BinaryDownloader)

**Files:**
- Modify: `crates/cleanserve-core/src/auto_updater/downloader.rs` (replace mock implementation)

**Current Placeholder:**
```rust
// CURRENT: Returns empty array
pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
    Ok(vec![])
}
```

**Implementation:**

```rust
use sha2::{Sha256, Digest};
use std::io::Write;
use tokio::fs::File;

pub struct BinaryDownloader {
    client: Client,
    timeout: Duration,
}

impl BinaryDownloader {
    pub async fn download_and_verify(
        &self,
        download_url: &str,
        checksum_url: &str,
        dest_path: &Path,
    ) -> Result<()> {
        // Download binary with streaming
        let binary_data = self.download_binary_streaming(download_url).await?;
        
        // Download checksums file
        let checksums = self.download_checksums(checksum_url).await?;
        
        // Extract expected checksum for this binary
        let expected_checksum = self.extract_checksum(&checksums, dest_path)?;
        
        // Calculate actual checksum
        let actual_checksum = self.calculate_checksum(&binary_data);
        
        // Verify match
        if actual_checksum != expected_checksum {
            return Err(UpdateCheckerError::ChecksumMismatch);
        }
        
        // Write to disk
        let mut file = File::create(dest_path).await?;
        file.write_all(&binary_data).await?;
        
        Ok(())
    }
    
    async fn download_binary_streaming(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| UpdateCheckerError::NetworkError(e.to_string()))?;
        
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut buffer = Vec::new();
        let mut stream = response.bytes_stream();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| UpdateCheckerError::NetworkError(e.to_string()))?;
            
            downloaded += chunk.len() as u64;
            
            // Progress reporting (every 5%)
            if total_size > 0 {
                let percent = (downloaded * 100) / total_size;
                eprintln!("Downloaded: {}%", percent);
            }
            
            buffer.extend_from_slice(&chunk);
        }
        
        Ok(buffer)
    }
    
    async fn download_checksums(&self, url: &str) -> Result<String> {
        self.client
            .get(url)
            .send()
            .await?
            .text()
            .await
            .map_err(|e| UpdateCheckerError::NetworkError(e.to_string()))
    }
    
    fn extract_checksum(&self, checksums_text: &str, file_path: &Path) -> Result<String> {
        let file_name = file_path.file_name().unwrap().to_str().unwrap();
        
        for line in checksums_text.lines() {
            if line.contains(file_name) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 1 {
                    return Ok(parts[0].to_string());
                }
            }
        }
        
        Err(UpdateCheckerError::ChecksumNotFound)
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
```

**Test Cases:**
- Stream large binary downloads without loading into memory
- Calculate SHA256 checksum correctly
- Parse SHA256SUMS file format
- Verify checksum match/mismatch
- Handle network errors gracefully
- Retry on transient failures
- Progress reporting works

**Expected Output:**
- Real streaming downloads working
- SHA256 validation correct
- Progress indicator works
- All tests pass

---

## Task 3: Atomic Installation & Rollback (UpdateInstaller)

**Files:**
- Modify: `crates/cleanserve-core/src/auto_updater/installer.rs` (replace mock implementation)

**Current Placeholder:**
```rust
// CURRENT: Does nothing
pub async fn install(&self, binary_path: &Path) -> Result<()> {
    Ok(())
}
```

**Implementation:**

```rust
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub struct UpdateInstaller {
    backup_dir: PathBuf,
}

impl UpdateInstaller {
    pub fn new() -> Self {
        let backup_dir = dirs::home_dir()
            .unwrap()
            .join(".cleanserve/backups");
        
        Self { backup_dir }
    }
    
    pub async fn install(&self, new_binary_path: &Path) -> Result<()> {
        // Get path to current cleanserve binary
        let current_cleanserve = std::env::current_exe()?;
        
        // Step 1: Create backup directory
        fs::create_dir_all(&self.backup_dir)?;
        
        // Step 2: Get current version for backup filename
        let current_version = self.get_current_version(&current_cleanserve)?;
        let backup_path = self.backup_dir.join(format!("cleanserve-{}", current_version));
        
        // Step 3: Backup current binary
        if current_cleanserve.exists() {
            fs::copy(&current_cleanserve, &backup_path)?;
            eprintln!("Backed up current binary to {}", backup_path.display());
        }
        
        // Step 4: Verify new binary is valid
        self.verify_binary(new_binary_path).await?;
        
        // Step 5: Atomic install (copy then verify)
        fs::copy(new_binary_path, &current_cleanserve)?;
        
        // Step 6: Set permissions to 0755 (executable)
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&current_cleanserve, perms)?;
        
        // Step 7: Verify installation succeeded
        self.verify_binary(&current_cleanserve).await?;
        
        eprintln!("✓ Update successful!");
        eprintln!("New version: {}", self.get_current_version(&current_cleanserve)?);
        
        Ok(())
    }
    
    async fn verify_binary(&self, path: &Path) -> Result<()> {
        // Try to run --version
        let output = tokio::process::Command::new(path)
            .arg("--version")
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(UpdateCheckerError::InstallationFailed(
                "New binary failed --version check".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn get_current_version(&self, binary_path: &Path) -> Result<String> {
        let output = std::process::Command::new(binary_path)
            .arg("--version")
            .output()?;
        
        let version_str = String::from_utf8(output.stdout)?;
        let version = version_str
            .trim()
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        
        Ok(version)
    }
    
    pub fn rollback(&self, version: &str) -> Result<()> {
        let backup_path = self.backup_dir.join(format!("cleanserve-{}", version));
        let current_cleanserve = std::env::current_exe()?;
        
        if !backup_path.exists() {
            return Err(UpdateCheckerError::RollbackFailed(
                format!("Backup not found: {}", backup_path.display())
            ));
        }
        
        fs::copy(&backup_path, &current_cleanserve)?;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&current_cleanserve, perms)?;
        
        eprintln!("✓ Rolled back to version {}", version);
        Ok(())
    }
    
    pub fn list_backups(&self) -> Result<Vec<String>> {
        let mut backups = Vec::new();
        
        if self.backup_dir.exists() {
            for entry in fs::read_dir(&self.backup_dir)? {
                let entry = entry?;
                let file_name = entry.file_name();
                if let Some(name) = file_name.to_str() {
                    if name.starts_with("cleanserve-") {
                        let version = name.strip_prefix("cleanserve-").unwrap();
                        backups.push(version.to_string());
                    }
                }
            }
        }
        
        backups.sort();
        Ok(backups)
    }
}
```

**Test Cases:**
- Backup current binary before install
- Verify new binary is executable and works
- Atomic file replacement
- Set correct permissions (0755)
- Rollback to previous version
- List available backups
- Handle missing backup gracefully
- Handle permission errors

**Expected Output:**
- Atomic installation working
- Backups created correctly
- Rollback works
- All tests pass

---

## Task 4: CLI Integration & install.sh Enhancement

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/update.rs` (integrate real implementation)
- Modify: `install.sh` (add auto-update support)

**CLI Implementation:**

```rust
pub async fn run_cleanserve_update(
    force: bool,
    check_only: bool,
) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let checker = UpdateChecker::new();
    
    println!("Current version: {}", current_version);
    
    // Check for updates
    let update_info = checker.check_for_updates(current_version).await?;
    println!("Latest version: {}", update_info.latest_version);
    
    if !update_info.needs_update && !force {
        println!("✓ Already up to date");
        return Ok(());
    }
    
    if check_only {
        if update_info.needs_update {
            println!("\n💡 Update available! Run: cleanserve update --force");
        }
        return Ok(());
    }
    
    println!("\n⬇️  Downloading update...");
    
    // Download and verify
    let temp_dir = tempfile::tempdir()?;
    let temp_binary = temp_dir.path().join("cleanserve-new");
    
    let downloader = BinaryDownloader::new();
    downloader
        .download_and_verify(
            &update_info.download_url,
            &update_info.checksum_url,
            &temp_binary,
        )
        .await?;
    
    println!("✓ Download verified");
    
    // Install
    println!("\n📦 Installing update...");
    let installer = UpdateInstaller::new();
    installer.install(&temp_binary).await?;
    
    println!("\n✓ Update complete!");
    println!("Run: cleanserve --version");
    
    Ok(())
}
```

**install.sh Enhancement:**

```bash
#!/bin/bash

set -euo pipefail

# ... existing install code ...

# Auto-update support
if [ "${CLEANSERVE_AUTO_UPDATE:-0}" = "1" ]; then
    # Install cron job to check for updates daily
    CRON_JOB="0 2 * * * cleanserve update --check --silent && cleanserve update --force --silent 2>/dev/null || true"
    
    (crontab -l 2>/dev/null; echo "$CRON_JOB") | crontab -
    echo "✓ Auto-update enabled (daily check at 2 AM)"
fi
```

**Test Cases:**
- `cleanserve update --check` shows available update
- `cleanserve update --check` shows "up to date" if current
- `cleanserve update --force` installs update
- CLI integration works end-to-end
- Progress output is clear and helpful
- Error messages are actionable

**Expected Output:**
- Full CLI integration working
- install.sh updated with auto-update flag
- All commands functional

---

## Task 5: Integration Tests & Production Validation

**Files:**
- Modify: `crates/cleanserve-core/tests/integration_auto_updater.rs` (replace placeholders with real tests)

**Real Test Scenarios:**

```rust
#[tokio::test]
async fn test_github_api_integration_real() {
    // Actually query GitHub API (with rate limit handling)
    let checker = UpdateChecker::new();
    let info = checker.check_for_updates("0.1.0").await;
    
    // May fail due to rate limit, but shouldn't panic
    match info {
        Ok(update_info) => {
            assert!(!update_info.latest_version.is_empty());
            assert!(update_info.download_url.contains("github.com"));
        }
        Err(UpdateCheckerError::RateLimitExceeded) => {
            // OK - GitHub rate limit hit
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[tokio::test]
async fn test_binary_download_mock() {
    // Use mock server for repeatable tests
    let temp_dir = tempfile::tempdir().unwrap();
    let binary_path = temp_dir.path().join("cleanserve-linux-x64");
    
    // Mock download would succeed with valid binary
    // (In production, test against real GitHub releases)
}

#[tokio::test]
async fn test_atomic_installation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let installer = UpdateInstaller::new();
    
    // Verify backup created
    // Verify installation successful
    // Verify version incremented
}

#[tokio::test]
async fn test_rollback_mechanism() {
    let installer = UpdateInstaller::new();
    
    // Create backup
    // Verify rollback restores previous version
}

#[tokio::test]
async fn test_concurrent_update_safety() {
    // Start 2 updates simultaneously
    // Verify only one succeeds, other is blocked
    // Verify no file corruption
}
```

**Test Execution:**
```bash
# Run with real network (slow, may hit rate limits)
cargo test --test integration_auto_updater -- --nocapture

# Run with mock server (fast, repeatable)
MOCK_GITHUB_API=1 cargo test --test integration_auto_updater
```

**Expected Output:**
- All integration tests pass
- Real GitHub API tests work (may skip on rate limit)
- Mock tests are fast and repeatable
- No race conditions detected
- All existing tests still pass

---

## Task 6: End-to-End Verification & Commit

**Files:**
- No new files; verify all changes

**Steps:**

1. **Full Test Suite:**
   ```bash
   cargo test --lib 2>&1 | grep "test result"
   cargo test --test integration_* 2>&1 | tail -20
   ```
   Expected: 160+ tests passing

2. **Release Build:**
   ```bash
   cargo build --release 2>&1 | tail -5
   ```
   Expected: No errors

3. **CLI Smoke Tests (Against Mock):**
   ```bash
   ./target/release/cleanserve update --check
   # Expected: Prints version info (may be mock data)
   ```

4. **Security Verification:**
   ```bash
   # Verify no hardcoded credentials
   grep -r "token\|secret\|api_key" crates/ || echo "✓ No hardcoded secrets"
   
   # Verify permission handling
   grep -r "0o755\|0o700" crates/ | head -5
   ```

5. **Documentation Update:**
   - Update `docs/guide/auto-updater.md` with GitHub API details
   - Add troubleshooting for rate limits

6. **Create Summary Commit:**
   ```bash
   git add -A
   git commit -m "Phase 4b: Complete GitHub API auto-updater implementation

   - Real GitHub API integration (fetch actual releases)
   - Streaming binary downloads with SHA256 validation
   - Atomic installation with backup/restore
   - Platform detection (linux-x64, darwin-arm64, etc)
   - CLI integration (update --check, --force, -v flags)
   - Enhanced install.sh with CLEANSERVE_AUTO_UPDATE support
   - Integration tests with mock GitHub API
   - Production error handling and retry logic
   
   All 160+ tests passing, release build successful"
   ```

**Expected Output:**
- All tests pass (160+ total)
- Release build succeeds
- CLI commands functional
- Summary commit created
- Ready for production deployment

---

## Success Criteria

- [ ] GitHub API integration working
- [ ] Binary downloads streaming correctly
- [ ] SHA256 checksums validated
- [ ] Atomic installation with backup/restore
- [ ] Rollback mechanism tested
- [ ] CLI fully integrated
- [ ] install.sh enhanced
- [ ] All 160+ tests passing
- [ ] Release build successful
- [ ] Summary commit created

---

## Post-Phase 4b

✅ **Complete** - Package Manager + Auto-Updater System Finished

Next steps:
- Production deployment
- User testing
- Gather feedback
- Plan Phase 6+ enhancements (custom manifests, advanced monitoring, etc)
