use cleanserve_core::{
    RateLimiter, RequestValidator, StaticBlacklist, PathTraversal, SlowlorisProtection,
    RequestEntry, RuntimeTelemetry, is_static_file,
};
use cleanserve_core::php_worker::DEFAULT_PHP_PORT;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, error, warn};
use crate::tls_config::create_tls_config;
use tokio_rustls::TlsAcceptor;

/// HMR event types
#[derive(Clone, Debug)]
pub enum HmrEvent {
    PhpReload,
    StyleReload(String),
}

/// Hot module replacement state
pub struct HmrState {
    tx: broadcast::Sender<HmrEvent>,
}

impl HmrState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HmrEvent> {
        self.tx.subscribe()
    }

    pub fn emit(&self, event: HmrEvent) {
        match &event {
            HmrEvent::PhpReload => info!("📢 Broadcasting PHP reload event"),
            HmrEvent::StyleReload(path) => info!("📢 Broadcasting style reload event: {}", path),
        }
        let _ = self.tx.send(event);
    }
}

impl Default for HmrState {
    fn default() -> Self {
        Self::new()
    }
}

const HMR_CLIENT_SCRIPT: &str = r#"
(function() {
    const wsPort = parseInt(location.port || (location.protocol === 'https:' ? '443' : '80')) + 1;
    const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(wsProto + '//' + location.hostname + ':' + wsPort + '/__cleanserve_hmr');
    ws.onmessage = (event) => {
        const data = JSON.parse(event.data);
        if (data.type === 'reload') {
            console.log('[CleanServe] Page reload requested');
            location.reload();
        } else if (data.type === 'style' && data.path) {
            console.log('[CleanServe] CSS updated:', data.path);
            const links = document.querySelectorAll('link[rel="stylesheet"]');
            links.forEach(link => {
                const href = link.href.split('?')[0];
                if (href.includes(data.path)) {
                    link.href = href + '?t=' + Date.now();
                }
            });
        }
    };
    ws.onclose = () => {
        console.log('[CleanServe] HMR disconnected, reconnecting...');
        setTimeout(function () {
            try {
                var ws2 = new WebSocket(wsProto + '//' + location.hostname + ':' + wsPort + '/__cleanserve_hmr');
                ws2.onmessage = ws.onmessage;
                ws2.onopen = ws.onopen;
                ws2.onclose = ws.onclose;
            } catch (e) {
                console.log('[CleanServe] HMR reconnect failed', e);
            }
        }, 1500);
    };
    ws.onopen = () => {
        console.log('[CleanServe] HMR connected on port ' + wsPort);
    };
})();
"#;

pub struct ProxyServer {
    port: u16,
    root: Arc<String>,
    hmr_state: Arc<RwLock<HmrState>>,
    rate_limiter: Arc<RateLimiter>,
    request_validator: Arc<RequestValidator>,
    slowloris_protection: Arc<SlowlorisProtection>,
    not_found_html: Arc<String>,
    telemetry: Option<Arc<RuntimeTelemetry>>,
    php_port: u16,
}

/// Where requests are forwarded and how the client reached us.
#[derive(Clone, Copy)]
struct Upstream {
    php_port: u16,
    https: bool,
}

/// Max time a client may take to send the request headers (real slowloris defense).
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);

impl ProxyServer {
    pub fn new(port: u16, root: String) -> Self {
        let not_found_html = Self::load_404_html();
        Self {
            port,
            root: Arc::new(root),
            hmr_state: Arc::new(RwLock::new(HmrState::new())),
            rate_limiter: Arc::new(RateLimiter::new(1000, 60)),
            request_validator: Arc::new(RequestValidator::new(10_000_000, 50_000)),
            slowloris_protection: Arc::new(SlowlorisProtection::new(30_000)),
            not_found_html: Arc::new(not_found_html),
            telemetry: None,
            php_port: DEFAULT_PHP_PORT,
        }
    }

    /// Port of the PHP built-in server requests are forwarded to.
    pub fn with_php_port(mut self, php_port: u16) -> Self {
        self.php_port = php_port;
        self
    }

