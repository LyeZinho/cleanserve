//! URL Security Validation Module
//!
//! Provides protection against SSRF, DNS rebinding, and other URL-based attacks.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

/// Private IP ranges that should never be accessible
const PRIVATE_RANGES: &[(&str, &str)] = &[
    ("10.0.0.0", "10.255.255.255"),
    ("172.16.0.0", "172.31.255.255"),
    ("192.168.0.0", "192.168.255.255"),
    ("127.0.0.0", "127.255.255.255"),
    ("169.254.0.0", "169.254.255.255"),
    ("0.0.0.0", "0.255.255.255"),
    ("100.64.0.0", "100.127.255.255"),
    ("192.0.0.0", "192.0.0.255"),
    ("192.0.2.0", "192.0.2.255"),
    ("198.51.100.0", "198.51.100.255"),
    ("203.0.113.0", "203.0.113.255"),
    ("fc00::", "fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"),
    ("fe80::", "febf:ffff:ffff:ffff:ffff:ffff:ffff:ffff"),
    ("::1", "::1"),
];

/// Allowed domains for downloads
const ALLOWED_DOMAINS: &[&str] = &[
    "github.com",
    "raw.githubusercontent.com",
    "githubusercontent.com",
    "downloads.php.net",
    "windows.php.net",
    "windows.php.net/downloads",
    "dl.google.com",
    "getcomposer.org",
    "packagist.org",
];

/// URL validation result
#[derive(Debug)]
pub enum UrlValidationError {
    InvalidUrl(String),
    PrivateIpBlocked(String),
    UntrustedDomain(String),
    ReservedIpBlocked(String),
}

/// URL validator for security
pub struct UrlValidator {
    cache: std::sync::Mutex<std::collections::HashMap<String, (Instant, Vec<IpAddr>)>>,
    cache_ttl: Duration,
}

impl UrlValidator {
    pub fn new() -> Self {
        Self {
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            cache_ttl: Duration::from_secs(30),
        }
    }

    pub fn validate_url(&self, url: &str) -> Result<(), UrlValidationError> {
        let parsed =
            url::Url::parse(url).map_err(|e| UrlValidationError::InvalidUrl(e.to_string()))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| UrlValidationError::InvalidUrl("No host in URL".to_string()))?;

        if let Some(port) = parsed.port() {
            if port == 0 {
                return Err(UrlValidationError::InvalidUrl("Invalid port 0".to_string()));
            }
        }

        if let Ok(ip) = host.parse::<IpAddr>() {
            self.validate_ip(&ip)?;
        } else {
            let ips = self.resolve_and_validate(host)?;
            for ip in ips {
                self.validate_ip(&ip)?;
            }
        }

        Ok(())
    }

    fn validate_ip(&self, ip: &IpAddr) -> Result<(), UrlValidationError> {
        if ip.is_loopback() {
            return Err(UrlValidationError::PrivateIpBlocked(
                "Loopback addresses blocked".to_string(),
            ));
        }

        if ip.is_private() {
            return Err(UrlValidationError::PrivateIpBlocked(
                "Private IP ranges blocked".to_string(),
            ));
        }

        if ip.is_unspecified() {
            return Err(UrlValidationError::ReservedIpBlocked(
                "Unspecified IP (0.0.0.0) blocked".to_string(),
            ));
        }

        if ip.is_broadcast() {
            return Err(UrlValidationError::ReservedIpBlocked(
                "Broadcast address blocked".to_string(),
            ));
        }

        if let IpAddr::V4(ipv4) = ip {
            if ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254 {
                return Err(UrlValidationError::PrivateIpBlocked(
                    "Link-local blocked".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn resolve_and_validate(&self, hostname: &str) -> Result<Vec<IpAddr>, UrlValidationError> {
        if hostname.contains("localhost") {
            return Err(UrlValidationError::PrivateIpBlocked(
                "localhost blocked".to_string(),
            ));
        }

        let domain_check = hostname.to_lowercase();
        let trusted = ALLOWED_DOMAINS
            .iter()
            .any(|d| domain_check == *d || domain_check.ends_with(&format!(".{}", d)));

        if !trusted {
            return Err(UrlValidationError::UntrustedDomain(format!(
                "Domain '{}' not in allowlist. Allowed: {:?}",
                hostname, ALLOWED_DOMAINS
            )));
        }

        #[cfg(not(test))]
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some((instant, ips)) = cache.get(hostname) {
                if instant.elapsed() < self.cache_ttl {
                    return Ok(ips.clone());
                }
            }

            let ips: Vec<IpAddr> = std::net::ToSocketAddrs::to_socket_addrs(hostname)
                .map_err(|e| {
                    UrlValidationError::InvalidUrl(format!("DNS resolution failed: {}", e))
                })?
                .map(|s| s.ip())
                .collect();

            cache.insert(hostname.to_string(), (Instant::now(), ips.clone()));
            Ok(ips)
        }

        #[cfg(test)]
        Ok(vec![])
    }
}

impl Default for UrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loopback_blocked() {
        let validator = UrlValidator::new();
        assert!(validator
            .validate_url("http://127.0.0.1:8080/test")
            .is_err());
        assert!(validator
            .validate_url("http://localhost:8080/test")
            .is_err());
    }

    #[test]
    fn test_private_ips_blocked() {
        let validator = UrlValidator::new();
        assert!(validator.validate_url("http://10.0.0.1/test").is_err());
        assert!(validator.validate_url("http://192.168.1.1/test").is_err());
        assert!(validator.validate_url("http://172.16.0.1/test").is_err());
    }

    #[test]
    fn test_github_allowed() {
        let validator = UrlValidator::new();
        assert!(validator
            .validate_url(
                "https://github.com/LyeZinho/cleanserve/releases/download/v0.3.0/test.tar.gz"
            )
            .is_ok());
        assert!(validator
            .validate_url("https://raw.githubusercontent.com/test/file.zip")
            .is_ok());
    }
}
