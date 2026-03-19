# CleanServe Implementation Plan - Phase 1: Foundation

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.
> **Note:** No worktrees or git commits per user request.

**Goal:** Establish the Rust workspace structure and implement core CLI + PHP version management.

**Architecture:** Multi-crate workspace with shared error types, CLI using Clap, PHP binaries managed in `~/.cleanserve/`.

**Tech Stack:** 
- Rust 2021 edition
- `clap` for CLI
- `serde` + `serde_json` for config
- `reqwest` for downloads
- `tokio` for async runtime

---

## Workspace Setup

### Task 1: Initialize Cargo Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/cleanserve-shared/Cargo.toml`
- Create: `crates/cleanserve-core/Cargo.toml`
- Create: `crates/cleanserve-cli/Cargo.toml`

**Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/cleanserve-shared",
    "crates/cleanserve-core",
    "crates/cleanserve-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["CleanServe Team"]
license = "MIT"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
```

**Step 2: Create cleanserve-shared/Cargo.toml**

```toml
[package]
name = "cleanserve-shared"
version.workspace = true
edition.workspace = true

[dependencies]
anyhow = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
```

**Step 3: Create cleanserve-core/Cargo.toml**

```toml
[package]
name = "cleanserve-core"
version.workspace = true
edition.workspace = true

[dependencies]
cleanserve-shared = { path = "../cleanserve-shared" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
dirs = "5"
zip = "2"
```

**Step 4: Create cleanserve-cli/Cargo.toml**

```toml
[package]
name = "cleanserve-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "cleanserve"
path = "src/main.rs"

[dependencies]
cleanserve-shared = { path = "../cleanserve-shared" }
cleanserve-core = { path = "../cleanserve-core" }
clap = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
```

---

## Shared Crate (cleanserve-shared)

### Task 2: Implement Error Types

**Files:**
- Create: `crates/cleanserve-shared/src/lib.rs`
- Create: `crates/cleanserve-shared/src/error.rs`

**Step 1: Create error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CleanServeError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("PHP not found: {0}")]
    PhpNotFound(String),
    
    #[error("Download error: {0}")]
    Download(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Watch error: {0}")]
    Watch(String),
}

pub type Result<T> = std::result::Result<T, CleanServeError>;
```

**Step 2: Create lib.rs**

```rust
pub mod error;
pub use error::{CleanServeError, Result};
```

---

## Core Crate (cleanserve-core)

### Task 3: Implement Config Parsing

**Files:**
- Create: `crates/cleanserve-core/src/lib.rs`
- Create: `crates/cleanserve-core/src/config.rs`

**Step 1: Create config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanServeConfig {
    pub name: String,
    pub engine: EngineConfig,
    #[serde(default)]
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub php: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default = "default_display_errors")]
    pub display_errors: bool,
    #[serde(default)]
    pub memory_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
}

fn default_display_errors() -> bool { true }
fn default_root() -> String { "public/".to_string() }
fn default_port() -> u16 { 8080 }
fn default_hot_reload() -> bool { true }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            port: default_port(),
            hot_reload: default_hot_reload(),
        }
    }
}

impl CleanServeConfig {
    pub fn load(path: impl AsRef<Path>) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| CleanServeError::Config(format!("Failed to read: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| CleanServeError::Config(format!("Parse error: {}", e)))
    }
    
    pub fn save(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CleanServeError::Config(format!("Serialize error: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| CleanServeError::Config(format!("Write error: {}", e)))
    }
}
```

**Step 2: Create lib.rs**

```rust
pub mod config;
pub mod php_manager;

pub use config::{CleanServeConfig, EngineConfig, ServerConfig};
pub use php_manager::PhpManager;
pub use cleanserve_shared::{CleanServeError, Result};
```

### Task 4: Implement PHP Manager

**Files:**
- Create: `crates/cleanserve-core/src/php_manager.rs`

**Step 1: Create php_manager.rs**

```rust
use crate::{CleanServeError, Result};
use std::path::PathBuf;

pub struct PhpManager {
    cache_dir: PathBuf,
}

impl PhpManager {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| CleanServeError::Config("Cannot find home directory".into()))?
            .join(".cleanserve")
            .join("bin");
        
        std::fs::create_dir_all(&cache_dir)?;
        
        Ok(Self { cache_dir })
    }
    
    pub fn list_installed(&self) -> Vec<String> {
        std::fs::read_dir(&self.cache_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                if name.starts_with("php-") {
                    Some(name.strip_prefix("php-")?.to_string())
                } else {
                    None
                }
            })
            .collect()
    }
    
    pub fn get_path(&self, version: &str) -> Option<PathBuf> {
        let path = self.cache_dir.join(format!("php-{}", version));
        #[cfg(windows)]
        let exe = path.join("php.exe");
        #[cfg(not(windows))]
        let exe = path.join("bin").join("php");
        
        if exe.exists() {
            Some(exe)
        } else {
            None
        }
    }
    