    /// Create proxy with shared HMR state (for hot reload integration)
    pub fn new_with_hmr(port: u16, root: String, hmr_state: Arc<RwLock<HmrState>>) -> Self {
        Self::new_with_hmr_and_telemetry(port, root, hmr_state, None)
    }

    pub fn new_with_hmr_and_telemetry(
        port: u16,
        root: String,
        hmr_state: Arc<RwLock<HmrState>>,
        telemetry: Option<Arc<RuntimeTelemetry>>,
    ) -> Self {
        let not_found_html = Self::load_404_html();
        Self {
            port,
            root: Arc::new(root),
            hmr_state,
            rate_limiter: Arc::new(RateLimiter::new(1000, 60)),
            request_validator: Arc::new(RequestValidator::new(10_000_000, 50_000)),
            slowloris_protection: Arc::new(SlowlorisProtection::new(30_000)),
            not_found_html: Arc::new(not_found_html),
            telemetry,
            php_port: DEFAULT_PHP_PORT,
        }
    }

    /// Load 404.html from .cleanserve/pages/ directory
    fn load_404_html() -> String {
        let path = PathBuf::from(".cleanserve/pages/404.html");
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                info!("✓ Loaded 404.html from {}", path.display());
                content
            }
            Err(e) => {
                warn!("Failed to load 404.html from {}: {}. Using fallback.", path.display(), e);
                "<h1>404 Not Found</h1><p>The requested resource was not found on this server.</p>".to_string()
            }
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        self.start_with_tls_paths(None, None).await
    }

    pub async fn start_https(&self, cert_path: &str, key_path: &str) -> anyhow::Result<()> {
        self.start_with_tls_paths(Some(cert_path), Some(key_path)).await
    }

    async fn start_with_tls_paths(
        &self,
        cert_path: Option<&str>,
        key_path: Option<&str>,
    ) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;

        let tls_acceptor = match (cert_path, key_path) {
            (Some(cert), Some(key)) => {
                let config = create_tls_config(cert, key).map_err(|e| anyhow::anyhow!("{}", e))?;
                info!("🔒 CleanServe proxy listening on https://{}", addr);
                Some(Arc::new(TlsAcceptor::from(Arc::new(config))))
            }
            _ => {
                info!("🚀 CleanServe proxy listening on http://{}", addr);
                None
            }
        };

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let root = Arc::clone(&self.root);
                    let hmr_state = Arc::clone(&self.hmr_state);
                    let rate_limiter = Arc::clone(&self.rate_limiter);
                    let request_validator = Arc::clone(&self.request_validator);
                    let slowloris_protection = Arc::clone(&self.slowloris_protection);
                    let not_found_html = Arc::clone(&self.not_found_html);
                    let telemetry = self.telemetry.clone();
                    let tls_acceptor = tls_acceptor.clone();
                    let php_port = self.php_port;
                    let https = tls_acceptor.is_some();
                    let upstream = Upstream { php_port, https };

                    let slowloris_cleanup = Arc::clone(&slowloris_protection);
                    slowloris_protection.register_connection(remote_addr);

                    if let Some(acceptor) = tls_acceptor.clone() {
                        tokio::spawn(async move {
                            let io = match acceptor.accept(stream).await {
                                Ok(tls_stream) => TokioIo::new(tls_stream),
                                Err(e) => {
                                    if let Some(t) = &telemetry {
                                        t.record_tls_handshake_failure();
                                    } else {
                                        error!("TLS handshake failed: {}", e);
                                    }
                                    slowloris_protection.remove_connection(remote_addr);
                                    return;
                                }
                            };
                            let stderr_logging = telemetry.is_none();
                            let service = service_fn(move |req| {
                                let root = Arc::clone(&root);
                                let hmr_state = Arc::clone(&hmr_state);
                                let rate_limiter = Arc::clone(&rate_limiter);
                                let request_validator = Arc::clone(&request_validator);
                                let slowloris_protection = Arc::clone(&slowloris_protection);
                                let not_found_html = Arc::clone(&not_found_html);
                                let telemetry = telemetry.clone();
                                handle_request(req, root, hmr_state, rate_limiter, request_validator, slowloris_protection, remote_addr, not_found_html, telemetry, upstream)
                            });
                            if let Err(e) = http1::Builder::new()
                                .timer(TokioTimer::new())
                                .header_read_timeout(HEADER_READ_TIMEOUT)
                                .serve_connection(io, service)
                                .await
                            {
                                if stderr_logging {
                                    error!("Error serving connection: {}", e);
                                }
                            }
                            slowloris_cleanup.remove_connection(remote_addr);
                        });
                    } else {
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            let stderr_logging = telemetry.is_none();
                            let service = service_fn(move |req| {
                                let root = Arc::clone(&root);
                                let hmr_state = Arc::clone(&hmr_state);
                                let rate_limiter = Arc::clone(&rate_limiter);
                                let request_validator = Arc::clone(&request_validator);
                                let slowloris_protection = Arc::clone(&slowloris_protection);
                                let not_found_html = Arc::clone(&not_found_html);
                                let telemetry = telemetry.clone();
                                handle_request(req, root, hmr_state, rate_limiter, request_validator, slowloris_protection, remote_addr, not_found_html, telemetry, upstream)
                            });
                            if let Err(e) = http1::Builder::new()
                                .timer(TokioTimer::new())
                                .header_read_timeout(HEADER_READ_TIMEOUT)
                                .serve_connection(io, service)
                                .await
                            {
                                if stderr_logging {
                                    error!("Error serving connection: {}", e);
                                }
                            }
                            slowloris_cleanup.remove_connection(remote_addr);
                        });
                    }
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    pub async fn start_hmr_server(&self, ws_port: u16) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], ws_port));
        let listener = TcpListener::bind(addr).await?;
        info!("🔌 HMR WebSocket server on ws://{}", addr);

        let hmr_state = Arc::clone(&self.hmr_state);
        
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let hmr_state = Arc::clone(&hmr_state);
                tokio::spawn(async move {
                    Self::handle_ws_connection(stream, hmr_state).await;
                });
            }
        }
    }

    /// Static HMR server that accepts external HmrState
    pub async fn start_hmr_server_static(
        ws_port: u16,
        hmr_state: Arc<RwLock<HmrState>>,
    ) -> anyhow::Result<()> {
        Self::start_hmr_server_static_tls(ws_port, hmr_state, None, None).await
    }

    pub async fn start_hmr_server_static_tls(
        ws_port: u16,
        hmr_state: Arc<RwLock<HmrState>>,
        cert_path: Option<&str>,
        key_path: Option<&str>,
    ) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], ws_port));
        let listener = TcpListener::bind(addr).await?;

        let tls_acceptor = match (cert_path, key_path) {
            (Some(cert), Some(key)) => {
                let config = create_tls_config(cert, key).map_err(|e| anyhow::anyhow!("{}", e))?;
                info!("🔌 HMR WebSocket server on wss://{}", addr);
                Some(Arc::new(TlsAcceptor::from(Arc::new(config))))
            }
            _ => {
                info!("🔌 HMR WebSocket server on ws://{}", addr);
                None
            }
        };

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let hmr_state = Arc::clone(&hmr_state);
                if let Some(acceptor) = tls_acceptor.clone() {
                    tokio::spawn(async move {
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => Self::handle_ws_connection(tls_stream, hmr_state).await,
                            Err(e) => warn!("HMR TLS handshake failed: {}", e),
                        }
                    });
                } else {
                    tokio::spawn(async move {
                        Self::handle_ws_connection(stream, hmr_state).await;
                    });
                }
            }
        }
    }

    async fn handle_ws_connection<S>(
        stream: S,
        hmr_state: Arc<RwLock<HmrState>>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        match accept_async(stream).await {
            Ok(ws_stream) => {
                let (mut write, _read) = ws_stream.split();
                let rx = {
                    let state = hmr_state.read().await;
                    state.subscribe()
                };
                let ack = Message::Text("{\"type\":\"connected\"}".into());
                let _ = write.send(ack).await;
                info!("✅ WebSocket client connected, listening for HMR events");

                let mut rx = rx;
                while let Ok(event) = rx.recv().await {
                    let msg: String = match event {
                        HmrEvent::PhpReload => {
                            info!("📤 Sending PHP reload to WebSocket client");
                            r#"{"type":"reload"}"#.to_string()
                        },
                        HmrEvent::StyleReload(path) => {
                            info!("📤 Sending style reload to WebSocket client: {}", path);
                            format!(r#"{{"type":"style","path":"{}"}}"#, path)
                        },
                    };
                    if write.send(Message::Text(msg.into())).await.is_err() {
                        info!("⚠️  WebSocket send failed, client disconnected");
                        break;
                    }
                }
            }
            Err(e) => {
                warn!("WebSocket handshake failed: {}", e);
            }
        }
    }
}

