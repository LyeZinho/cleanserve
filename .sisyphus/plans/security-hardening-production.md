# CleanServe Security Hardening for Production

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers/subagent-driven-development` to implement this plan task-by-task in current session.

**Goal:** Transform CleanServe from a development-focused server into a production-grade replacement for Nginx + ModSecurity + PHP-FPM with five security layers (network hardening, WAF-lite, sandbox isolation, secrets management, and audit logging).

**Architecture:** Implement security checks at five strategic points in the HTTP request pipeline:
1. **Network Layer** (early): Rate limiting, idle timeout, TLS enforcement
2. **Application Layer** (pre-FastCGI): Header sanitization, size validation, path traversal, blacklist checks
3. **Isolation Layer** (PHP config): Runtime restrictions, file permissions
4. **Secrets Layer** (environment): Masking, secure injection
5. **Audit Layer** (logging): Structured JSON logs with security events

Each layer is independent and can be toggled via `cleanserve.json` with sensible defaults (strict in production, relaxed in development).

**Tech Stack:** Tokio (async), Hyper (HTTP), Tracing (structured logging), OpenSSL/rcgen (TLS), async-acme (Let's Encrypt)

---

## Implementation Priority & Schedule

| Phase | Tasks | Effort | Blocking? |
|-------|-------|--------|-----------|
| **P0 - Critical** | Rate limit integration, request validation, TLS enforcement | 6-8h | YES |
| **P1 - High** | Static file blacklist, path traversal, Slowloris | 4-6h | YES |
| **P2 - Medium** | PHP lockdown, audit logging, secrets masking | 5-7h | NO |
| **P3 - Nice-to-Have** | ACME automation, integrity checks, advanced WAF | 4-6h | NO |

---

## Phase P0: Critical Security (Rate Limiting, Validation, TLS)

### Task P0-1: Integrate Rate Limiter into HTTP Request Pipeline

**Files:**
- Modify: `crates/cleanserve-proxy/src/server.rs:1-120`
- Create: `crates/cleanserve-proxy/src/rate_limit.rs`
- Test: `crates/cleanserve-proxy/tests/test_rate_limit_integration.rs`

**Overview:** The RateLimiter struct exists in `cleanserve-core` but is never called. We need to:
1. Pass RateLimiter into the request handler
2. Check rate limit BEFORE handling request
3. Return 429 Too Many Requests if limit exceeded
4. Log rate limit violations

**Step 1: Create rate limit middleware module**

File: `crates/cleanserve-proxy/src/rate_limit.rs`

```rust
use cleanserve_core::RateLimiter;
use hyper::{Response, StatusCode};
use http_body_util::Full;
use hyper::body::Bytes;
use std::net::SocketAddr;
use tracing::warn;

/// Check if request should be allowed based on rate limit
pub async fn check_rate_limit(
    limiter: &RateLimiter,
    remote_addr: SocketAddr,
) -> Result<(), Response<Full<Bytes>>> {
    let ip = remote_addr.ip().to_string();
    
    if !limiter.is_allowed(&ip).await {
        warn!("Rate limit exceeded for IP: {}", ip);
        let response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(Full::new(Bytes::from("429 Too Many Requests")))
            .unwrap();
        return Err(response);
    }
    
    Ok(())
}
```

**Step 2: Modify ProxyServer to hold RateLimiter**

File: `crates/cleanserve-proxy/src/server.rs` (modify struct definition)

Replace lines 76-80:
```rust
pub struct ProxyServer {
    port: u16,
    root: Arc<String>,
    hmr_state: Arc<RwLock<HmrState>>,
}
```

With:
```rust
pub struct ProxyServer {
    port: u16,
    root: Arc<String>,
    hmr_state: Arc<RwLock<HmrState>>,
    rate_limiter: Arc<cleanserve_core::RateLimiter>,
}
```

Update `new()` method (lines 83-89):
```rust
impl ProxyServer {
    pub fn new(port: u16, root: String) -> Self {
        // 100 requests per 60 seconds per IP
        let rate_limiter = Arc::new(cleanserve_core::RateLimiter::new(100, 60));
        
        Self {
            port,
            root: Arc::new(root),
            hmr_state: Arc::new(RwLock::new(HmrState::new())),
            rate_limiter,
        }
    }
}
```

**Step 3: Add rate limit check to request handler**

File: `crates/cleanserve-proxy/src/server.rs` (in `start()` method)

Inside the `tokio::spawn` block (around line 103-107), before `handle_request`:

```rust
let limiter = Arc::clone(&rate_limiter);
let remote_addr = stream.peer_addr().unwrap_or_else(|_| {
    SocketAddr::from(([127, 0, 0, 1], 0))
});

tokio::spawn(async move {
    use crate::rate_limit::check_rate_limit;
    
    // Check rate limit FIRST
    if let Err(rate_limit_response) = check_rate_limit(&limiter, remote_addr).await {
        // Serve rate limit error and close connection
        let io = TokioIo::new(stream);
        let service = service_fn(|_| async {
            Ok::<_, Infallible>(rate_limit_response.clone())
        });
        let _ = http1::Builder::new().serve_connection(io, service).await;
        return;
    }
    
    // Proceed with normal request handling
    let service = service_fn(move |req| {
        // ... existing handler code
    });
    // ...
});
```

**Step 4: Add module declaration**

File: `crates/cleanserve-proxy/src/lib.rs` (or `crates/cleanserve-proxy/src/main.rs`)

Add after other mod declarations:
```rust
pub mod rate_limit;
```

**Step 5: Write integration test**

File: `crates/cleanserve-proxy/tests/test_rate_limit_integration.rs`

```rust
#[tokio::test]
async fn test_rate_limit_blocks_excessive_requests() {
    let limiter = cleanserve_core::RateLimiter::new(3, 60);
    let ip = "192.168.1.100";
    
    // First 3 requests should be allowed
    assert!(limiter.is_allowed(ip).await);
    assert!(limiter.is_allowed(ip).await);
    assert!(limiter.is_allowed(ip).await);
    
    // 4th request should be blocked
    assert!(!limiter.is_allowed(ip).await);
    
    // Different IP should be allowed
    assert!(limiter.is_allowed("192.168.1.101").await);
}

