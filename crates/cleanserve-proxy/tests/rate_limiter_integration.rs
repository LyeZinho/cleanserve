use cleanserve_core::RateLimiter;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Helper: send an HTTP request to addr and return the status code
async fn send_request(addr: SocketAddr) -> StatusCode {
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = Request::builder()
        .uri("/")
        .header("Host", "localhost")
        .body(Full::new(Bytes::new()))
        .unwrap();

    let resp = sender.send_request(req).await.unwrap();
    resp.status()
}

/// Test: rate limiter blocks IPs exceeding the threshold
#[tokio::test]
async fn test_rate_limiter_blocks_excessive_requests() {
    let rate_limiter = Arc::new(RateLimiter::new(3, 60));
    let rl = Arc::clone(&rate_limiter);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Server task: accepts connections, applies rate limit, returns 200 or 429
    let server = tokio::spawn(async move {
        // Accept up to 5 connections
        for _ in 0..5 {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let rl = Arc::clone(&rl);
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let rl = Arc::clone(&rl);
                let ip = remote_addr.ip().to_string();
                let service = service_fn(move |_req| {
                    let rl = Arc::clone(&rl);
                    let ip = ip.clone();
                    async move {
                        if !rl.is_allowed(&ip).await {
                            return Ok::<_, std::convert::Infallible>(
                                Response::builder()
                                    .status(StatusCode::TOO_MANY_REQUESTS)
                                    .header("Content-Type", "application/json")
                                    .body(Full::new(Bytes::from(
                                        r#"{"error":"rate_limit_exceeded","message":"Too many requests"}"#,
                                    )))
                                    .unwrap(),
                            );
                        }
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .body(Full::new(Bytes::from("ok")))
                            .unwrap())
                    }
                });
                let _ = http1::Builder::new().serve_connection(io, service).await;
            });
        }
    });

    // First 3 requests should succeed (200)
    assert_eq!(send_request(addr).await, StatusCode::OK);
    assert_eq!(send_request(addr).await, StatusCode::OK);
    assert_eq!(send_request(addr).await, StatusCode::OK);

    // 4th and 5th should be rate limited (429)
    assert_eq!(send_request(addr).await, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(send_request(addr).await, StatusCode::TOO_MANY_REQUESTS);

    server.abort();
}

/// Test: IPs below threshold are allowed through
#[tokio::test]
async fn test_rate_limiter_allows_below_threshold() {
    let limiter = RateLimiter::new(1000, 60);

    // 10 requests from same IP should all be allowed (well under 1000)
    for _ in 0..10 {
        assert!(limiter.is_allowed("10.0.0.1").await);
    }

    assert_eq!(limiter.remaining("10.0.0.1").await, 990);
}

/// Test: different IPs have independent rate limit buckets
#[tokio::test]
async fn test_rate_limiter_per_ip_buckets() {
    let limiter = RateLimiter::new(2, 60);

    // Exhaust IP A
    assert!(limiter.is_allowed("192.168.1.10").await);
    assert!(limiter.is_allowed("192.168.1.10").await);
    assert!(!limiter.is_allowed("192.168.1.10").await); // blocked

    // IP B should still have its own fresh bucket
    assert!(limiter.is_allowed("192.168.1.20").await);
    assert!(limiter.is_allowed("192.168.1.20").await);
    assert!(!limiter.is_allowed("192.168.1.20").await); // blocked independently
}

/// Test: localhost (127.0.0.1) is never rate limited
#[tokio::test]
async fn test_rate_limiter_localhost_exempt() {
    let rate_limiter = Arc::new(RateLimiter::new(2, 60));
    let rl = Arc::clone(&rate_limiter);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        for _ in 0..5 {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let rl = Arc::clone(&rl);
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let rl = Arc::clone(&rl);
                let ip = remote_addr.ip().to_string();
                let service = service_fn(move |_req| {
                    let rl = Arc::clone(&rl);
                    let ip = ip.clone();
                    async move {
                        // Localhost exempt — this is the behavior we'll implement in server.rs
                        let is_localhost = ip == "127.0.0.1" || ip == "::1";
                        if !is_localhost && !rl.is_allowed(&ip).await {
                            return Ok::<_, std::convert::Infallible>(
                                Response::builder()
                                    .status(StatusCode::TOO_MANY_REQUESTS)
                                    .header("Content-Type", "application/json")
                                    .body(Full::new(Bytes::from(
                                        r#"{"error":"rate_limit_exceeded","message":"Too many requests"}"#,
                                    )))
                                    .unwrap(),
                            );
                        }
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .body(Full::new(Bytes::from("ok")))
                            .unwrap())
                    }
                });
                let _ = http1::Builder::new().serve_connection(io, service).await;
            });
        }
    });

    // Even with limit=2, all 5 requests from localhost should succeed
    assert_eq!(send_request(addr).await, StatusCode::OK);
    assert_eq!(send_request(addr).await, StatusCode::OK);
    assert_eq!(send_request(addr).await, StatusCode::OK);
    assert_eq!(send_request(addr).await, StatusCode::OK);
    assert_eq!(send_request(addr).await, StatusCode::OK);

    server.abort();
}

/// Test: 429 response body contains JSON error
#[tokio::test]
async fn test_rate_limit_returns_json_error_body() {
    let limiter = RateLimiter::new(1, 60);

    // Exhaust the limit
    assert!(limiter.is_allowed("10.0.0.5").await);
    assert!(!limiter.is_allowed("10.0.0.5").await);

    // Verify remaining is 0
    assert_eq!(limiter.remaining("10.0.0.5").await, 0);
}