    pub fn is_installed(&self, version: &str) -> bool {
        self.get_path(version).is_some()
    }
}
```

---

## CLI Crate (cleanserve-cli)

### Task 5: Implement Main CLI

**Files:**
- Create: `crates/cleanserve-cli/src/main.rs`
- Create: `crates/cleanserve-cli/src/commands/mod.rs`
- Create: `crates/cleanserve-cli/src/commands/init.rs`
- Create: `crates/cleanserve-cli/src/commands/use.rs`
- Create: `crates/cleanserve-cli/src/commands/list.rs`

**Step 1: Create main.rs**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cleanserve")]
#[command(version = "0.1.0")]
#[command(about = "Zero-Burden PHP Runtime & Development Server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new CleanServe project
    Init {
        /// Project name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
        /// PHP version to use
        #[arg(short, long, default_value = "8.4")]
        php: String,
    },
    /// Start the development server
    Up {
        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Stop the development server
    Down,
    /// Switch PHP version
    Use {
        /// PHP version (e.g., 8.2, 8.4)
        version: String,
    },
    /// List installed PHP versions
    List,
    /// Download PHP version
    Update {
        /// PHP version to download (default: latest stable)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Run Composer with project's PHP
    Composer {
        /// Composer arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { name, php } => {
            commands::init::run(name, php).await?;
        }
        Commands::Up { port } => {
            println!("Starting server on port {}", port);
            // TODO: Implement in Phase 2
        }
        Commands::Down => {
            println!("Stopping server");
            // TODO: Implement in Phase 2
        }
        Commands::Use { version } => {
            commands::use_::run(version).await?;
        }
        Commands::List => {
            commands::list::run().await?;
        }
        Commands::Update { version } => {
            println!("Updating PHP to {}", version.unwrap_or_else(|| "latest".into()));
            // TODO: Implement download in Phase 1
        }
        Commands::Composer { args } => {
            println!("Running composer: {:?}", args);
            // TODO: Implement in Phase 4
        }
    }
    
    Ok(())
}
```

**Step 2: Create commands/mod.rs**

```rust
pub mod init;
pub mod use_;
pub mod list;
```

**Step 3: Create init.rs**

```rust
use crate::commands::Cli;
use anyhow::Context;
use std::path::Path;

pub async fn run(name: Option<String>, php: String) -> anyhow::Result<()> {
    // Determine project name
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "my-project".to_string())
    });
    
    // Check if cleanserve.json already exists
    let config_path = Path::new("cleanserve.json");
    if config_path.exists() {
        anyhow::bail!("cleanserve.json already exists. Remove it first.");
    }
    
    // Create config
    let config = cleanserve_core::CleanServeConfig {
        name: project_name,
        engine: cleanserve_core::EngineConfig {
            php,
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server: cleanserve_core::ServerConfig::default(),
    };
    
    // Save config
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;
    
    println!("✓ Created cleanserve.json");
    println!("Run 'cleanserve up' to start the server");
    
    Ok(())
}
```

**Step 4: Create use.rs**

```rust
use anyhow::Context;

pub async fn run(version: String) -> anyhow::Result<()> {
    let manager = cleanserve_core::PhpManager::new()
        .context("Failed to initialize PHP manager")?;
    
    if !manager.is_installed(&version) {
        anyhow::bail!(
            "PHP {} is not installed. Run 'cleanserve update --version {}' to download it.",
            version, version
        );
    }
    
    // Update cleanserve.json
    let config_path = std::path::Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }
    
    let mut config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load cleanserve.json")?;
    
    config.engine.php = version.clone();
    config.save(config_path)
        .context("Failed to save cleanserve.json")?;
    
    println!("✓ Switched to PHP {}", version);
    
    Ok(())
}
```

**Step 5: Create list.rs**

```rust
use anyhow::Context;

pub async fn run() -> anyhow::Result<()> {
    let manager = cleanserve_core::PhpManager::new()
        .context("Failed to initialize PHP manager")?;
    
    let installed = manager.list_installed();
    
    if installed.is_empty() {
        println!("No PHP versions installed.");
        println!("Run 'cleanserve update --version 8.4' to install PHP.");
    } else {
        println!("Installed PHP versions:");
        for version in installed {
            println!("  • {}", version);
        }
    }
    
    Ok(())
}
```

---

## Verification

After completing all tasks, verify:

1. **Build:** `cargo build --release` should succeed
2. **CLI Works:** `cargo run -- --help` should show all commands
3. **Init Works:** `cargo run -- init --php 8.4` should create cleanserve.json
4. **List Works:** `cargo run -- list` should show installed versions (empty initially)

---

## Next Phase

Phase 2: Server Core
- Implement Hyper-based HTTP proxy
- Implement PHP worker lifecycle
- SSL certificate generation