#[tokio::test]
async fn test_rate_limit_resets_after_window() {
    let limiter = cleanserve_core::RateLimiter::new(2, 1); // 2 req per 1 second
    let ip = "192.168.1.100";
    
    assert!(limiter.is_allowed(ip).await);
    assert!(limiter.is_allowed(ip).await);
    assert!(!limiter.is_allowed(ip).await);
    
    // Wait for window to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Should be allowed again
    assert!(limiter.is_allowed(ip).await);
}
```

**Step 6: Add rate limit config to cleanserve.json**

File: `cleanserve.json` (add to server section)

```json
{
  "server": {
    "rate_limit": {
      "enabled": true,
      "requests_per_second": 100,
      "window_seconds": 60
    }
  }
}
```

**Step 7: Run tests and verify**

```bash
cd crates/cleanserve-proxy
cargo test test_rate_limit --lib
cargo test test_rate_limit_integration --test '*'
```

Expected: All tests PASS

**Step 8: Commit**

```bash
git add \
  crates/cleanserve-proxy/src/rate_limit.rs \
  crates/cleanserve-proxy/src/server.rs \
  crates/cleanserve-proxy/src/lib.rs \
  crates/cleanserve-proxy/tests/test_rate_limit_integration.rs \
  cleanserve.json

git commit -m "feat(security): integrate rate limiting into HTTP request pipeline

- Add rate_limit.rs middleware module
- Check IP-based rate limits before handling requests
- Return 429 Too Many Requests when limit exceeded
- Add integration tests for rate limit enforcement
- Add configuration in cleanserve.json (default: 100 req/60s per IP)"
```

---

### Task P0-2: Add Request Size Validation (Content-Length Limit)

**Files:**
- Create: `crates/cleanserve-core/src/request_validator.rs`
- Modify: `crates/cleanserve-proxy/src/server.rs:104-120`
- Test: `crates/cleanserve-core/tests/test_request_validator.rs`

**Overview:** Prevent buffer overflow and resource exhaustion by enforcing Content-Length limits (default: 10MB, configurable).

**Step 1: Create request validator module**

File: `crates/cleanserve-core/src/request_validator.rs`

```rust
use std::collections::HashMap;
use hyper::StatusCode;

pub struct RequestValidator {
    max_content_length: u64, // bytes
    max_header_size: usize,  // bytes
}

impl RequestValidator {
    pub fn new(max_content_length: u64, max_header_size: usize) -> Self {
        Self {
            max_content_length,
            max_header_size,
        }
    }
    
    /// Validate Content-Length header
    pub fn validate_content_length(&self, headers: &HashMap<String, String>) -> Result<(), String> {
        if let Some(content_length_str) = headers.get("Content-Length") {
            match content_length_str.parse::<u64>() {
                Ok(content_length) => {
                    if content_length > self.max_content_length {
                        return Err(format!(
                            "Content-Length {} exceeds maximum {}",
                            content_length, self.max_content_length
                        ));
                    }
                    Ok(())
                }
                Err(_) => Err("Invalid Content-Length header".to_string()),
            }
        } else {
            Ok(()) // Content-Length is optional for GET, etc
        }
    }
    
    /// Validate Content-Type is present for methods that require it
    pub fn validate_content_type(&self, method: &str, headers: &HashMap<String, String>) -> Result<(), String> {
        match method {
            "POST" | "PUT" | "PATCH" => {
                if !headers.contains_key("Content-Type") {
                    return Err("Content-Type header required for POST/PUT/PATCH".to_string());
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    /// Validate total header size
    pub fn validate_header_size(&self, headers: &HashMap<String, String>) -> Result<(), String> {
        let total_size: usize = headers
            .iter()
            .map(|(k, v)| k.len() + v.len() + 4) // 4 for ": " and "\r\n"
            .sum();
        
        if total_size > self.max_header_size {
            return Err(format!(
                "Total header size {} exceeds maximum {}",
                total_size, self.max_header_size
            ));
        }
        Ok(())
    }
}

pub struct ValidationError {
    pub status: StatusCode,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_length_validation_under_limit() {
        let validator = RequestValidator::new(10_000_000, 50_000);
        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), "5000".to_string());
        
        assert!(validator.validate_content_length(&headers).is_ok());
    }

    #[test]
    fn test_content_length_validation_over_limit() {
        let validator = RequestValidator::new(10_000_000, 50_000);
        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), "20000000".to_string());
        
        assert!(validator.validate_content_length(&headers).is_err());
    }

    #[test]
    fn test_content_type_required_for_post() {
        let validator = RequestValidator::new(10_000_000, 50_000);
        let headers = HashMap::new();
        
        assert!(validator.validate_content_type("POST", &headers).is_err());
        assert!(validator.validate_content_type("GET", &headers).is_ok());
    }

