# CleanServe Implementation Plan - Phase 2: Server Core

> **For Claude:** Use subagent-driven development to implement Phase 2.
> **Note:** No worktrees or git commits per user request.

**Goal:** Implement the Hyper-based HTTP proxy, PHP worker lifecycle, and SSL certificate generation.

**Architecture:**
- `cleanserve-proxy`: Hyper HTTP server with request routing and WebSocket support
- `cleanserve-watcher`: File system monitoring with notify crate
- PHP worker: Persistent process communicating via CGI-like pipes

**Tech Stack:**
- `hyper` for HTTP server (low-level for <5ms latency)
- `tokio` for async runtime
- `notify` for file watching
- `rcgen` for SSL certificate generation
- `rustls` for TLS

---

## Phase 2.1: Create cleanserve-proxy crate

### Add to workspace:

### Task: Create cleanserve-proxy Cargo.toml

**File:** `crates/cleanserve-proxy/Cargo.toml`

```toml
[package]
name = "cleanserve-proxy"
version.workspace = true
edition.workspace = true

[dependencies]
cleanserve-shared = { path = "../cleanserve-shared" }
cleanserve-core = { path = "../cleanserve-core" }
hyper = { version = "1", features = ["server", "http1", "http2"] }
hyper-util = { version = "0.1", features = ["tokio", "server"] }
http-body-util = "0.1"
tokio = { workspace = true }
anyhow = { workspace = true }
rcgen = "0.11"
rustls = "0.23"
rustls-pemfile = "2"
time = "0.3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Task: Implement Hyper server

**File:** `crates/cleanserve-proxy/src/server.rs`

```rust
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, error};

pub struct ProxyServer {
    port: u16,
    root: String,
}

impl ProxyServer {
    pub fn new(port: u16, root: String) -> Self {
        Self { port, root }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;
        
        info!("🚀 CleanServe proxy listening on http://{}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let io = TokioIo::new(stream);
                    let root = self.root.clone();
                    
                    tokio::spawn(async move {
                        let service = service_fn(move |req| handle_request(req, &root));
                        if let Err(e) = http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            error!("Error serving connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    root: &str,
) -> Result<Response<hyper::body::Incoming>, Infallible> {
    let path = req.uri().path();
    
    // TODO: Route to PHP worker for .php files
    // TODO: Serve static files for other paths
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(hyper::body::Incoming::default())
        .unwrap())
}
```

**File:** `crates/cleanserve-proxy/src/lib.rs`

```rust
pub mod server;

pub use server::ProxyServer;
```

---

## Phase 2.2: Create cleanserve-watcher crate

### Task: Create cleanserve-watcher Cargo.toml

**File:** `crates/cleanserve-watcher/Cargo.toml`

```toml
[package]
name = "cleanserve-watcher"
version.workspace = true
edition.workspace = true

[dependencies]
notify = { version = "6", default-features = false, features = ["macos_kqueue"] }
notify-debouncer-mini = "0.4"
tokio = { workspace = true }
anyhow = { workspace = true }
tracing = "0.1"
```

### Task: Implement file watcher

**File:** `crates/cleanserve-watcher/src/watcher.rs`

```rust
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, error};

pub enum FileEvent {
    PhpChanged(Vec<PathBuf>),
    StyleChanged(Vec<PathBuf>),
}

pub struct FileWatcher {
    root: PathBuf,
}

impl FileWatcher {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
        }
    }

    pub fn watch(&self) -> anyhow::Result<mpsc::Receiver<FileEvent>> {
        let (tx, rx) = mpsc::channel(100);
        let root = self.root.clone();
        
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                if let Ok(events) = res {
                    let mut php_events = Vec::new();
                    let mut style_events = Vec::new();
                    
                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            let path = event.path.clone();
                            if let Some(ext) = path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if ext_str == "php" {
                                    php_events.push(path);
                                } else if ext_str == "css" || ext_str == "js" {
                                    style_events.push(path);
                                }
                            }
                        }
                    }
                    
