use anyhow::Context;
use cleanserve_core::{
    ensure_writable_dirs, find_server_ports, prepare_project_php_ini, FrameworkInfo, PhpDownloader,
    PhpManager, PhpWorker, SslManager,
};
use cleanserve_proxy::{HmrEvent, HmrState, ProxyServer};
use cleanserve_watcher::{FileEvent, FileWatcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

const PHP_BACKEND_PORT: u16 = 9000;

pub async fn run(port: Option<u16>) -> anyhow::Result<()> {
    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load config")?;

    let preferred_port = port.unwrap_or(config.server.port);
    let root = config.server.root.clone();
    let hot_reload = config.server.hot_reload;
    let php_version = &config.engine.php;
    let project_root = std::env::current_dir().context("Failed to get project directory")?;

    let (server_port, hmr_port) = find_server_ports(preferred_port, hot_reload).ok_or_else(|| {
        anyhow::anyhow!(
            "No free ports near {} (need {} consecutive port(s)). Stop the other process or use --port.",
            preferred_port,
            if hot_reload { 2 } else { 1 }
        )
    })?;

    if server_port != preferred_port {
        println!(
            "⚠️  Port {} is in use — using https://localhost:{} instead",
            preferred_port,
            server_port
        );
    }

    println!("\n🌀 CleanServe v0.3.0 starting...\n");

    let manager = PhpManager::new().context("Failed to init PHP manager")?;
    let php_path = if let Some(path) = manager.get_path(php_version) {
        info!("Using PHP: {}", path.display());
        path
    } else {
        println!("📦 Downloading PHP {}...", php_version);
        let downloader =
            PhpDownloader::global().context("Failed to initialize PHP downloader")?;
        downloader
            .ensure(php_version)
            .await
            .context("Failed to download PHP")?
    };

    let resolved = manager
        .resolve_installed_version(php_version)
        .unwrap_or_else(|| php_version.clone());
    println!("📦 Using PHP {} (NTS)", resolved);

    let framework = FrameworkInfo::detect(&project_root);
    for result in ensure_writable_dirs(&framework) {
        if let Err(e) = result {
            info!("Could not prepare framework directory: {}", e);
        }
    }

    let php_runtime = prepare_project_php_ini(
        &project_root,
        &php_path,
        &resolved,
        &config.engine,
        &framework,
    )
    .context("Failed to write project php.ini")?;
    println!("📝 PHP config: {}", php_runtime.ini_path.display());

    let hmr_state = Arc::new(RwLock::new(HmrState::new()));

    let php_root = Path::new(&root)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&root));
    let mut php_worker =
        PhpWorker::new(php_path, php_root).with_php_ini(php_runtime.ini_path);
    let php_backend_port = php_worker
        .start_on_port(PHP_BACKEND_PORT)
        .context("Failed to start PHP worker")?;
    if php_backend_port != PHP_BACKEND_PORT {
        println!(
            "⚠️  PHP port {} in use — worker listening on {}",
            PHP_BACKEND_PORT,
            php_backend_port
        );
    }
    println!("🔧 PHP worker started (127.0.0.1:{})", php_backend_port);

    if Path::new("composer.json").exists() {
        if Path::new("vendor/autoload.php").exists() {
            println!("🎼 Composer dependencies ready");
        } else {
            println!("⚠️  vendor/ missing — run 'cleanserve composer install'");
        }
    }

    let ssl = SslManager::new().context("Failed to init SSL manager")?;
    let (key_path, cert_path) = ssl
        .get_or_create_cert("localhost")
        .context("Failed to prepare TLS certificate")?;
    println!(
        "🔒 TLS certificate: {} (localhost)",
        cert_path.display()
    );

    let proxy = ProxyServer::new_with_hmr(server_port, root.clone(), hmr_state.clone())
        .with_tls(cert_path, key_path)
        .with_php_backend(php_backend_port);

    let proxy_handle = tokio::spawn(async move { proxy.start().await });

    let hmr_state2 = hmr_state.clone();
    let hmr_handle = tokio::spawn(async move {
        ProxyServer::start_hmr_server_static(hmr_port, hmr_state2).await
    });

    println!("\n🚀 Server running at https://localhost:{}", server_port);
    if hot_reload {
        println!("   Hot Reload: enabled (ws://localhost:{})", hmr_port);
    }
    println!("\nPress Ctrl+C to stop\n");

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
                                    state.emit(HmrEvent::StyleReload(
                                        path.to_string_lossy().to_string(),
                                    ));
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

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n👋 Server stopped");
        }
        result = proxy_handle => {
            match result {
                Ok(Ok(())) => {
                    eprintln!("❌ Proxy stopped unexpectedly (port {} may be in use)", server_port);
                }
                Ok(Err(e)) => {
                    eprintln!("❌ Proxy failed: {}", e);
                }
                Err(e) => {
                    eprintln!("❌ Proxy task crashed: {}", e);
                }
            }
        }
        result = hmr_handle => {
            match result {
                Ok(Ok(())) => {
                    eprintln!("❌ HMR server stopped (port {} may be in use)", hmr_port);
                }
                Ok(Err(e)) => {
                    eprintln!("❌ HMR server failed: {}", e);
                }
                Err(e) => {
                    eprintln!("❌ HMR task crashed: {}", e);
                }
            }
        }
    }

    php_worker.stop();

    Ok(())
}