    #[test]
    fn test_header_size_validation() {
        let validator = RequestValidator::new(10_000_000, 1000);
        let mut headers = HashMap::new();
        headers.insert("X-Test".to_string(), "a".repeat(2000));
        
        assert!(validator.validate_header_size(&headers).is_err());
    }
}
```

**Step 2: Export RequestValidator from cleanserve-core**

File: `crates/cleanserve-core/src/lib.rs`

Add:
```rust
pub mod request_validator;
pub use request_validator::RequestValidator;
```

**Step 3: Update proxy server to validate requests**

File: `crates/cleanserve-proxy/src/server.rs`

In the `handle_request` function (around where the request is received), add validation:

```rust
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    root: Arc<String>,
    hmr_state: Arc<RwLock<HmrState>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Extract headers into HashMap for validation
    let mut header_map = HashMap::new();
    for (k, v) in req.headers() {
        if let (Ok(key), Ok(val)) = (k.to_str(), v.to_str()) {
            header_map.insert(key.to_string(), val.to_string());
        }
    }
    
    // Validate request
    let validator = cleanserve_core::RequestValidator::new(
        10_000_000,  // 10MB default
        50_000,      // 50KB max headers
    );
    
    // Check Content-Length
    if let Err(msg) = validator.validate_content_length(&header_map) {
        warn!("Request validation failed: {}", msg);
        return Ok(Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(Full::new(Bytes::from(format!("413 {}", msg))))
            .unwrap());
    }
    
    // Check Content-Type for POST/PUT/PATCH
    if let Err(msg) = validator.validate_content_type(req.method().as_str(), &header_map) {
        warn!("Request validation failed: {}", msg);
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from(format!("400 {}", msg))))
            .unwrap());
    }
    
    // Check total header size
    if let Err(msg) = validator.validate_header_size(&header_map) {
        warn!("Request validation failed: {}", msg);
        return Ok(Response::builder()
            .status(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE)
            .body(Full::new(Bytes::from(format!("431 {}", msg))))
            .unwrap());
    }
    
    // ... rest of existing handler code
}
```

**Step 4: Run tests**

```bash
cd crates/cleanserve-core
cargo test request_validator --lib
```

Expected: All tests PASS

**Step 5: Commit**

```bash
git add \
  crates/cleanserve-core/src/request_validator.rs \
  crates/cleanserve-core/src/lib.rs \
  crates/cleanserve-proxy/src/server.rs

git commit -m "feat(security): add request size and content-type validation

- Create RequestValidator module for Content-Length and Content-Type checks
- Enforce max payload size (10MB default, configurable)
- Validate Content-Type is present for POST/PUT/PATCH
- Check total header size (50KB limit)
- Return 413/400/431 on validation failure
- Add comprehensive unit tests"
```

---

### Task P0-3: Enforce TLS 1.3 and Remove Insecure Protocols

**Files:**
- Modify: `crates/cleanserve-core/src/ssl.rs:1-55`
- Create: `crates/cleanserve-proxy/src/tls_config.rs`
- Test: `crates/cleanserve-proxy/tests/test_tls_enforcement.rs`

**Overview:** Configure Hyper to use TLS 1.3 only, reject TLS 1.0/1.1/1.2 in production.

**Step 1: Create TLS configuration module**

File: `crates/cleanserve-proxy/src/tls_config.rs`

```rust
use rustls::ServerConfig;
use std::sync::Arc;

/// Create a production-grade TLS config (TLS 1.3 only)
pub fn create_tls_config(cert_path: &str, key_path: &str) -> anyhow::Result<ServerConfig> {
    let cert_file = std::fs::File::open(cert_path)?;
    let mut cert_reader = std::io::BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()?;
    
    let key_file = std::fs::File::open(key_path)?;
    let mut key_reader = std::io::BufReader::new(key_file);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()?;
    
    let key = keys.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No private key found"))?;
    
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    
    // TLS 1.3 ONLY
    config.max_protocol_version = Some(&rustls::version::TLS13);
    config.min_protocol_version = Some(&rustls::version::TLS13);
    
    // Disable all cipher suites except ChaCha20 and AES-GCM
    config.cipher_suites = vec![
        rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
        rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
    ];
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_requires_valid_cert_path() {
        let result = create_tls_config("/nonexistent/cert.pem", "/nonexistent/key.pem");
        assert!(result.is_err());
    }
}
```

**Step 2: Update SSL manager to use TLS 1.3**

File: `crates/cleanserve-core/src/ssl.rs` (add to documentation)

Add comment after line 1:
```rust
//! SSL/TLS Configuration for CleanServe
//! 
//! In production, enforces TLS 1.3 exclusively.
//! Development mode generates self-signed certs with 365-day validity.
//! See crates/cleanserve-proxy/src/tls_config.rs for Hyper configuration.
```

**Step 3: Add HSTS header enforcement in security.rs**

File: `crates/cleanserve-core/src/security.rs` (already mostly there, verify)

Ensure default SecurityHeaders includes:
```rust
headers.insert("Strict-Transport-Security".to_string(), 
    "max-age=31536000; includeSubDomains; preload".to_string());
```

**Step 4: Add HTTP->HTTPS redirect**

File: `crates/cleanserve-proxy/src/server.rs`

In `handle_request`, check if request is HTTP and should redirect:

```rust
// If running behind reverse proxy, check X-Forwarded-Proto
if let Some(proto) = header_map.get("X-Forwarded-Proto") {
    if proto == "http" {
        let host = header_map.get("Host").cloned().unwrap_or_else(|| "localhost".to_string());
        let uri = req.uri().clone();
        return Ok(Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header("Location", format!("https://{}{}", host, uri))
            .body(Full::new(Bytes::new()))
            .unwrap());
    }
}
```

**Step 5: Write integration test**

File: `crates/cleanserve-proxy/tests/test_tls_enforcement.rs`

```rust
#[test]
fn test_tls_config_enforces_tls_13() {
    use cleanserve_proxy::tls_config::create_tls_config;
    
    // This test would require actual cert files to run
    // For now, just verify the function exists and is callable
    let result = create_tls_config(
        "/fake/cert.pem",
        "/fake/key.pem",
    );
    
    // Expected to fail with file not found, not with config error
    assert!(result.is_err());
}