/// Handle one request and record it (method, path, status, duration) for the dashboard.
#[allow(clippy::too_many_arguments)]
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    root: Arc<String>,
    hmr_state: Arc<RwLock<HmrState>>,
    rate_limiter: Arc<RateLimiter>,
    request_validator: Arc<RequestValidator>,
    slowloris_protection: Arc<SlowlorisProtection>,
    remote_addr: SocketAddr,
    not_found_html: Arc<String>,
    telemetry: Option<Arc<RuntimeTelemetry>>,
    upstream: Upstream,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    let started = std::time::Instant::now();
    let at = std::time::SystemTime::now();
    let method = req.method().as_str().to_string();
    // Path only: the query string may carry tokens we do not want on screen.
    let path = req.uri().path().to_string();

    let result = handle_request_inner(
        req, root, hmr_state, rate_limiter, request_validator, slowloris_protection,
        remote_addr, not_found_html, telemetry.clone(), upstream,
    )
    .await;

    if let (Some(t), Ok(resp)) = (&telemetry, &result) {
        t.push_request(RequestEntry {
            at,
            method,
            path,
            status: resp.status().as_u16(),
            duration: started.elapsed(),
        });
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn handle_request_inner(
    req: Request<hyper::body::Incoming>,
    root: Arc<String>,
    _hmr_state: Arc<RwLock<HmrState>>,
    rate_limiter: Arc<RateLimiter>,
    request_validator: Arc<RequestValidator>,
    slowloris_protection: Arc<SlowlorisProtection>,
    remote_addr: SocketAddr,
    not_found_html: Arc<String>,
    telemetry: Option<Arc<RuntimeTelemetry>>,
    upstream: Upstream,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    if let Some(t) = &telemetry {
        t.record_request();
    }

    let ip = remote_addr.ip();
    let is_localhost = ip.is_loopback();

    // The request headers are fully received once hyper hands us the request (the real
    // slow-header defense is hyper's `header_read_timeout`), so restart the idle timer.
    slowloris_protection.mark_request_complete(remote_addr);

    if !is_localhost && !rate_limiter.is_allowed(&ip.to_string()).await {
        warn!("Rate limit exceeded for IP: {}", ip);
        slowloris_protection.mark_request_complete(remote_addr);
        return Ok(Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error":"rate_limit_exceeded","message":"Too many requests"}"#,
            )))
            .expect("valid 429 response"));
    }

    let mut header_map: HashMap<String, String> = HashMap::new();
    for (k, v) in req.headers() {
        if let Ok(val) = v.to_str() {
            header_map.insert(k.as_str().to_lowercase(), val.to_string());
        }
    }

    if let Err(msg) = request_validator.validate_content_length(&header_map) {
        warn!("Request validation failed: {}", msg);
        return Ok(Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                format!(r#"{{"error":"payload_too_large","message":"{}"}}"#, msg),
            )))
            .expect("valid 413 response"));
    }

    if let Err(msg) = request_validator.validate_content_type(req.method().as_str(), &header_map) {
        warn!("Request validation failed: {}", msg);
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                format!(r#"{{"error":"bad_request","message":"{}"}}"#, msg),
            )))
            .expect("valid 400 response"));
    }

    if let Err(msg) = request_validator.validate_header_size(&header_map) {
        warn!("Request validation failed: {}", msg);
        return Ok(Response::builder()
            .status(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                format!(r#"{{"error":"header_too_large","message":"{}"}}"#, msg),
            )))
            .expect("valid 431 response"));
     }

     let path = req.uri().path();
     let method = req.method();

     if !PathTraversal::is_valid_request_path(path) {
         warn!("Invalid path detected - traversal attack: {}", path);
         return Ok(Response::builder()
             .status(StatusCode::BAD_REQUEST)
             .header("Content-Type", "application/json")
             .body(Full::new(Bytes::from(
                 r#"{"error":"bad_request","message":"Invalid path format"}"#
             )))
             .expect("valid 400 response"));
     }

     // Check static file blacklist
     if StaticBlacklist::is_blocked(path) {
         warn!("Attempted to access blacklisted path: {}", path);
         return Ok(Response::builder()
             .status(StatusCode::FORBIDDEN)
             .header("Content-Type", "application/json")
             .body(Full::new(Bytes::from(
                 r#"{"error":"forbidden","message":"This file cannot be accessed"}"#,
             )))
             .expect("valid 403 response"));
     }

      // Check for dangerous uploads
      if StaticBlacklist::is_in_upload_dir(path) && StaticBlacklist::is_dangerous_upload(path) {
          warn!("Attempted to serve dangerous upload: {}", path);
          return Ok(Response::builder()
              .status(StatusCode::FORBIDDEN)
              .header("Content-Type", "application/json")
              .body(Full::new(Bytes::from(
                  r#"{"error":"forbidden","message":"Executable uploads are not allowed"}"#,
              )))
              .expect("valid 403 response"));
      }

      info!("Processing request: {} {}, root={}", method, path, root.as_str());

     {
        let mut request_path = path.to_string();

        if request_path == "/" || request_path.ends_with('/') {
            let index_php = PathBuf::from(root.as_str()).join(request_path.trim_start_matches('/'));
            let index_php = index_php.join("index.php");
            if index_php.exists() {
                request_path = if request_path == "/" {
                    "/index.php".to_string()
                } else {
                    format!("{}index.php", request_path)
                };
            } else {
                let index_html = PathBuf::from(root.as_str())
                    .join(request_path.trim_start_matches('/'))
                    .join("index.html");
                if index_html.exists() {
                    request_path = if request_path == "/" {
                        "/index.html".to_string()
                    } else {
                        format!("{}index.html", request_path)
                    };
                }
            }
        }

        let file_path = PathBuf::from(root.as_str()).join(request_path.trim_start_matches('/'));

        let is_php_script = file_path
            .extension()
            .is_some_and(|ext| ext == "php");

        // Static file serving (only for GET, existing files, non-PHP)
        let is_get = method == Method::GET || method == Method::HEAD;
        let path_starts = file_path.to_string_lossy().starts_with(root.as_str());
        let is_file = file_path.is_file();
        
        if is_get {
            info!("GET request: file_path={}, root={}, starts_with={}, is_file={}", 
                  file_path.display(), root.as_str(), path_starts, is_file);
        }
        
        if is_get && path_starts && is_file && !is_php_script {
            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let mime = mime_guess::from_path(&file_path)
                        .first_or_octet_stream()
                        .to_string();

                    info!("📄 Serving static file: {} (mime: {})", file_path.display(), mime);

                    let resp = Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", mime.as_str());

                    if mime.starts_with("text/html") {
                        if let Ok(body) = String::from_utf8(content.clone()) {
                            info!("🔧 Injecting HMR script into HTML response");
                            let injected = inject_hmr_script(&body);
                            return Ok(resp.body(Full::new(Bytes::from(injected))).unwrap());
                        }
                    }

                    return Ok(resp.body(Full::new(Bytes::from(content))).unwrap());
                }
                Err(e) => {
                    warn!("Failed to read static file {}: {}", file_path.display(), e);
                }
            }
        }

        // Anything that is not a GET/HEAD for a static file belongs to the application
        // (PUT/PATCH/DELETE/OPTIONS/POST APIs, front-controller routes, PHP scripts).
        let route_to_php = is_php_script
            || !is_get
            || (!is_file && !is_static_file(path));

        if route_to_php {
            match forward_to_php_worker(req, &request_path, remote_addr, upstream).await {
                Ok(response) => {
                    slowloris_protection.mark_request_complete(remote_addr);
                    return Ok(response);
                }
                Err(e) => {
                    error!("PHP worker error: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header("Content-Type", "text/html")
                        .body(Full::new(Bytes::from(format!(
                            "<h1>PHP Error</h1><p>{}</p>",
                            e
                        ))))
                        .expect("valid 500 response"));
                }
            }
        }
    }

    slowloris_protection.mark_request_complete(remote_addr);
    
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from((*not_found_html).clone())))
        .unwrap())
}