                    if !php_events.is_empty() {
                        let _ = tx.blocking_send(FileEvent::PhpChanged(php_events));
                    }
                    if !style_events.is_empty() {
                        let _ = tx.blocking_send(FileEvent::StyleChanged(style_events));
                    }
                }
            },
        )?;

        debouncer.watcher().watch(&root, RecursiveMode::Recursive)?;
        info!("👀 Watching {} for changes", root.display());
        
        Ok(rx)
    }
}
```

**File:** `crates/cleanserve-watcher/src/lib.rs`

```rust
pub mod watcher;

pub use watcher::{FileWatcher, FileEvent};
```

---

## Phase 2.3: Implement PHP worker lifecycle

### Task: Create PHP worker module

**File:** `crates/cleanserve-core/src/php_worker.rs`

```rust
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::io::{BufRead, BufReader, Write};
use tokio::sync::mpsc;
use tracing::{info, error};

pub struct PhpWorker {
    php_path: PathBuf,
    root: PathBuf,
    child: Option<Child>,
}

impl PhpWorker {
    pub fn new(php_path: PathBuf, root: PathBuf) -> Self {
        Self {
            php_path,
            root,
            child: None,
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        // Kill existing worker if any
        self.stop();
        
        info!("🔄 Starting PHP worker: {}", self.php_path.display());
        
        let child = std::process::Command::new(&self.php_path)
            .args([
                "-S", "127.0.0.1:9000",
                "-t", self.root.to_str().unwrap_or("."),
                "-d", "variables_order=EGPCS",
                "-d", "cgi.fix_pathinfo=1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        self.child = Some(child);
        
        // Wait a bit for worker to start
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        info!("✅ PHP worker started on port 9000");
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            info!("🛑 PHP worker stopped");
        }
    }

    pub fn is_running(&self) -> bool {
        if let Some(ref mut child) = self.child {
            child.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    }
}

impl Drop for PhpWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
```

Update `crates/cleanserve-core/src/lib.rs`:

```rust
pub mod config;
pub mod php_manager;
pub mod php_worker;

pub use config::{CleanServeConfig, EngineConfig, ServerConfig};
pub use php_manager::PhpManager;
pub use php_worker::PhpWorker;
pub use cleanserve_shared::{CleanServeError, Result};
```

---

## Phase 2.4: SSL certificate generation

### Task: Add SSL utilities

**File:** `crates/cleanserve-core/src/ssl.rs`

```rust
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType};
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub struct SslManager {
    cert_dir: PathBuf,
}

impl SslManager {
    pub fn new() -> anyhow::Result<Self> {
        let cert_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
            .join(".cleanserve")
            .join("certs");
        
        fs::create_dir_all(&cert_dir)?;
        
        Ok(Self { cert_dir })
    }

    pub fn get_or_create_cert(&self, domain: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
        let key_path = self.cert_dir.join(format!("{}.key", domain));
        let cert_path = self.cert_dir.join(format!("{}.crt", domain));
        
        // Return existing certificates if they exist
        if key_path.exists() && cert_path.exists() {
            return Ok((key_path, cert_path));
        }
        
        info!("🔐 Generating self-signed certificate for {}", domain);
        
        // Generate new certificate
        let mut cert_params = CertificateParams::default();
        cert_params.is_ca = rcgen::IsCa::NoCa;
        
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, domain);
        distinguished_name.push(DnType::OrganizationName, "CleanServe");
        distinguished_name.push(DnType::CountryName, "XX");
        cert_params.distinguished_name = distinguished_name;
        
        // Validity: 1 year
        let not_before = time::OffsetDateTime::now_utc();
        let not_after = not_before + time::Duration::days(365);
        cert_params.not_before = not_before;
        cert_params.not_after = not_after;
        
        let cert = Certificate::from_params(cert_params)?;
        let pem_cert = cert.pem();
        let pem_key = cert.serialize_private_key_pem();
        
        fs::write(&cert_path, pem_cert)?;
        fs::write(&key_path, pem_key)?;
        
        info!("✅ Certificate generated: {}", cert_path.display());
        
        Ok((key_path, cert_path))
    }
}
```

Update `crates/cleanserve-core/src/lib.rs`:

```rust
pub mod config;
pub mod php_manager;
pub mod php_worker;
pub mod ssl;

pub use config::{CleanServeConfig, EngineConfig, ServerConfig};
pub use php_manager::PhpManager;
pub use php_worker::PhpWorker;
pub use ssl::SslManager;
pub use cleanserve_shared::{CleanServeError, Result};
```

---

## Phase 2.5: Integrate with CLI 'up' command

### Task: Update main.rs to implement 'up' command

Update `crates/cleanserve-cli/src/main.rs`:

```rust
use clap::{Parser, Subcommand};
use anyhow::Context;

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
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long, default_value = "8.4")]
        php: String,
    },
    /// Start the development server
    Up {
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Stop the development server
    Down,
    /// Switch PHP version
    Use {
        version: String,
    },
    /// List installed PHP versions
    List,
    /// Download PHP version
    Update {
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Run Composer with project's PHP
    Composer {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("cleanserve=info".parse()?))
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { name, php } => {
            commands::init::run(name, php).await?;
        }
        Commands::Up { port } => {
            commands::up::run(port).await?;
        }
        Commands::Down => {
            commands::down::run().await?;
        }
        Commands::Use { version } => {
            commands::use_::run(version).await?;
        }
        Commands::List => {
            commands::list::run().await?;
        }
        Commands::Update { version } => {
            println!("PHP download coming in Phase 2...");
        }
        Commands::Composer { args } => {
            println!("Composer integration coming in Phase 4...");
        }
    }
    
    Ok(())
}
```

### Task: Create up.rs command

**File:** `crates/cleanserve-cli/src/commands/up.rs`

```rust
use anyhow::Context;