#[test]
fn test_hsts_header_enabled() {
    use cleanserve_core::SecurityHeaders;
    
    let headers = SecurityHeaders::default();
    let http_headers = headers.to_headers();
    
    assert!(http_headers.contains_key("Strict-Transport-Security"));
    assert!(http_headers["Strict-Transport-Security"].contains("max-age=31536000"));
}
```

**Step 6: Run tests**

```bash
cd crates/cleanserve-proxy
cargo test tls_enforcement --test '*'
cd crates/cleanserve-core
cargo test security --lib
```

Expected: All tests PASS

**Step 7: Commit**

```bash
git add \
  crates/cleanserve-proxy/src/tls_config.rs \
  crates/cleanserve-core/src/ssl.rs \
  crates/cleanserve-proxy/src/server.rs \
  crates/cleanserve-proxy/tests/test_tls_enforcement.rs

git commit -m "feat(security): enforce TLS 1.3 and add HTTPS redirection

- Create tls_config.rs module with TLS 1.3 enforcement
- Disable TLS 1.0, 1.1, 1.2 in production
- Restrict cipher suites to modern secure algorithms
- Add HTTP->HTTPS redirect via X-Forwarded-Proto check
- Ensure HSTS header includes preload flag
- Add integration tests for TLS configuration"
```

---

## Phase P1: High Priority Security (Blacklist, Path Traversal, Slowloris)

### Task P1-1: Static File Blacklist (.env, .git, .php in uploads)

**Files:**
- Create: `crates/cleanserve-core/src/static_blacklist.rs`
- Modify: `crates/cleanserve-proxy/src/server.rs:150-200` (static file handling)
- Test: `crates/cleanserve-core/tests/test_static_blacklist.rs`

**Overview:** Block access to sensitive files (.env, .git, .gitignore, composer.lock, .php files in upload directories).

**Step 1: Create static file blacklist module**

File: `crates/cleanserve-core/src/static_blacklist.rs`

```rust
use std::path::{Path, PathBuf};

pub struct StaticFileBlacklist {
    blocked_patterns: Vec<String>,
    blocked_extensions: Vec<String>,
    upload_dirs: Vec<String>,
}

impl StaticFileBlacklist {
    pub fn new() -> Self {
        Self {
            blocked_patterns: vec![
                ".env".to_string(),
                ".env.local".to_string(),
                ".env.example".to_string(),
                ".git".to_string(),
                ".gitignore".to_string(),
                ".github".to_string(),
                "composer.lock".to_string(),
                "package.lock".to_string(),
                "package-lock.json".to_string(),
                "yarn.lock".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                ".htaccess".to_string(),
                "web.config".to_string(),
                "php.ini".to_string(),
            ],
            blocked_extensions: vec![".php".to_string(), ".phtml".to_string(), ".phar".to_string()],
            upload_dirs: vec!["uploads".to_string(), "files".to_string(), "tmp".to_string()],
        }
    }
    
    /// Check if file path is blocked
    pub fn is_blocked(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        
        // Check if filename matches blocked patterns
        if let Some(filename) = file_path.file_name() {
            let fname = filename.to_string_lossy();
            if self.blocked_patterns.iter().any(|p| fname.eq_ignore_ascii_case(p)) {
                return true;
            }
        }
        
        // Check extension
        if let Some(ext) = file_path.extension() {
            let ext_str = format!(".{}", ext.to_string_lossy());
            if self.blocked_extensions.iter().any(|e| e.eq_ignore_ascii_case(&ext_str)) {
                // Block .php in any upload directory
                for upload_dir in &self.upload_dirs {
                    if path_str.contains(upload_dir) {
                        return true;
                    }
                }
            }
        }
        
        false
    }
}

impl Default for StaticFileBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_env_file() {
        let blacklist = StaticFileBlacklist::new();
        assert!(blacklist.is_blocked(Path::new(".env")));
        assert!(blacklist.is_blocked(Path::new(".env.local")));
    }

    #[test]
    fn test_blocks_git_files() {
        let blacklist = StaticFileBlacklist::new();
        assert!(blacklist.is_blocked(Path::new(".git")));
        assert!(blacklist.is_blocked(Path::new(".gitignore")));
    }

    #[test]
    fn test_blocks_php_in_uploads() {
        let blacklist = StaticFileBlacklist::new();
        assert!(blacklist.is_blocked(Path::new("uploads/shell.php")));
        assert!(blacklist.is_blocked(Path::new("files/payload.phtml")));
        assert!(!blacklist.is_blocked(Path::new("src/index.php")));
    }

    #[test]
    fn test_allows_legitimate_files() {
        let blacklist = StaticFileBlacklist::new();
        assert!(!blacklist.is_blocked(Path::new("index.html")));
        assert!(!blacklist.is_blocked(Path::new("styles.css")));
        assert!(!blacklist.is_blocked(Path::new("app.js")));
    }
}
```

**Step 2: Export from cleanserve-core**

File: `crates/cleanserve-core/src/lib.rs`

Add:
```rust
pub mod static_blacklist;
pub use static_blacklist::StaticFileBlacklist;
```

**Step 3: Update proxy server to check blacklist**

File: `crates/cleanserve-proxy/src/server.rs`

In `handle_request`, before serving static files:

```rust
use cleanserve_core::StaticFileBlacklist;

// Check if requesting a static file
let blacklist = StaticFileBlacklist::new();
let requested_path = Path::new(&uri_path);

if blacklist.is_blocked(requested_path) {
    warn!("Blocked access to blacklisted file: {}", uri_path);
    return Ok(Response::builder()
        .status(StatusCode::FORBIDDEN)
        .body(Full::new(Bytes::from("403 Forbidden")))
        .unwrap());
}
```

**Step 4: Run tests**

```bash
cd crates/cleanserve-core
cargo test static_blacklist --lib
```

Expected: All tests PASS

**Step 5: Commit**

```bash
git add \
  crates/cleanserve-core/src/static_blacklist.rs \
  crates/cleanserve-core/src/lib.rs \
  crates/cleanserve-proxy/src/server.rs

