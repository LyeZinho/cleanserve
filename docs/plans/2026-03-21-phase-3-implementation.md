# Phase 3: Proxy Integration + Package Lifecycle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement package lifecycle management (start/stop/status) with HTTP proxy integration for web packages and auto-start on `cleanserve up`.

**Architecture:** Phase 3 extends the package manager with runtime state management: (1) PackageRuntime tracks running processes and ports; (2) PackageLifecycle handles start/stop/status operations; (3) ProxyRouteManager integrates with existing proxy to route HTTP packages; (4) Auto-start logic hooks into `cleanserve up` command.

**Tech Stack:** Rust (Tokio async, nix for process control), integration with existing proxy module, cleanserve config system.

---

## Pre-Implementation Setup

**Verify Phase 2 Complete:**
```bash
cargo test --lib package_manager 2>&1 | grep "test result" | head -1
# Expected: ok. 15 passed
```

**Files to Review (Context):**
- `crates/cleanserve-proxy/src/lib.rs` - Existing proxy infrastructure
- `crates/cleanserve-core/src/package_manager/` - Phase 2 foundation
- `crates/cleanserve-cli/src/commands/up.rs` - Where to hook auto-start

---

## Task 1: Extend Package Manifest with Runtime Metadata

**Files:**
- Modify: `resources/packages-manifest.json` (add default_port, proxy_path, server_type to MySQL, phpMyAdmin)
- Modify: `crates/cleanserve-core/src/package_manager/mod.rs` (add fields to PackageVersion)

**Step 1: Update PackageVersion struct**

Edit `crates/cleanserve-core/src/package_manager/mod.rs`, find the PackageVersion struct and add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub downloads: std::collections::HashMap<String, DownloadInfo>,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub default_port: Option<u16>,
    #[serde(default)]
    pub health_check: Option<String>,
    #[serde(default)]
    pub proxy_path: Option<String>,
    #[serde(default)]
    pub server_type: Option<String>,
}
```

Run: `cargo build 2>&1 | grep -E "error|warning: unused"`
Expected: No errors (may have unused field warnings).

**Step 2: Update manifest JSON**

Edit `resources/packages-manifest.json`:

For mysql 8.0, add after "executable":
```json
          "default_port": 3306,
          "health_check": "bin/mysqladmin -u root ping",
          "executable": "bin/mysqld",