fn inject_hmr_script(html: &str) -> String {
    if html.contains("__cleanserve_hmr") {
        return html.to_string();
    }
    let script_tag = format!(r#"<script>{}</script>"#, HMR_CLIENT_SCRIPT);
    if let Some(pos) = html.to_ascii_lowercase().rfind("</body>") {
        format!("{}{}{}", &html[..pos], script_tag, &html[pos..])
    } else {
        format!("{}{}", html, script_tag)
    }
}

/// Hop-by-hop headers must not be forwarded by a proxy (RFC 9110 §7.6.1).
fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Forward request to the PHP built-in server.
async fn forward_to_php_worker(
    req: Request<hyper::body::Incoming>,
    rewritten_path: &str,
    remote_addr: SocketAddr,
    upstream: Upstream,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error>> {
    use http_body_util::BodyExt;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;

    let method = req.method().clone();
    let headers = req.headers().clone();
    let uri = req.uri().clone();

    let mut php_path = rewritten_path.to_string();
    if let Some(query) = uri.query() {
        if !query.is_empty() {
            php_path.push('?');
            php_path.push_str(query);
        }
    }

    let php_url = format!("http://127.0.0.1:{}{}", upstream.php_port, php_path);

    // Collect request body
    let body_bytes = req.into_body().collect().await?.to_bytes();

    let client = Client::builder(TokioExecutor::new()).build_http();

    let mut php_req = hyper::Request::builder().method(method).uri(&php_url);

    // Keep the client's Host so the app generates correct URLs/redirects,
    // and tell it how the client actually reached us.
    let original_host = headers.get("host").cloned();
    for (key, value) in headers.iter() {
        let name = key.as_str();
        if name.eq_ignore_ascii_case("host")
            || name.eq_ignore_ascii_case("content-length")
            || name.starts_with("x-forwarded-")
            || is_hop_by_hop(name)
        {
            continue;
        }
        php_req = php_req.header(key, value);
    }
    if let Some(host) = original_host {
        php_req = php_req.header("Host", host.clone()).header("X-Forwarded-Host", host);
    } else {
        php_req = php_req.header("Host", format!("127.0.0.1:{}", upstream.php_port));
    }
    php_req = php_req
        .header("X-Forwarded-For", remote_addr.ip().to_string())
        .header("X-Forwarded-Proto", if upstream.https { "https" } else { "http" })
        .header("Content-Length", body_bytes.len().to_string());

    let php_req = php_req.body(http_body_util::Full::new(body_bytes))?;

    let php_resp = client.request(php_req).await?;

    let status = php_resp.status();
    let resp_headers = php_resp.headers().clone();
    let resp_body = php_resp.into_body().collect().await?.to_bytes();

    let mut builder = Response::builder().status(status);
    for (key, value) in resp_headers.iter() {
        let name = key.as_str();
        // Length is recomputed from the buffered body; hop-by-hop headers are not ours to forward.
        // PHP's built-in server also echoes a bogus `Host:` response header (its own address).
        if name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("host")
            || is_hop_by_hop(name)
        {
            continue;
        }
        builder = builder.header(key, value);
    }

    // Inject the HMR script into plain (not compressed) HTML responses from PHP.
    let header_str = |name: &str| {
        resp_headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    let is_html = header_str("content-type").contains("text/html");
    let is_encoded = {
        let enc = header_str("content-encoding");
        !enc.is_empty() && enc != "identity"
    };

    let final_body = if is_html && !is_encoded {
        match String::from_utf8(resp_body.to_vec()) {
            Ok(html) => Bytes::from(inject_hmr_script(&html)),
            Err(_) => resp_body,
        }
    } else {
        resp_body
    };

    Ok(builder.body(Full::new(final_body))?)
}
