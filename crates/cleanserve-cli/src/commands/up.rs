use anyhow::Context;
use cleanserve_core::{
    ensure_writable_dirs, find_server_ports, prepare_project_php_ini, resolve_apps, FrameworkInfo,
    PhpDownloader, PhpManager, PhpWorker, SslManager,
};

use super::project_env::ensure_document_root;
use cleanserve_proxy::{AppRoute, HmrEvent, HmrState, ProxyServer};
use cleanserve_watcher::{FileEvent, FileWatcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

const PHP_BACKEND_PORT: u16 = 9000;

pub async fn run(port: Option<u16>) -> anyhow::Result<()> {
    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load config")?;

    let preferred_port = port.unwrap_or(config.server.port);
    let hot_reload = config.server.hot_reload;
    let php_version = &config.engine.php;
    let project_root = std::env::current_dir().context("Failed to get project directory")?;

    super::stack::auto_start(&project_root).await?;

    let apps = resolve_apps(&config, &project_root);
    if apps.is_empty() {
        anyhow::bail!(
            "No PHP application found. Add an `apps` map to cleanserve.json or create public/index.php."
        );
    }

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
            preferred_port, server_port
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

    if apps.iter().any(|app| app.domain != "localhost") {
        let suffix = config.server.domain_suffix.clone();
        tokio::spawn(async move {
            if let Err(e) = cleanserve_core::local_dns::serve(&suffix).await {
                info!("Local DNS server unavailable: {}", e);
            }
        });
    }

    let hmr_state = Arc::new(RwLock::new(HmrState::new()));

    let mut workers: Vec<PhpWorker> = Vec::new();
    let mut routes: Vec<AppRoute> = Vec::new();
    let mut app_roots: Vec<PathBuf> = Vec::new();
    let mut next_php_port = PHP_BACKEND_PORT;

    for app in &apps {
        let root_arg = relative_root(&project_root, &app.root);
        let php_root = ensure_document_root(
            &project_root,
            &root_arg,
            &config.name,
            &resolved,
        )
        .with_context(|| format!("Failed to prepare app '{}'", app.name))?;

        if workers.len() > 0 {
            println!();
        }
        println!("📱 App '{}' -> https://{}", app.name, app.domain);
        println!("   Root: {}", php_root.display());

        let mut worker = PhpWorker::new(php_path.clone(), php_root.clone())
            .with_php_ini(php_runtime.ini_path.clone());
        if let Some(router) = &app.router {
            worker = worker.with_router(router.clone());
        }

        let php_backend_port = worker
            .start_on_port(next_php_port)
            .with_context(|| format!("Failed to start PHP worker for '{}'", app.name))?;
        next_php_port = php_backend_port.saturating_add(10);

        println!(
            "🔧 PHP worker for '{}' started (127.0.0.1:{})",
            app.name, php_backend_port
        );

        routes.push(AppRoute {
            domain: app.domain.clone(),
            root: php_root.to_string_lossy().into_owned(),
            php_backend_port,
        });
        app_roots.push(php_root);
        workers.push(worker);
    }

    if Path::new("composer.json").exists() {
        if Path::new("vendor/autoload.php").exists() {
            println!("🎼 Composer dependencies ready");
        } else {
            println!("⚠️  vendor/ missing — run 'cleanserve composer install'");
        }
    }

    let domains: Vec<String> = apps.iter().map(|app| app.domain.clone()).collect();
    let ssl = SslManager::new().context("Failed to init SSL manager")?;
    let (key_path, cert_path) = ssl
        .get_or_create_cert_for_domains(&domains)
        .context("Failed to prepare TLS certificate")?;
    println!(
        "🔒 TLS certificate: {} ({})",
        cert_path.display(),
        domains.join(", ")
    );

    let default_root = routes
        .first()
        .map(|route| route.root.clone())
        .unwrap_or_else(|| project_root.to_string_lossy().into_owned());
    let default_php_port = routes
        .first()
        .map(|route| route.php_backend_port)
        .unwrap_or(PHP_BACKEND_PORT);

    let proxy = ProxyServer::new_with_hmr(
        server_port,
        default_root,
        hmr_state.clone(),
    )
    .with_tls(cert_path, key_path)
    .with_php_backend(default_php_port)
    .with_routes(routes);

    let proxy_handle = tokio::spawn(async move { proxy.start().await });

    let hmr_state2 = hmr_state.clone();
    let hmr_handle = tokio::spawn(async move {
        ProxyServer::start_hmr_server_static(hmr_port, hmr_state2).await
    });

    let display_port = if server_port == 443 {
        String::new()
    } else {
        format!(":{}", server_port)
    };
    println!("\n🚀 Server running:");
    for app in &apps {
        println!("   https://{}{}", app.domain, display_port);
    }
    if hot_reload {
        println!("   Hot Reload: enabled (ws://localhost:{})", hmr_port);
    }
    println!("\nPress Ctrl+C to stop\n");

    let mut watcher_guards = Vec::new();
    if hot_reload {
        for root in &app_roots {
            let watcher = FileWatcher::new(root);
            let hmr = hmr_state.clone();
            match watcher.watch() {
                Ok((mut rx, guard)) => {
                    info!("👀 File watcher started for {}", root.display());
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
                    watcher_guards.push(guard);
                }
                Err(e) => {
                    info!(
                        "⚠️ File watcher failed for {}: {}. Hot reload disabled for this app.",
                        root.display(),
                        e
                    );
                }
            }
        }
    }

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

    for worker in &mut workers {
        worker.stop();
    }

    Ok(())
}

fn relative_root(project_root: &Path, root: &Path) -> String {
    let rel = root.strip_prefix(project_root).unwrap_or(root);
    let value = rel.to_string_lossy();
    if value.is_empty() {
        "./".to_string()
    } else {
        value.into_owned()
    }
}
