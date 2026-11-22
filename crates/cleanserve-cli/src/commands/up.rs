use anyhow::Context;
use cleanserve_core::{
    ComposerTool, Framework, FrameworkInfo, PhpWorker, ProjectLayout, RuntimeTelemetry,
    SslManager, ensure_project_php, ensure_writable_dirs,
};
use cleanserve_proxy::{ProxyServer, HmrEvent, HmrState};
use cleanserve_watcher::{FileWatcher, FileEvent};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub async fn run(port: Option<u16>, no_tui: bool, http_only: bool) -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;

    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let config = cleanserve_core::CleanServeConfig::load(config_path)
        .context("Failed to load config")?;

    let port = port.unwrap_or(config.server.port);
    let _pid_guard = super::pidfile::PidGuard::acquire(&project_root)?;
    ensure_ports_free(port)?;
    let hot_reload = config.server.hot_reload;
    let php_version = config.engine.php.clone();
    let use_tls = config.server.tls && !http_only;

    let telemetry = Arc::new(RuntimeTelemetry::new());
    telemetry.push_log("Starting CleanServe...");

    let layout = ProjectLayout::detect(&project_root);

    let mut root = config.server.root.clone();
    if Path::new("composer.json").exists() {
        let detected = layout.document_root_for_config(&project_root);
        let detected_abs = project_root.join(detected.trim_end_matches('/'));
        if detected != root && detected_abs.exists() {
            telemetry.push_log(format!(
                "Document root {} (cleanserve.json had {})",
                detected,
                root
            ));
            root = detected;
        } else if layout.framework == Framework::Yii2 && !detected_abs.exists() {
            telemetry.push_log(
                "Yii2: run `cleanserve php init` to create frontend/web/index.php".to_string(),
            );
        }
    }
    let framework_info = FrameworkInfo::detect(&project_root);
    let _ = ensure_writable_dirs(&framework_info);

    let php_path = ensure_project_php(&project_root, &php_version)
        .await
        .context("Failed to prepare project PHP")?;

    ComposerTool::ensure_vendor(&php_path, &project_root)
        .await
        .context("Composer install failed")?;

    let document_root = project_root.join(root.trim_end_matches('/'));
    if !document_root.exists() {
        anyhow::bail!(
            "Document root {} does not exist. For Yii/Laravel projects run the framework init (e.g. ./init) or fix server.root in cleanserve.json.",
            document_root.display()
        );
    }
    let _ = cleanserve_core::ensure_dev_router(&project_root, &document_root);

    let router = layout
        .router_script
        .clone()
        .filter(|p| p.exists());

    let hmr_state = Arc::new(RwLock::new(HmrState::new()));

    let php_log = project_root.join(".cleanserve").join("php-server.log");
    let _ = std::fs::remove_file(&php_log);
    let mut php_worker = if let Some(router) = router {
        PhpWorker::with_router(php_path.clone(), document_root.clone(), router)
    } else {
        PhpWorker::new(php_path.clone(), document_root.clone())
    }
    .with_log_file(php_log.clone());
    php_worker
        .start()
        .context("Failed to start PHP worker")?;
    telemetry.push_log(format!(
        "PHP {} worker on 127.0.0.1:{} (log: {})",
        php_version,
        php_worker.port(),
        ".cleanserve/php-server.log"
    ));

    let tls_material: Option<(String, String)> = if use_tls {
        let ssl = SslManager::new().context("Failed to init SSL")?;
        let (key_path, cert_path) = ssl
            .get_or_create_cert("localhost")
            .context("Failed to prepare TLS certificate")?;
        telemetry.push_log("TLS enabled (self-signed localhost cert — accept once in the browser)");
        Some((cert_path.to_string_lossy().into_owned(), key_path.to_string_lossy().into_owned()))
    } else {
        telemetry.push_log("HTTP only — use https:// only when server.tls is true".to_string());
        None
    };

    let proxy = ProxyServer::new_with_hmr_and_telemetry(
        port,
        root.clone(),
        hmr_state.clone(),
        Some(telemetry.clone()),
    )
    .with_php_port(php_worker.port());

    let proxy_tls = tls_material.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Some((cert, key)) = proxy_tls {
            proxy.start_https(&cert, &key).await
        } else {
            proxy.start().await
        }
    });

    let hmr_port = port + 1;
    let hmr_state2 = hmr_state.clone();
    let hmr_tls = tls_material.clone();
    let hmr_handle = tokio::spawn(async move {
        if let Some((cert, key)) = hmr_tls {
            ProxyServer::start_hmr_server_static_tls(hmr_port, hmr_state2, Some(&cert), Some(&key))
                .await
        } else {
            ProxyServer::start_hmr_server_static(hmr_port, hmr_state2).await
        }
    });

    let scheme = if use_tls { "https" } else { "http" };
    let url = format!("{}://localhost:{}", scheme, port);
    telemetry.push_log(format!("Listening on {}", url));

    let _watcher_guard = if hot_reload {
        let watcher = FileWatcher::new(&root);
        let hmr = hmr_state.clone();
        match watcher.watch() {
            Ok((mut rx, guard)) => {
                info!("File watcher started");
                telemetry.push_log("Hot reload enabled");
                tokio::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        match event {
                            FileEvent::PhpChanged(paths) => {
                                info!("PHP files changed: {:?}", paths);
                                let state = hmr.read().await;
                                state.emit(HmrEvent::PhpReload);
                            }
                            FileEvent::StyleChanged(paths) => {
                                info!("Style files changed: {:?}", paths);
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
                telemetry.push_log(format!("Hot reload disabled: {}", e));
                None
            }
        }
    } else {
        None
    };

    let outcome = if no_tui {
        println!("\nCleanServe running at {}\nPress Ctrl+C to stop\n", url);
        tokio::select! {
            _ = shutdown_signal() => Ok(()),
            result = proxy_handle => flatten("Proxy", result),
            result = hmr_handle => flatten("HMR server", result),
        }
    } else {
        let shutdown = Arc::new(AtomicBool::new(false));
        let dashboard_state = crate::tui::UpDashboardState {
            project_name: config.name.clone(),
            project_root: project_root.clone(),
            php_version,
            document_root: root.clone(),
            url,
            framework: layout.framework.to_string(),
            hot_reload,
            telemetry: telemetry.clone(),
            shutdown: shutdown.clone(),
        };

        let mut tui_task = tokio::task::spawn_blocking(move || {
            crate::tui::run_up_dashboard(dashboard_state)
        });

        let mut tui_finished = false;
        let outcome = tokio::select! {
            result = &mut tui_task => {
                // The user quit from the dashboard.
                tui_finished = true;
                match result {
                    Ok(inner) => inner,
                    Err(e) => Err(anyhow::anyhow!("TUI task failed: {}", e)),
                }
            }
            _ = shutdown_signal() => Ok(()),
            result = proxy_handle => flatten("Proxy", result),
            result = hmr_handle => flatten("HMR server", result),
        };

        // Make sure the dashboard restores the terminal before we print anything or exit.
        if !tui_finished {
            shutdown.store(true, Ordering::SeqCst);
            let _ = tui_task.await;
        }
        outcome
    };

    php_worker.stop();
    outcome
}

/// Turn a finished server task into an error: a server task should only end on failure.
fn flatten(name: &str, result: Result<anyhow::Result<()>, tokio::task::JoinError>) -> anyhow::Result<()> {
    match result {
        Ok(Ok(())) => Err(anyhow::anyhow!("{} stopped unexpectedly", name)),
        Ok(Err(e)) => Err(e.context(format!("{} failed", name))),
        Err(e) => Err(anyhow::anyhow!("{} task failed: {}", name, e)),
    }
}

/// Resolves on Ctrl+C or (on Unix) SIGTERM, which is what `cleanserve down` sends.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

/// Fail early with an actionable message instead of dying silently inside a spawned task.
fn ensure_ports_free(port: u16) -> anyhow::Result<()> {
    for (p, what) in [(port, "HTTP"), (port.saturating_add(1), "hot-reload WebSocket")] {
        if let Err(e) = std::net::TcpListener::bind(("127.0.0.1", p)) {
            anyhow::bail!(
                "Port {} ({}) is not available: {}. Stop the other process or use `cleanserve up --port <PORT>`.",
                p, what, e
            );
        }
    }
    Ok(())
}