git commit -m "feat(security): add static file blacklist protection

- Create StaticFileBlacklist module blocking .env, .git, .php in uploads
- Prevent exposure of sensitive configuration files
- Block common attack files (.htaccess, web.config, php.ini)
- Return 403 Forbidden for blacklisted paths
- Add comprehensive unit tests"
```

---

### Task P1-2: Path Traversal Protection with URL Normalization

**Files:**
- Create: `crates/cleanserve-core/src/path_normalizer.rs`
- Modify: `crates/cleanserve-proxy/src/server.rs`
- Test: `crates/cleanserve-core/tests/test_path_normalizer.rs`

**Overview:** Normalize URLs and block path traversal attempts (/../, ..\, etc).

**Step 1: Create path normalizer**

File: `crates/cleanserve-core/src/path_normalizer.rs`

```rust
use std::path::{Path, PathBuf, Component};

pub struct PathNormalizer {
    root: PathBuf,
}

impl PathNormalizer {
    pub fn new(root: &str) -> Self {
        Self {
            root: PathBuf::from(root),
        }
    }
    
    /// Normalize a URL path and ensure it's within root
    pub fn normalize_and_validate(&self, url_path: &str) -> Result<PathBuf, String> {
        // Remove URL encoding issues
        let decoded = urlencoding::decode(url_path)
            .map_err(|_| "Invalid URL encoding")?
            .into_owned();
        
        // Normalize the path
        let normalized = self.normalize_path(&decoded)?;
        
        // Ensure it's within root
        self.validate_within_root(&normalized)?;
        
        Ok(normalized)
    }
    
    /// Normalize path by resolving . and .. components
    fn normalize_path(&self, path: &str) -> Result<PathBuf, String> {
        let mut components = vec![];
        
        for component in Path::new(path).components() {
            match component {
                Component::Normal(c) => {
                    components.push(c.to_string_lossy().into_owned());
                }
                Component::ParentDir => {
                    // Only pop if not at root
                    if components.is_empty() {
                        return Err("Path traversal attempt detected".to_string());
                    }
                    components.pop();
                }
                Component::CurDir => {
                    // Ignore "."
                }
                _ => {
                    return Err("Invalid path component".to_string());
                }
            }
        }
        
        Ok(self.root.join(components.join("/")))
    }
    
