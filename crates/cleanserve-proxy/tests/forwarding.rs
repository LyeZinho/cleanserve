//! Regression tests for how the proxy forwards requests to the PHP upstream.
//! The upstream is a tiny fake server that echoes the raw request it received.

use cleanserve_proxy::ProxyServer;
use std::net::TcpListener as StdListener;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn free_port() -> u16 {
    StdListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// Upstream that answers every request with `200` and the raw request text as body.
async fn spawn_echo_upstream() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { return };
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // Read until headers end, then the body announced by Content-Length.
                loop {
                    let n = sock.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    if let Some(idx) = text.find("\r\n\r\n") {
                        let len = text[..idx]
                            .lines()
                            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                            .unwrap_or(0);
                        if buf.len() >= idx + 4 + len {
                            break;
                        }
                    }
                }
                let body = String::from_utf8_lossy(&buf).to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nHost: 127.0.0.1:1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            });
        }
    });
    port
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cleanserve-proxy-{}-{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn start_proxy(root: &PathBuf, php_port: u16) -> u16 {
    let port = free_port();
    let proxy = ProxyServer::new(port, root.to_string_lossy().into_owned()).with_php_port(php_port);
    tokio::spawn(async move {
        let _ = proxy.start().await;
    });
    // Wait for the listener.
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return port;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("proxy did not start");
}

async fn raw_request(port: u16, request: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    s.write_all(request.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), s.read_to_end(&mut out))
        .await
        .expect("response timed out")
        .unwrap();
    String::from_utf8_lossy(&out).to_string()
}

#[tokio::test]
async fn every_http_method_reaches_php_with_original_host() {
    let upstream = spawn_echo_upstream().await;
    let root = temp_root("methods");
    let proxy = start_proxy(&root, upstream).await;

    for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"] {
        let body = if method == "GET" || method == "OPTIONS" { "" } else { "{}" };
        let req = format!(
            "{method} /api/items/1?x=1 HTTP/1.1\r\nHost: myapp.test:8080\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let resp = raw_request(proxy, &req).await;
        let lower = resp.to_ascii_lowercase();

        assert!(resp.starts_with("HTTP/1.1 200"), "{method}: {resp}");
        assert!(resp.contains(&format!("{method} /api/items/1?x=1 HTTP/1.1")), "{method}: {resp}");
        assert!(lower.contains("host: myapp.test:8080"), "{method} lost the original Host: {resp}");
        assert!(lower.contains("x-forwarded-host: myapp.test:8080"), "{method}: {resp}");
        assert!(lower.contains("x-forwarded-proto: http"), "{method}: {resp}");
        assert!(lower.contains("x-forwarded-for: 127.0.0.1"), "{method}: {resp}");
        if !body.is_empty() {
            assert!(resp.ends_with(body), "{method} body not forwarded: {resp}");
        }
        // PHP's built-in server's bogus Host response header must not leak to clients.
        let headers_only = lower.split("\r\n\r\n").next().unwrap();
        assert!(!headers_only.contains("host: 127.0.0.1:1"), "{method}: {resp}");
    }
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn static_html_is_served_directly_with_hmr_without_touching_php() {
    // Nothing listens on this port: if the proxy tried PHP the request would fail.
    let dead_upstream = free_port();
    let root = temp_root("static");
    std::fs::write(root.join("index.html"), "<html><body>hi</body></html>").unwrap();
    let proxy = start_proxy(&root, dead_upstream).await;

    let resp = raw_request(proxy, "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await;
    assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
    assert!(resp.contains("hi"), "{resp}");
    assert!(resp.contains("__cleanserve_hmr"), "HMR client script missing: {resp}");

    let head = raw_request(proxy, "HEAD /index.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    std::fs::remove_dir_all(root).ok();
}

#[tokio::test]
async fn keep_alive_connection_survives_multiple_requests() {
    let upstream = spawn_echo_upstream().await;
    let root = temp_root("keepalive");
    let proxy = start_proxy(&root, upstream).await;

    let mut s = TcpStream::connect(("127.0.0.1", proxy)).await.unwrap();
    for i in 0..3 {
        s.write_all(format!("GET /page{i}.php HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut buf = [0u8; 4096];
        let n = tokio::time::timeout(Duration::from_secs(5), s.read(&mut buf))
            .await
            .expect("timed out")
            .unwrap();
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.starts_with("HTTP/1.1 200"), "request {i}: {resp}");
    }
    std::fs::remove_dir_all(root).ok();
}
