use cleanserve_core::{RateLimiter, RequestValidator, StaticBlacklist, PathTraversal, SlowlorisProtection};
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, error, warn};

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
    const wsPort = parseInt(location.port || '80') + 1;
    const ws = new WebSocket('ws://' + location.hostname + ':' + wsPort + '/__cleanserve_hmr');
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
        setTimeout(() => location.reload(), 3000);
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
}

impl ProxyServer {
    pub fn new(port: u16, root: String) -> Self {
        Self {
            port,
            root: Arc::new(root),
            hmr_state: Arc::new(RwLock::new(HmrState::new())),
            rate_limiter: Arc::new(RateLimiter::new(1000, 60)),
            request_validator: Arc::new(RequestValidator::new(10_000_000, 50_000)),
            slowloris_protection: Arc::new(SlowlorisProtection::new(30_000)),
        }
    }

    /// Create proxy with shared HMR state (for hot reload integration)
    pub fn new_with_hmr(port: u16, root: String, hmr_state: Arc<RwLock<HmrState>>) -> Self {
        Self {
            port,
            root: Arc::new(root),
            hmr_state,
            rate_limiter: Arc::new(RateLimiter::new(1000, 60)),
            request_validator: Arc::new(RequestValidator::new(10_000_000, 50_000)),
            slowloris_protection: Arc::new(SlowlorisProtection::new(30_000)),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;
        info!("🚀 CleanServe proxy listening on http://{}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let io = TokioIo::new(stream);
                    let root = Arc::clone(&self.root);
                    let hmr_state = Arc::clone(&self.hmr_state);
                    let rate_limiter = Arc::clone(&self.rate_limiter);
                    let request_validator = Arc::clone(&self.request_validator);
                    let slowloris_protection = Arc::clone(&self.slowloris_protection);
                    
                    slowloris_protection.register_connection(remote_addr);
                    
                    tokio::spawn(async move {
                        let service = service_fn(move |req| {
                            let root = Arc::clone(&root);
                            let hmr_state = Arc::clone(&hmr_state);
                            let rate_limiter = Arc::clone(&rate_limiter);
                            let request_validator = Arc::clone(&request_validator);
                            let slowloris_protection = Arc::clone(&slowloris_protection);
                            handle_request(req, root, hmr_state, rate_limiter, request_validator, slowloris_protection, remote_addr)
                        });
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
        let addr = SocketAddr::from(([127, 0, 0, 1], ws_port));
        let listener = TcpListener::bind(addr).await?;
        info!("🔌 HMR WebSocket server on ws://{}", addr);

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let hmr_state = Arc::clone(&hmr_state);
                tokio::spawn(async move {
                    Self::handle_ws_connection(stream, hmr_state).await;
                });
            }
        }
    }

    async fn handle_ws_connection(
        stream: tokio::net::TcpStream,
        hmr_state: Arc<RwLock<HmrState>>,
    ) {
        match accept_async(stream).await {
            Ok(ws_stream) => {
                let (mut write, _read) = ws_stream.split();
                let rx = {
                    let state = hmr_state.read().await;
                    state.subscribe()
                };
                let ack = Message::Text("{\"type\":\"connected\"}".into());
                let _ = write.send(ack).await;

                tokio::spawn(async move {
                    let mut rx = rx;
                    while let Ok(event) = rx.recv().await {
                        let msg: String = match event {
                            HmrEvent::PhpReload => r#"{"type":"reload"}"#.to_string(),
                            HmrEvent::StyleReload(path) => format!(r#"{{"type":"style","path":"{}"}}"#, path),
                        };
                        if write.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                });
            }
            Err(e) => {
                warn!("WebSocket handshake failed: {}", e);
            }
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    root: Arc<String>,
    _hmr_state: Arc<RwLock<HmrState>>,
    rate_limiter: Arc<RateLimiter>,
    request_validator: Arc<RequestValidator>,
    slowloris_protection: Arc<SlowlorisProtection>,
    remote_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    let ip = remote_addr.ip();
    let is_localhost = ip.is_loopback();

    if !slowloris_protection.is_connection_valid(remote_addr) {
        warn!("Slowloris attack detected - connection timeout: {}", remote_addr);
        slowloris_protection.mark_request_complete(remote_addr);
        return Ok(Response::builder()
            .status(StatusCode::REQUEST_TIMEOUT)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error":"request_timeout","message":"Request headers timeout"}"#,
            )))
            .expect("valid 408 response"));
    }

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

     if method == Method::GET || method == Method::POST {
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

        if let Some(ext) = file_path.extension() {
            if ext == "php" {
                match forward_to_php_worker(req, &request_path).await {
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

        // Static file serving (only for GET)
        if method == Method::GET && file_path.starts_with(root.as_str()) && file_path.is_file() {
            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let mime = mime_guess::from_path(&file_path)
                        .first_or_octet_stream()
                        .to_string();

                    let resp = Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", mime.as_str());

                    if mime.starts_with("text/html") {
                        if let Ok(body) = String::from_utf8(content.clone()) {
                            let injected = inject_hmr_script(&body);
                            return Ok(resp.body(Full::new(Bytes::from(injected))).unwrap());
                        }
                    }

                    return Ok(resp.body(Full::new(Bytes::from(content))).unwrap());
                }
                Err(_) => {}
            }
        }
    }

    slowloris_protection.mark_request_complete(remote_addr);
    
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(
            "<h1>404 Not Found</h1><p>The requested resource was not found on this server.</p>",
        )))
        .unwrap())
}

fn inject_hmr_script(html: &str) -> String {
    let script_tag = format!(r#"<script>{}</script>"#, HMR_CLIENT_SCRIPT);
    if let Some(pos) = html.find("</body>") {
        format!("{}{}{}", &html[..pos], script_tag, &html[pos..])
    } else {
        format!("{}{}", html, script_tag)
    }
}

/// Forward request to PHP built-in server on port 9000
async fn forward_to_php_worker(
    req: Request<hyper::body::Incoming>,
    rewritten_path: &str,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error>> {
    use http_body_util::BodyExt;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;

    let method = req.method().clone();
    let headers = req.headers().clone();

    let php_url = format!("http://127.0.0.1:9000{}", rewritten_path);

    // Collect request body
    let body_bytes = req.into_body().collect().await?.to_bytes();

    // Create client and forward request
    let client = Client::builder(TokioExecutor::new()).build_http();

    let mut php_req = hyper::Request::builder()
        .method(method)
        .uri(&php_url);

    // Copy relevant headers
    for (key, value) in headers.iter() {
        if !key.as_str().starts_with("host") {
            php_req = php_req.header(key, value);
        }
    }
    php_req = php_req.header("Host", "127.0.0.1:9000");

    let php_req = php_req.body(http_body_util::Full::new(body_bytes))?;

    let php_resp = client.request(php_req).await?;

    // Build response to return
    let status = php_resp.status();
    let resp_headers = php_resp.headers().clone();
    let resp_body = php_resp.into_body().collect().await?.to_bytes();

    let mut builder = Response::builder().status(status);
    for (key, value) in resp_headers.iter() {
        builder = builder.header(key, value);
    }

    // Inject HMR script into HTML responses from PHP
    let content_type = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let final_body = if content_type.contains("text/html") {
        if let Ok(html) = String::from_utf8(resp_body.to_vec()) {
            Bytes::from(inject_hmr_script(&html))
        } else {
            resp_body
        }
    } else {
        resp_body
    };

    Ok(builder.body(Full::new(final_body))?)
}
