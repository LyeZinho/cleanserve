use anyhow::Context;
use cleanserve_proxy::ProxyServer;
use cleanserve_watcher::{FileWatcher, FileEvent};
use tracing::{info, error};

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
    let hot_reload = config.server.hot_reload;
    let php_version = &config.engine.php;
    
    info!("🚀 Starting CleanServe server");
    println!("📁 Root: {}", root);
    println!("🔌 Port: {}", port);
    println!("🐘 PHP: {}", php_version);
    println!("🔄 Hot Reload: {}", if hot_reload { "enabled" } else { "disabled" });
    println!();
    
    // Create proxy server
    let proxy = ProxyServer::new(port, root.clone());
    
    // Start proxy server in background
    let proxy_handle = tokio::spawn(async move {
        proxy.start().await
    });
    
    // Start HMR WebSocket server
    let hmr_port = port + 1;
    let proxy2 = ProxyServer::new(port, root.clone());
    let hmr_handle = tokio::spawn(async move {
        proxy2.start_hmr_server(hmr_port).await
    });
    
    println!("🌐 Server running at http://localhost:{}", port);
    if hot_reload {
        println!("🔌 HMR WebSocket running on ws://localhost:{}", hmr_port);
    }
    println!();
    println!("Press Ctrl+C to stop");
    
    // Start file watcher if hot reload is enabled
    if hot_reload {
        let watcher = FileWatcher::new(&root);
        match watcher.watch() {
            Ok(mut rx) => {
                info!("👀 File watcher started");
                tokio::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        match event {
                            FileEvent::PhpChanged(paths) => {
                                info!("📦 PHP files changed: {:?}", paths);
                                // TODO: Restart PHP worker
                            }
                            FileEvent::StyleChanged(paths) => {
                                info!("🎨 Style files changed: {:?}", paths);
                                // TODO: Trigger style injection via HMR
                            }
                        }
                    }
                });
            }
            Err(e) => {
                info!("⚠️ File watcher failed to start: {}. Hot reload disabled.", e);
            }
        }
    }
    
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
    
    Ok(())
}
