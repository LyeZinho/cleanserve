use anyhow::Context;
use cleanserve_core::PhpDownloader;
use cleanserve_core::PhpWorker;
use cleanserve_proxy::{ProxyServer, HmrEvent, HmrState};
use cleanserve_watcher::{FileWatcher, FileEvent};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

pub async fn run(port: Option<u16>) -> anyhow::Result<()> {
    // Load config
    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load config")?;

    let port = port.unwrap_or(config.server.port);
    let root = config.server.root.clone();
    let hot_reload = config.server.hot_reload;
    let php_version = &config.engine.php;

    info!("🚀 Starting CleanServe server");
    println!("📁 Root: {}", root);
    println!("🔌 Port: {}", port);
    println!("🐘 PHP: {}", php_version);
    println!("🔄 Hot Reload: {}", if hot_reload { "enabled" } else { "disabled" });
    println!();

    // Project-local PHP directory: .cleanserve/php/
    let project_php_dir = Path::new(".cleanserve").join("php");

    // Find or download PHP (project-local, standalone)
    let php_path = if let Some(path) = find_project_php(&project_php_dir) {
        println!("📍 Using project PHP: {}", path.display());
        path
    } else {
        println!("📥 Downloading PHP {} (standalone)...", php_version);
        let downloader = PhpDownloader::new(&project_php_dir)
            .context("Failed to initialize PHP downloader")?;
        downloader.download(php_version).await
            .context("Failed to download PHP")?;
        find_project_php(&project_php_dir)
            .context("PHP downloaded but not found in .cleanserve/php/")?
    };

    println!("✅ PHP ready: {}", php_path.display());

    // Create shared HMR state for event broadcasting
    let hmr_state = Arc::new(RwLock::new(HmrState::new()));

    // Start PHP worker
    let php_root = Path::new(&root).canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&root));
    let mut php_worker = PhpWorker::new(php_path, php_root);
    php_worker.start().context("Failed to start PHP worker")?;
    println!("✅ PHP worker running on port 9000");

    // Create proxy server with shared HMR state
    let proxy = ProxyServer::new_with_hmr(port, root.clone(), hmr_state.clone());

    // Start proxy server in background
    let proxy_handle = tokio::spawn(async move {
        proxy.start().await
    });

    // Start HMR WebSocket server on port+1
    let hmr_port = port + 1;
    let hmr_state2 = hmr_state.clone();
    let hmr_handle = tokio::spawn(async move {
        ProxyServer::start_hmr_server_static(hmr_port, hmr_state2).await
    });

    println!("🌐 Server running at http://localhost:{}", port);
    if hot_reload {
        println!("🔌 HMR WebSocket running on ws://localhost:{}", hmr_port);
    }
    println!();
    println!("Press Ctrl+C to stop");

    // _watcher_guard MUST live until shutdown — dropping it kills the OS file watcher.
    let _watcher_guard = if hot_reload {
        let watcher = FileWatcher::new(&root);
        let hmr = hmr_state.clone();
        match watcher.watch() {
            Ok((mut rx, guard)) => {
                info!("👀 File watcher started");
                tokio::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        match event {
                            FileEvent::PhpChanged(paths) => {
                                info!("📦 PHP files changed: {:?}", paths);
                                let state = hmr.read().await;
                                state.emit(HmrEvent::PhpReload);
                            }
                            FileEvent::StyleChanged(paths) => {
                                info!("🎨 Style files changed: {:?}", paths);
                                let state = hmr.read().await;
                                for path in paths {
                                    state.emit(HmrEvent::StyleReload(path.to_string_lossy().to_string()));
                                }
                            }
                        }
                    }
                });
                Some(guard)
            }
            Err(e) => {
                info!("⚠️ File watcher failed to start: {}. Hot reload disabled.", e);
                None
            }
        }
    } else {
        None
    };

    // Wait for Ctrl+C or error
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n👋 Server stopped");
        }
        result = proxy_handle => {
            if let Err(e) = result {
                error!("Proxy error: {}", e);
            }
        }
        result = hmr_handle => {
            if let Err(e) = result {
                error!("HMR server error: {}", e);
            }
        }
    }

    // Cleanup
    php_worker.stop();

    Ok(())
}

/// Find PHP binary in project's .cleanserve/php/ directory
fn find_project_php(php_dir: &Path) -> Option<PathBuf> {
    // Look for php-X.Y/php (versioned subdirectory)
    if let Ok(entries) = std::fs::read_dir(php_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("php-") {
                    let path = entry.path();
                    
                    // Check direct php-*/php
                    let php_bin = path.join("php");
                    if php_bin.exists() {
                        return Some(php_bin);
                    }
                    
                    // Check php-*/bin/php
                    let php_bin_alt = path.join("bin").join("php");
                    if php_bin_alt.exists() {
                        return Some(php_bin_alt);
                    }
                    
                    // Check php-*/linux-x64/php-* (for versioned binaries)
                    if let Ok(platform_entries) = std::fs::read_dir(&path) {
                        for platform_entry in platform_entries.flatten() {
                            if let Some(platform_name) = platform_entry.file_name().to_str() {
                                if platform_name.contains("x64") || platform_name == "bin" {
                                    let platform_path = platform_entry.path();
                                    // Look for php or php-X.Y.Z
                                    if let Ok(bin_entries) = std::fs::read_dir(&platform_path) {
                                        for bin_entry in bin_entries.flatten() {
                                            if let Some(bin_name) = bin_entry.file_name().to_str() {
                                                if bin_name.starts_with("php") && bin_entry.path().is_file() {
                                                    return Some(bin_entry.path());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