pub async fn run(port: Option<u16>) -> anyhow::Result<()> {
    // Load config
    let config_path = std::path::Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }
    
    let config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load config")?;
    
    let port = port.unwrap_or(config.server.port);
    let root = config.server.root.clone();
    let php_version = &config.engine.php;
    
    // Check if PHP is installed
    let manager = cleanserve_core::PhpManager::new()
        .context("Failed to init PHP manager")?;
    
    let php_path = manager.get_path(php_version)
        .context(format!("PHP {} not found. Run 'cleanserve update --version {}'", php_version, php_version))?;
    
    println!("🚀 Starting CleanServe server");
    println!("📁 Root: {}", root);
    println!("🔌 Port: {}", port);
    println!("🐘 PHP: {}", php_version);
    println!();
    println!("Server running at https://localhost:{}", port);
    println!("Press Ctrl+C to stop");
    
    // TODO: Start proxy server (Phase 2 full implementation)
    // TODO: Start PHP worker
    // TODO: Start file watcher
    
    // For now, just show info
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Server stopped");
    
    Ok(())
}
```

### Task: Create down.rs command

**File:** `crates/cleanserve-cli/src/commands/down.rs`

```rust
pub async fn run() -> anyhow::Result<()> {
    println!("🛑 Stopping CleanServe server...");
    // TODO: Stop running server (PID file or similar)
    println!("✅ Server stopped");
    Ok(())
}
```

Update `crates/cleanserve-cli/src/commands/mod.rs`:

```rust
pub mod init;
pub mod up;
pub mod down;
pub mod use_;
pub mod list;
```

---

## Update workspace Cargo.toml

**File:** `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/cleanserve-shared",
    "crates/cleanserve-core",
    "crates/cleanserve-proxy",
    "crates/cleanserve-watcher",
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
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## Verification

1. `cargo check --workspace` - All crates compile
2. `cargo run --bin cleanserve -- up` - Shows server starting (placeholder)
3. `cargo build --release` - Release build succeeds

---

## Next Phase

Phase 3: Hot Reload
- WebSocket server for HMR
- Client-side HMR script injection
- CSS injection without page refresh