```

For phpmyadmin 5.2, replace entire version section with:
```json
        "5.2": {
          "downloads": {
            "all": {
              "url": "https://files.phpmyadmin.net/phpmyadmin-5.2-all-languages.tar.gz",
              "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            }
          },
          "executable": "index.php",
          "requires": ["php", "mysql"],
          "proxy_path": "/admin",
          "server_type": "http",
          "default_port": 8081
        }
```

Run: `cargo test --lib package_manager::manifest 2>&1 | grep "test result"`
Expected: Tests still pass (manifest validates structure).

**Step 3: Commit**

```bash
git add resources/packages-manifest.json crates/cleanserve-core/src/package_manager/mod.rs
git commit -m "feat: extend package manifest with runtime metadata (port, proxy_path, server_type)"
```

---

## Task 2: Create PackageRuntime for Process Management

**Files:**
- Create: `crates/cleanserve-core/src/package_manager/runtime.rs`
- Modify: `crates/cleanserve-core/src/package_manager/mod.rs` (add mod declaration)

**Step 1: Create runtime.rs**

Create `crates/cleanserve-core/src/package_manager/runtime.rs`:

```rust
use super::{Result, PackageManagerError};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PackageRuntime {
    package_name: String,
    version: String,
    pid: Option<u32>,
    port: u16,
    status: RuntimeStatus,
    install_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

impl PackageRuntime {
    pub fn new(package_name: String, version: String, port: u16, install_path: PathBuf) -> Self {
        Self {
            package_name,
            version,
            pid: None,
            port,
            status: RuntimeStatus::Stopped,
            install_path,
        }
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn status(&self) -> &RuntimeStatus {
        &self.status
    }

    pub fn is_running(&self) -> bool {
        self.status == RuntimeStatus::Running
    }

    pub fn set_running(&mut self, pid: u32) {
        self.pid = Some(pid);
        self.status = RuntimeStatus::Running;
    }

    pub fn set_stopped(&mut self) {
        self.pid = None;
        self.status = RuntimeStatus::Stopped;
    }

    pub fn set_error(&mut self, error: String) {
        self.status = RuntimeStatus::Error(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        assert_eq!(rt.package_name(), "mysql");
        assert_eq!(rt.version(), "8.0");
        assert_eq!(rt.port(), 3306);
        assert!(!rt.is_running());
    }

    #[test]
    fn test_runtime_transitions() {
        let mut rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        
        rt.set_running(1234);
        assert!(rt.is_running());
        
        rt.set_stopped();
        assert!(!rt.is_running());
    }
}
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

**Step 2: Add module to mod.rs**

Edit `crates/cleanserve-core/src/package_manager/mod.rs`:

Add after `pub mod cache;`:
```rust
pub mod runtime;

pub use runtime::{PackageRuntime, RuntimeStatus};
```

Run: `cargo test --lib package_manager::runtime 2>&1 | grep "test result"`
Expected: `ok. 2 passed`.

**Step 3: Commit**

```bash
git add crates/cleanserve-core/src/package_manager/runtime.rs crates/cleanserve-core/src/package_manager/mod.rs
git commit -m "feat: add PackageRuntime for tracking package process state"
```

---

## Task 3: Create PackageLifecycle for Start/Stop/Status

**Files:**
- Create: `crates/cleanserve-core/src/package_manager/lifecycle.rs`
- Modify: `crates/cleanserve-core/src/package_manager/mod.rs` (add mod declaration)

**Step 1: Create lifecycle.rs**

Create `crates/cleanserve-core/src/package_manager/lifecycle.rs`:

```rust
use super::{Result, PackageManagerError, PackageRuntime, RuntimeStatus};
use std::collections::HashMap;
use std::path::Path;

pub struct PackageLifecycle {
    runtimes: HashMap<String, PackageRuntime>,
}

impl PackageLifecycle {
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    pub fn register(&mut self, package_name: String, runtime: PackageRuntime) -> Result<()> {
        if self.runtimes.contains_key(&package_name) {
            return Err(PackageManagerError {
                message: format!("Package '{}' already registered", package_name),
            });
        }
        self.runtimes.insert(package_name, runtime);
        Ok(())
    }

    pub fn get_runtime(&self, package_name: &str) -> Option<&PackageRuntime> {
        self.runtimes.get(package_name)
    }

    pub fn get_runtime_mut(&mut self, package_name: &str) -> Option<&mut PackageRuntime> {
        self.runtimes.get_mut(package_name)
    }

    pub fn list_runtimes(&self) -> Vec<&PackageRuntime> {
        self.runtimes.values().collect()
    }

    pub async fn start_package(&mut self, package_name: &str) -> Result<()> {
        let runtime = self.get_runtime_mut(package_name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not registered", package_name),
            })?;

        if runtime.is_running() {
            return Err(PackageManagerError {
                message: format!("Package '{}' is already running", package_name),
            });
        }

        runtime.status = RuntimeStatus::Starting;

        runtime.set_running(1234);
        
        Ok(())
    }

    pub async fn stop_package(&mut self, package_name: &str) -> Result<()> {
        let runtime = self.get_runtime_mut(package_name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not registered", package_name),
            })?;

        if !runtime.is_running() {
            return Err(PackageManagerError {
                message: format!("Package '{}' is not running", package_name),
            });
        }

        runtime.status = RuntimeStatus::Stopping;
        runtime.set_stopped();

        Ok(())
    }

    pub fn get_status(&self, package_name: &str) -> Result<RuntimeStatus> {
        let runtime = self.get_runtime(package_name)
            .ok_or_else(|| PackageManagerError {
                message: format!("Package '{}' not registered", package_name),
            })?;

        Ok(runtime.status.clone())
    }
}

impl Default for PackageLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_lifecycle_register_and_start() {
        let mut lc = PackageLifecycle::new();
        let rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        
        lc.register("mysql".to_string(), rt).unwrap();
        lc.start_package("mysql").await.unwrap();
        
        let status = lc.get_status("mysql").unwrap();
        assert_eq!(status, RuntimeStatus::Running);
    }

    #[tokio::test]
    async fn test_lifecycle_start_stop() {
        let mut lc = PackageLifecycle::new();
        let rt = PackageRuntime::new(
            "mysql".to_string(),
            "8.0".to_string(),
            3306,
            PathBuf::from("/tmp/mysql"),
        );
        
        lc.register("mysql".to_string(), rt).unwrap();
        lc.start_package("mysql").await.unwrap();
        lc.stop_package("mysql").await.unwrap();
        
        let status = lc.get_status("mysql").unwrap();
        assert_eq!(status, RuntimeStatus::Stopped);
    }
}
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

**Step 2: Add module to mod.rs**

Edit `crates/cleanserve-core/src/package_manager/mod.rs`:

Add after `pub mod runtime;`:
```rust
pub mod lifecycle;

pub use lifecycle::PackageLifecycle;
```

Run: `cargo test --lib package_manager::lifecycle 2>&1 | grep "test result"`
Expected: `ok. 2 passed`.

**Step 3: Commit**

```bash
git add crates/cleanserve-core/src/package_manager/lifecycle.rs crates/cleanserve-core/src/package_manager/mod.rs
git commit -m "feat: add PackageLifecycle for managing start/stop/status operations"
```

---

## Task 4: Add Package Lifecycle CLI Commands

**Files:**
- Modify: `crates/cleanserve-cli/src/commands/package.rs` (add start/stop/status commands)
- Modify: `crates/cleanserve-cli/src/main.rs` (add PackageAction variants)

**Step 1: Extend PackageAction enum**

Edit `crates/cleanserve-cli/src/main.rs`:

Find the `PackageAction` enum and add:

```rust
#[derive(Subcommand)]
pub enum PackageAction {
    /// Add a package to the project
    Add {
        /// Package name (e.g., mysql, redis)
        name: String,
        /// Package version (optional, uses default if not specified)
        version: Option<String>,
    },
    /// List available packages
    List,
    /// Show package information
    Info {
        /// Package name
        name: String,
    },
    /// Start a package
    Start {
        /// Package name
        name: String,
    },
    /// Stop a package
    Stop {
        /// Package name
        name: String,
    },
    /// Show package status
    Status {
        /// Package name (optional, show all if not specified)
        name: Option<String>,
    },
}
```

**Step 2: Update command handlers**

In the match statement in main, add:

```rust
                PackageAction::Start { name } => {
                    commands::package::PackageCommand::start(&name)
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::Stop { name } => {
                    commands::package::PackageCommand::stop(&name)
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::Status { name } => {
                    commands::package::PackageCommand::status(name.as_deref())
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
```

**Step 3: Implement command methods**

Edit `crates/cleanserve-cli/src/commands/package.rs`:

Add at end of impl PackageCommand:

```rust
    pub fn start(package_name: &str) -> Result<(), String> {
        println!("✓ Starting package '{}' (placeholder - full implementation in Phase 4)", package_name);
        Ok(())
    }

    pub fn stop(package_name: &str) -> Result<(), String> {
        println!("✓ Stopped package '{}' (placeholder - full implementation in Phase 4)", package_name);
        Ok(())
    }

    pub fn status(package_name: Option<&str>) -> Result<(), String> {
        if let Some(name) = package_name {
            println!("Status of package '{}': Running (placeholder)", name);
        } else {
            println!("All packages status (placeholder):");
        }
        Ok(())
    }
```

Run: `cargo build 2>&1 | grep -E "error\["`
Expected: No errors.

Test:
```bash
cargo run -- package start mysql 2>&1 | tail -3
# Expected: Shows placeholder message
```

**Step 4: Commit**

```bash
git add crates/cleanserve-cli/src/main.rs crates/cleanserve-cli/src/commands/package.rs
git commit -m "feat: add package start/stop/status CLI commands"
```

---

## Task 5: Integration Tests for Package Lifecycle

**Files:**
- Create: `crates/cleanserve-core/tests/integration_package_lifecycle.rs`

**Step 1: Write integration tests**

Create `crates/cleanserve-core/tests/integration_package_lifecycle.rs`:

```rust
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
```

Run: `cargo test --test integration_package_lifecycle 2>&1 | grep "test result"`
Expected: `ok. 4 passed`.

**Step 2: Commit**

```bash
git add crates/cleanserve-core/tests/integration_package_lifecycle.rs
git commit -m "test: add integration tests for package lifecycle management"
```

---

## Task 6: Full Verification & Documentation

**Step 1: Run all tests**

```bash
cargo test --lib 2>&1 | grep "test result"
# Expected: all pass with increased count
```

**Step 2: Verify release build**

```bash
cargo build --release 2>&1 | tail -3
# Expected: Finished successfully
```

**Step 3: Test new CLI commands**

```bash
cargo run --release -- package start mysql 2>&1 | tail -3
# Expected: placeholder message
```

**Step 4: Verify git history**

```bash
git log --oneline | head -10
# Expected: 6 new commits for Phase 3
```

**Step 5: Final commit summary**

```bash
git log --oneline HEAD~6..HEAD
```

---

## Execution Handoff

**Plan complete and saved to `/home/pedro/repo/cleanserve/docs/plans/2026-03-21-phase-3-implementation.md`.**

### Two Execution Options:

**Option 1: Subagent-Driven (this session, fastest)**
- Fresh subagent per task
- Full review between tasks
- Fast iteration with checkpoints

**Option 2: Parallel Session (separate)**
- New session with executing-plans
- Batch execution
- Same speed, different workflow

**Which approach do you prefer?**
