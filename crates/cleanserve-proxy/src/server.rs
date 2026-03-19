use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
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
    const ws = new WebSocket('ws://' + location.host + '/__cleanserve_hmr');
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
})();
"#;

pub struct ProxyServer {
    port: u16,
    root: Arc<String>,
    hmr_state: Arc<RwLock<HmrState>>,
}

impl ProxyServer {
    pub fn new(port: u16, root: String) -> Self {
        Self {
            port,
            root: Arc::new(root),
            hmr_state: Arc::new(RwLock::new(HmrState::new())),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;
        info!("🚀 CleanServe proxy listening on http://{}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let io = TokioIo::new(stream);
                    let root = Arc::clone(&self.root);
                    let hmr_state = Arc::clone(&self.hmr_state);
                    
                    tokio::spawn(async move {
                        let service = service_fn(move |req| {
                            let root = Arc::clone(&root);
                            let hmr_state = Arc::clone(&hmr_state);
                            handle_request(req, root, hmr_state)
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
                });
            }
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    root: Arc<String>,
    _hmr_state: Arc<RwLock<HmrState>>,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    let path = req.uri().path();
    let method = req.method();

    if method == Method::GET {
        let file_path = PathBuf::from(root.as_str()).join(path);
        
        if file_path.starts_with(root.as_str()) && file_path.is_file() {
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

    let html = format!(r#"
        <!DOCTYPE html>
        <html>
        <head><title>CleanServe</title></head>
        <body>
            <h1>🚀 CleanServe Running</h1>
            <p>PHP worker starting...</p>
            <p>Root: {}</p>
        </body>
        </html>
    "#, root.as_str());
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(html)))
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
