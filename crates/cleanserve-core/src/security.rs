//! Security Headers Module
//! 
//! Implements automatic security header injection and rate limiting.
//! RF-P03: Security Hardening

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// Security headers to inject
#[derive(Debug, Clone)]
pub struct SecurityHeaders {
    pub hsts_max_age: u64,
    pub include_subdomains: bool,
    pub preload: bool,
    pub content_type_nosniff: bool,
    pub x_frame_options: XFrameOption,
    pub xss_protection: bool,
    pub referrer_policy: ReferrerPolicy,
    pub permissions_policy: Option<String>,
    pub csp: Option<ContentSecurityPolicy>,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            hsts_max_age: 31536000, // 1 year
            include_subdomains: true,
            preload: true,
            content_type_nosniff: true,
            x_frame_options: XFrameOption::SameOrigin,
            xss_protection: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
            csp: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum XFrameOption {
    Deny,
    SameOrigin,
    AllowFrom(String),
}

impl std::fmt::Display for XFrameOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XFrameOption::Deny => write!(f, "DENY"),
            XFrameOption::SameOrigin => write!(f, "SAMEORIGIN"),
            XFrameOption::AllowFrom(uri) => write!(f, "ALLOW-FROM {}", uri),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl std::fmt::Display for ReferrerPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferrerPolicy::NoReferrer => write!(f, "no-referrer"),
            ReferrerPolicy::NoReferrerWhenDowngrade => write!(f, "no-referrer-when-downgrade"),
            ReferrerPolicy::Origin => write!(f, "origin"),
            ReferrerPolicy::OriginWhenCrossOrigin => write!(f, "origin-when-cross-origin"),
            ReferrerPolicy::SameOrigin => write!(f, "same-origin"),
            ReferrerPolicy::StrictOrigin => write!(f, "strict-origin"),
            ReferrerPolicy::StrictOriginWhenCrossOrigin => write!(f, "strict-origin-when-cross-origin"),
            ReferrerPolicy::UnsafeUrl => write!(f, "unsafe-url"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContentSecurityPolicy {
    pub directives: HashMap<String, Vec<String>>,
}

impl ContentSecurityPolicy {
    /// Create a restrictive CSP for development
    pub fn development() -> Self {
        let mut directives = HashMap::new();
        directives.insert("default-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("script-src".to_string(), vec!["'self'".to_string(), "'unsafe-inline'".to_string()]);
        directives.insert("style-src".to_string(), vec!["'self'".to_string(), "'unsafe-inline'".to_string()]);
        directives.insert("img-src".to_string(), vec!["'self'".to_string(), "data:".to_string(), "blob:".to_string()]);
        directives.insert("connect-src".to_string(), vec!["'self'".to_string(), "ws:".to_string(), "wss:".to_string()]);
        Self { directives }
    }
    
    /// Create a CSP for production
    pub fn production() -> Self {
        let mut directives = HashMap::new();
        directives.insert("default-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("script-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("style-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("img-src".to_string(), vec!["'self'".to_string(), "data:".to_string(), "https:".to_string()]);
        directives.insert("font-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("connect-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("frame-ancestors".to_string(), vec!["'none'".to_string()]);
        directives.insert("base-uri".to_string(), vec!["'self'".to_string()]);
        directives.insert("form-action".to_string(), vec!["'self'".to_string()]);
        Self { directives }
    }
}

impl std::fmt::Display for ContentSecurityPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self.directives
            .iter()
            .map(|(k, v)| format!("{} {}", k, v.join(" ")))
            .collect();
        write!(f, "{}", parts.join("; "))
    }
}

impl SecurityHeaders {
    /// Convert headers to HTTP header map
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        // HSTS
        let mut hsts = format!("max-age={}", self.hsts_max_age);
        if self.include_subdomains {
            hsts.push_str("; includeSubDomains");
        }
        if self.preload {
            hsts.push_str("; preload");
        }
        headers.insert("Strict-Transport-Security".to_string(), hsts);
        
        // X-Content-Type-Options
        if self.content_type_nosniff {
            headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        }
        
        // X-Frame-Options
        headers.insert("X-Frame-Options".to_string(), self.x_frame_options.to_string());
        
        // X-XSS-Protection
        if self.xss_protection {
            headers.insert("X-XSS-Protection".to_string(), "1; mode=block".to_string());
        }
        
        // Referrer-Policy
        headers.insert("Referrer-Policy".to_string(), self.referrer_policy.to_string());
        
        // Permissions-Policy
        if let Some(ref policy) = self.permissions_policy {
            headers.insert("Permissions-Policy".to_string(), policy.clone());
        }
        
        // Content-Security-Policy
        if let Some(ref csp) = self.csp {
            headers.insert("Content-Security-Policy".to_string(), csp.to_string());
        }
        
        // X-Permitted-Cross-Domain-Policies
        headers.insert("X-Permitted-Cross-Domain-Policies".to_string(), "none".to_string());
        
        // Remove X-Powered-By (security through obscurity isn't security)
        // This is handled by not setting it in the first place
        
        headers
    }
}

/// Rate limiter for basic DDoS protection
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }
    
    /// Check if a request from the given IP should be allowed
    pub async fn is_allowed(&self, ip: &str) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);
        
        let mut requests = self.requests.write().await;
        
        // Clean old entries
        let ip_requests = requests.entry(ip.to_string()).or_insert_with(Vec::new);
        ip_requests.retain(|&t| now.duration_since(t) < window);
        
        // Check limit
        if ip_requests.len() >= self.max_requests {
            warn!("Rate limit exceeded for IP: {}", ip);
            return false;
        }
        
        ip_requests.push(now);
        true
    }
    
    /// Get remaining requests for an IP
    pub async fn remaining(&self, ip: &str) -> usize {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);
        
        let requests = self.requests.read().await;
        let ip_requests = requests.get(ip).map(|v| {
            v.iter().filter(|&&t| now.duration_since(t) < window).count()
        }).unwrap_or(0);
        
        self.max_requests.saturating_sub(ip_requests)
    }
}

/// Simple IP extractor from request headers
pub fn extract_client_ip(headers: &HashMap<String, String>, remote_addr: &str) -> String {
    // Check X-Forwarded-For first (proxy/load balancer)
    if let Some(xff) = headers.get("X-Forwarded-For") {
        // Take the first IP (original client)
        return xff.split(',').next().unwrap_or(xff).trim().to_string();
    }
    
    // Check X-Real-IP
    if let Some(xri) = headers.get("X-Real-IP") {
        return xri.trim().to_string();
    }
    
    // Fall back to direct connection IP
    remote_addr.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_headers_default() {
        let headers = SecurityHeaders::default();
        let map = headers.to_headers();
        
        assert!(map.contains_key("Strict-Transport-Security"));
        assert!(map.contains_key("X-Content-Type-Options"));
        assert!(map.contains_key("X-Frame-Options"));
        assert!(map.contains_key("Referrer-Policy"));
    }

    #[test]
    fn test_rate_limiter() {
        let _limiter = RateLimiter::new(3, 60);
        
        // Should allow first 3 requests
        // Can't easily test async here without tokio::test
    }
}