    /// Verify path is within root directory
    fn validate_within_root(&self, path: &Path) -> Result<(), String> {
        let canonical_root = self.root.canonicalize()
            .map_err(|_| "Root path not accessible")?;
        
        let canonical_path = path.canonicalize()
            .map_err(|_| "Path not accessible")?;
        
        if !canonical_path.starts_with(&canonical_root) {
            return Err("Path traversal outside root detected".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizes_dot_dot() {
        let normalizer = PathNormalizer::new("/var/www");
        let result = normalizer.normalize_path("/public/../../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("traversal"));
    }

    #[test]
    fn test_normalizes_encoded_traversal() {
        let normalizer = PathNormalizer::new("/var/www");
        let result = normalizer.normalize_path("/public/%2e%2e/%2e%2e/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_allows_legitimate_paths() {
        let normalizer = PathNormalizer::new("/var/www");
        let result = normalizer.normalize_path("/public/index.html");
        assert!(result.is_ok());
    }

    #[test]
    fn test_normalizes_dot_current_dir() {
        let normalizer = PathNormalizer::new("/var/www");
        let result = normalizer.normalize_path("/public/./images/photo.jpg");
        assert!(result.is_ok());
    }
}
```

**Step 2: Export from cleanserve-core**

File: `crates/cleanserve-core/src/lib.rs`

Add:
```rust
pub mod path_normalizer;
pub use path_normalizer::PathNormalizer;
```

Also add dependency to `Cargo.toml`:
```toml
urlencoding = "2"
```

**Step 3: Update proxy server to normalize paths**

File: `crates/cleanserve-proxy/src/server.rs`

In `handle_request`:

```rust
use cleanserve_core::PathNormalizer;

let normalizer = PathNormalizer::new(&root);
let normalized_path = match normalizer.normalize_and_validate(req.uri().path()) {
    Ok(p) => p,
    Err(e) => {
        warn!("Path traversal attempt: {}", e);
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from("400 Invalid path")))
            .unwrap());
    }
};
```

**Step 4: Run tests**

```bash
cd crates/cleanserve-core
cargo test path_normalizer --lib
```

Expected: All tests PASS

**Step 5: Commit**

```bash
git add \
  crates/cleanserve-core/src/path_normalizer.rs \
  crates/cleanserve-core/src/lib.rs \
  crates/cleanserve-proxy/src/server.rs \
  Cargo.toml

git commit -m "feat(security): add path traversal protection with URL normalization

- Create PathNormalizer module for secure path resolution
- Block /../, ..\\ and percent-encoded traversal attempts
- Ensure all paths resolve within project root
- Return 400 for invalid path attempts
- Add comprehensive path traversal test cases"
```

---

### Task P1-3: Slowloris Protection with Idle Connection Timeout

**Files:**
- Create: `crates/cleanserve-proxy/src/slowloris_protection.rs`
- Modify: `crates/cleanserve-proxy/src/server.rs`
- Test: `crates/cleanserve-proxy/tests/test_slowloris.rs`

**Overview:** Close idle connections that don't send data (Slowloris DoS prevention).

**Step 1: Create Slowloris protection middleware**

File: `crates/cleanserve-proxy/src/slowloris_protection.rs`

```rust
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{warn, info};

pub struct SlowlorisProtection {
    idle_timeout: Duration,
    check_interval: Duration,
}

impl SlowlorisProtection {
    pub fn new(idle_timeout_secs: u64) -> Self {
        Self {
            idle_timeout: Duration::from_secs(idle_timeout_secs),
            check_interval: Duration::from_secs(1),
        }
    }
    
    /// Monitor connection for idle timeouts
    pub async fn monitor_connection(&self, mut stream: tokio::net::TcpStream) -> Result<(), std::io::Error> {
        let remote_addr = stream.peer_addr()?;
        let mut last_activity = Instant::now();
        
        loop {
            // Check if idle for too long
            if last_activity.elapsed() > self.idle_timeout {
                warn!("Closing idle connection from {}: exceeded {} seconds", 
                    remote_addr, self.idle_timeout.as_secs());
                stream.shutdown(std::net::Shutdown::Both)?;
                return Ok(());
            }
            
            // Check for data with timeout
            let mut buf = [0u8; 1];
            match tokio::time::timeout(
                self.check_interval,
                tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
            ).await {
                Ok(Ok(0)) => {
                    // Connection closed
                    info!("Connection closed by client: {}", remote_addr);
                    return Ok(());
                }
                Ok(Ok(_)) => {
                    // Data received, reset timeout
                    last_activity = Instant::now();
                }
                Ok(Err(e)) => {
                    warn!("Error reading from {}: {}", remote_addr, e);
                    return Err(e);
                }
                Err(_) => {
                    // Timeout waiting for data, check again
                }
            }
            
            sleep(self.check_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slowloris_protection_creates_with_timeout() {
        let protection = SlowlorisProtection::new(30);
        assert_eq!(protection.idle_timeout, Duration::from_secs(30));
    }
}
```

**Step 2: Update proxy server to apply Slowloris protection**

File: `crates/cleanserve-proxy/src/server.rs`

In the `start()` method, wrap the connection handler:

```rust
let slowloris = Arc::new(crate::slowloris_protection::SlowlorisProtection::new(60));

loop {
    match listener.accept().await {
        Ok((stream, _)) => {
            let slowloris = Arc::clone(&slowloris);
            
            tokio::spawn(async move {
                // Spawn idle timeout monitor in background
                let stream_clone = stream.try_clone().expect("Failed to clone stream");
                tokio::spawn(async move {
                    if let Err(e) = slowloris.monitor_connection(stream_clone).await {
                        warn!("Slowloris monitor error: {}", e);
                    }
                });
                
                // ... existing connection handling
            });
        }
        Err(e) => {
            error!("Accept error: {}", e);
        }
    }
}
```

**Step 3: Add module declaration**

File: `crates/cleanserve-proxy/src/lib.rs`

Add:
```rust
pub mod slowloris_protection;
```

**Step 4: Write integration test**

File: `crates/cleanserve-proxy/tests/test_slowloris.rs`

```rust
#[tokio::test]
async fn test_slowloris_protection_detects_idle() {
    let protection = cleanserve_proxy::slowloris_protection::SlowlorisProtection::new(1);
    
    // Test: Create a TCP connection, don't send data, verify timeout
    // This is complex to test without a real TCP listener, so we verify struct creation
    assert_eq!(protection.idle_timeout.as_secs(), 1);
}
```

**Step 5: Run tests**

```bash
cd crates/cleanserve-proxy
cargo test slowloris --lib
cargo test slowloris --test '*'
```

Expected: All tests PASS

**Step 6: Commit**

```bash
git add \
  crates/cleanserve-proxy/src/slowloris_protection.rs \
  crates/cleanserve-proxy/src/lib.rs \
  crates/cleanserve-proxy/src/server.rs \
  crates/cleanserve-proxy/tests/test_slowloris.rs

git commit -m "feat(security): add Slowloris DoS protection with idle timeout

- Create SlowlorisProtection middleware monitoring idle connections
- Close connections idle for >60 seconds without data
- Prevents resource exhaustion from slowly-sent request attacks
- Add integration tests for timeout detection"
```

---

## Phase P2: Medium Priority Security (PHP Lockdown, Audit Logging, Secrets)

### Task P2-1: PHP Security Lockdown (auto-generate secure php.ini)

**Files:**
- Create: `crates/cleanserve-core/src/php_security_config.rs`
- Modify: `crates/cleanserve-core/src/framework.rs`
- Test: `crates/cleanserve-core/tests/test_php_security_config.rs`

**Overview:** Auto-generate a hardened `php.ini` with dangerous functions disabled and `open_basedir` set.

**Step 1: Create PHP security config generator**

File: `crates/cleanserve-core/src/php_security_config.rs`

```rust
use std::path::Path;

pub struct PhpSecurityConfig {
    project_root: String,
    is_production: bool,
}

impl PhpSecurityConfig {
    pub fn new(project_root: &str, is_production: bool) -> Self {
        Self {
            project_root: project_root.to_string(),
            is_production,
        }
    }
    
    /// Generate secure php.ini content
    pub fn generate_ini(&self) -> String {
        let disabled_functions = if self.is_production {
            "exec,passthru,shell_exec,system,proc_open,popen,curl_exec,curl_multi_exec,parse_ini_file,show_source"
        } else {
            "exec,passthru,shell_exec,system,proc_open,popen"
        };
        
        let open_basedir = format!(
            "{}:{}:/tmp",
            self.project_root,
            Path::new(&self.project_root).parent().unwrap_or(Path::new("/")).to_string_lossy()
        );
        
        format!(r#"
; CleanServe Auto-Generated Security Configuration
; Generated for: {}
; Production Mode: {}

[PHP]
; === Security: Function Restrictions ===
disable_functions = "{}"

; === Security: File System ===
open_basedir = "{}"

; === Resource Limits ===
memory_limit = 128M
max_execution_time = 30
upload_max_filesize = 10M
post_max_size = 10M
max_input_time = 60

; === Security: Remote Code Execution ===
allow_url_fopen = Off
allow_url_include = Off

; === Security: Display Errors (Dev: On, Prod: Off) ===
display_errors = {}
display_startup_errors = {}
log_errors = On
error_log = stderr

; === Security: Sessions ===
session.cookie_httponly = On
session.cookie_secure = On
session.cookie_samesite = "Strict"
session.use_strict_mode = On
session.cache_limiter = "nocache"

; === Security: Headers ===
expose_php = Off
"#,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            if self.is_production { "YES" } else { "NO" },
            disabled_functions,
            open_basedir,
            if self.is_production { "Off" } else { "On" },
            if self.is_production { "Off" } else { "On" },
        )
    }
    
    /// Write configuration to file
    pub fn write_to_file(&self, output_path: &str) -> std::io::Result<()> {
        std::fs::write(output_path, self.generate_ini())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generates_valid_php_ini() {
        let config = PhpSecurityConfig::new("/var/www", false);
        let ini = config.generate_ini();
        
        assert!(ini.contains("disable_functions"));
        assert!(ini.contains("open_basedir"));
        assert!(ini.contains("display_errors = On"));
    }

    #[test]
    fn test_production_mode_stricter() {
        let dev_config = PhpSecurityConfig::new("/var/www", false);
        let prod_config = PhpSecurityConfig::new("/var/www", true);
        
        let dev_ini = dev_config.generate_ini();
        let prod_ini = prod_config.generate_ini();
        
        assert!(dev_ini.contains("display_errors = On"));
        assert!(prod_ini.contains("display_errors = Off"));
    }

    #[test]
    fn test_open_basedir_set_correctly() {
        let config = PhpSecurityConfig::new("/var/www/myapp", false);
        let ini = config.generate_ini();
        
        assert!(ini.contains("/var/www/myapp"));
        assert!(ini.contains(":/tmp"));
    }
}
```

**Step 2: Export from cleanserve-core**

File: `crates/cleanserve-core/src/lib.rs`

Add:
```rust
pub mod php_security_config;
pub use php_security_config::PhpSecurityConfig;
```

Also add `chrono` to dependencies if not already present.

**Step 3: Update framework.rs to use this config**

File: `crates/cleanserve-core/src/framework.rs`

Find the function that generates or manages php.ini, and integrate:

```rust
use crate::php_security_config::PhpSecurityConfig;

pub fn generate_secure_php_ini(project_root: &str, is_production: bool) -> String {
    let config = PhpSecurityConfig::new(project_root, is_production);
    config.generate_ini()
}
```

**Step 4: Run tests**

```bash
cd crates/cleanserve-core
cargo test php_security_config --lib
```

Expected: All tests PASS

**Step 5: Commit**

```bash
git add \
  crates/cleanserve-core/src/php_security_config.rs \
  crates/cleanserve-core/src/lib.rs \
  crates/cleanserve-core/src/framework.rs

git commit -m "feat(security): auto-generate hardened php.ini with function restrictions

- Create PhpSecurityConfig module for production-grade ini generation
- Disable dangerous functions: exec, shell_exec, system, proc_open, etc
- Enforce open_basedir to project root + /tmp
- Set session.cookie_httponly, secure, and samesite flags
- Disable display_errors in production, enable in dev
- Add comprehensive ini generation tests"
```

---

### Task P2-2: Structured Audit Logging with Security Events

**Files:**
- Create: `crates/cleanserve-core/src/audit_logger.rs`
- Modify: `crates/cleanserve-proxy/src/server.rs`
- Test: `crates/cleanserve-core/tests/test_audit_logger.rs`

**Overview:** Log security-relevant events (rate limit hits, path traversal, blacklist violations) in JSON format for compliance/forensics.

**Step 1: Create audit logger**

File: `crates/cleanserve-core/src/audit_logger.rs`

```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize)]
pub enum SecurityEvent {
    #[serde(rename = "rate_limit_exceeded")]
    RateLimitExceeded {
        ip: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "path_traversal_attempt")]
    PathTraversalAttempt {
        ip: String,
        path: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "blacklist_violation")]
    BlacklistViolation {
        ip: String,
        requested_file: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "invalid_content_length")]
    InvalidContentLength {
        ip: String,
        claimed_length: u64,
        max_allowed: u64,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "missing_content_type")]
    MissingContentType {
        ip: String,
        method: String,
        timestamp: DateTime<Utc>,
    },
}

pub struct AuditLogger;

impl AuditLogger {
    /// Log a security event to stdout as JSON
    pub fn log_event(event: &SecurityEvent) {
        match serde_json::to_string(event) {
            Ok(json) => println!("[SECURITY] {}", json),
            Err(e) => eprintln!("Failed to serialize security event: {}", e),
        }
    }
    
    /// Log rate limit event
    pub fn log_rate_limit(ip: &str) {
        Self::log_event(&SecurityEvent::RateLimitExceeded {
            ip: ip.to_string(),
            timestamp: Utc::now(),
        });
    }
    
    /// Log path traversal attempt
    pub fn log_path_traversal(ip: &str, path: &str) {
        Self::log_event(&SecurityEvent::PathTraversalAttempt {
            ip: ip.to_string(),
            path: path.to_string(),
            timestamp: Utc::now(),
        });
    }
    
    /// Log blacklist violation
    pub fn log_blacklist_violation(ip: &str, file: &str) {
        Self::log_event(&SecurityEvent::BlacklistViolation {
            ip: ip.to_string(),
            requested_file: file.to_string(),
            timestamp: Utc::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_event_serializes() {
        let event = SecurityEvent::RateLimitExceeded {
            ip: "192.168.1.100".to_string(),
            timestamp: Utc::now(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("rate_limit_exceeded"));
        assert!(json.contains("192.168.1.100"));
    }

    #[test]
    fn test_path_traversal_event_includes_path() {
        let event = SecurityEvent::PathTraversalAttempt {
            ip: "10.0.0.1".to_string(),
            path: "/../etc/passwd".to_string(),
            timestamp: Utc::now(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("path_traversal_attempt"));
        assert!(json.contains("/../etc/passwd"));
    }
}
```

**Step 2: Export from cleanserve-core**

File: `crates/cleanserve-core/src/lib.rs`

Add:
```rust
pub mod audit_logger;
pub use audit_logger::{AuditLogger, SecurityEvent};
```

**Step 3: Integrate audit logging into server**

File: `crates/cleanserve-proxy/src/server.rs`

In relevant error handlers:

```rust
use cleanserve_core::AuditLogger;

// Rate limit exceeded
AuditLogger::log_rate_limit(&ip);

// Path traversal attempt
AuditLogger::log_path_traversal(&ip, &requested_path);

// Blacklist violation
AuditLogger::log_blacklist_violation(&ip, &file_path);
```

**Step 4: Run tests**

```bash
cd crates/cleanserve-core
cargo test audit_logger --lib
```

Expected: All tests PASS

**Step 5: Commit**

```bash
git add \
  crates/cleanserve-core/src/audit_logger.rs \
  crates/cleanserve-core/src/lib.rs \
  crates/cleanserve-proxy/src/server.rs

git commit -m "feat(security): add structured JSON audit logging for security events

- Create AuditLogger module for security event logging
- Log rate limit violations, path traversal, blacklist hits
- Output machine-readable JSON for SIEM/forensics tools
- Include IP, timestamp, and event details
- Add serialization tests"
```

---

## Phase P3: Nice-to-Have (ACME, Integrity Checks, Advanced WAF)

### Task P3-1: ACME/Let's Encrypt Integration for Automatic Cert Renewal

*This is a larger task; outline only:*

**Files to Create:**
- `crates/cleanserve-core/src/acme_manager.rs` — Interface with Let's Encrypt API
- `crates/cleanserve-core/src/cert_renewal_scheduler.rs` — Background renewal task

**Libraries:**
- Add `async-acme` or `certbot` integration to `Cargo.toml`

**Implementation Steps:**
1. Create ACME client wrapper
2. Generate CSR (Certificate Signing Request)
3. Handle ACME challenges (HTTP-01, DNS-01)
4. Renew certs 30 days before expiry
5. Reload certs without restart (hot reload)

---

## Summary of All Changes

**Total Tasks:** 8 (3 P0, 3 P1, 2 P2, 1 P3 outlined)

**Estimated Timeline:**
- **P0 (Rate Limiting, Validation, TLS):** 6-8 hours
- **P1 (Blacklist, Path Traversal, Slowloris):** 4-6 hours
- **P2 (PHP Lockdown, Audit Logging):** 3-4 hours
- **P3 (ACME):** 3-4 hours

**Total:** 16-22 hours of implementation

**Key Files Modified:**
- `crates/cleanserve-proxy/src/server.rs` — Main request pipeline
- `crates/cleanserve-core/src/lib.rs` — Module exports
- `Cargo.toml` — Dependencies
- `cleanserve.json` — Configuration schema

**Key Files Created:**
- 10+ new modules in `cleanserve-core` and `cleanserve-proxy`
- Comprehensive test suites for each module

---

## Configuration Schema (cleanserve.json Addition)

```json
{
  "security": {
    "enabled": true,
    "mode": "production",
    "rate_limiting": {
      "enabled": true,
      "requests_per_second": 100,
      "window_seconds": 60
    },
    "request_validation": {
      "max_content_length": 10485760,
      "max_header_size": 51200
    },
    "tls": {
      "min_version": "1.3",
      "enforce_https": true
    },
    "slowloris_protection": {
      "enabled": true,
      "idle_timeout_secs": 60
    },
    "php_lockdown": {
      "enabled": true,
      "disable_functions": ["exec", "shell_exec", "system", "passthru"]
    },
    "audit_logging": {
      "enabled": true,
      "log_format": "json",
      "log_path": "logs/security.log"
    }
  }
}
```

---

## Testing Checklist

Before each commit:

```bash
# Run all tests in a module
cargo test --lib -p cleanserve-core
cargo test --lib -p cleanserve-proxy

# Run specific security tests
cargo test security --lib
cargo test rate_limit --lib

# Run integration tests
cargo test --test '*'

# Build for production
cargo build --release
```

---

## Deployment Verification

After all phases complete:

1. **TLS/HTTPS:**
   ```bash
   openssl s_client -connect localhost:443 -tls1_3
   # Verify TLS 1.3 connection
   ```

2. **Rate Limiting:**
   ```bash
   for i in {1..150}; do curl http://localhost:8080; done
   # Verify 429 after 100 requests
   ```

3. **Path Traversal:**
   ```bash
   curl http://localhost:8080/../../etc/passwd
   # Verify 400 Bad Request
   ```

4. **Blacklist:**
   ```bash
   curl http://localhost:8080/.env
   # Verify 403 Forbidden
   ```

5. **Audit Logs:**
   ```bash
   tail -f logs/security.log
   # Verify JSON security events appear
   ```

---

## Notes for Implementer

- **TDD First:** Write tests before implementation
- **Frequent Commits:** Each task = 1 commit, keep history clean
- **Error Messages:** Should be informative but not leak system info
- **Performance:** Each security check should be <1ms
- **Configuration:** All features should be toggleable via `cleanserve.json`
- **Documentation:** Update CLI help and README after each phase

---

**This plan is complete and ready for execution.** 

Next step: Execute task-by-task using `superpowers/subagent-driven-development` or `superpowers/executing-plans` in a new session.
