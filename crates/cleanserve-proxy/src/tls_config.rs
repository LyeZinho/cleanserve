//! TLS Configuration Module
//!
//! Enforces TLS 1.3 exclusively with secure cipher suites.
//! Rejects TLS 1.0, 1.1, and 1.2 connections.
//!
//! Cipher suites (in priority order):
//! - TLS_CHACHA20_POLY1305_SHA256
//! - TLS_AES_256_GCM_SHA384
//! - TLS_AES_128_GCM_SHA256

use rustls::crypto::ring as ring_provider;
use rustls::crypto::CryptoProvider;
use rustls::ServerConfig;
use std::path::Path;
use std::sync::Arc;

/// TLS 1.3 only cipher suites — ChaCha20 and AES-GCM variants.
fn tls13_secure_provider() -> CryptoProvider {
    let mut provider = ring_provider::default_provider();
    provider.cipher_suites = vec![
        ring_provider::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        ring_provider::cipher_suite::TLS13_AES_256_GCM_SHA384,
        ring_provider::cipher_suite::TLS13_AES_128_GCM_SHA256,
    ];
    provider
}

/// Create production-grade TLS config enforcing TLS 1.3 only.
///
/// - Rejects TLS 1.0, 1.1, 1.2
/// - Uses only ChaCha20-Poly1305 and AES-GCM cipher suites
/// - Validates cert and key files exist before loading
pub fn create_tls_config(
    cert_path: &str,
    key_path: &str,
) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    if !Path::new(cert_path).exists() {
        return Err(format!("Certificate file not found: {}", cert_path).into());
    }
    if !Path::new(key_path).exists() {
        return Err(format!("Private key file not found: {}", key_path).into());
    }

    let cert_file = std::fs::File::open(cert_path)?;
    let mut cert_reader = std::io::BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    if certs.is_empty() {
        return Err("No certificates found in cert file".into());
    }

    let key_file = std::fs::File::open(key_path)?;
    let mut key_reader = std::io::BufReader::new(key_file);
    let keys =
        rustls_pemfile::pkcs8_private_keys(&mut key_reader).collect::<Result<Vec<_>, _>>()?;

    if keys.is_empty() {
        return Err("No private key found in key file".into());
    }

    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        keys.into_iter().next().ok_or("No private key found")?,
    );

    let provider = Arc::new(tls13_secure_provider());

    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_rejects_nonexistent_cert() {
        let result = create_tls_config("/nonexistent/cert.pem", "/nonexistent/key.pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_tls_config_error_message_cert() {
        let result = create_tls_config("/nonexistent/cert.pem", "/nonexistent/key.pem");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "Expected 'not found' in error message, got: {}",
            err
        );
    }

    #[test]
    fn test_tls_config_rejects_nonexistent_key() {
        let tmp_dir = std::env::temp_dir();
        let fake_cert = tmp_dir.join("cleanserve_test_fake.crt");
        std::fs::write(&fake_cert, "fake cert data").unwrap();

        let result = create_tls_config(fake_cert.to_str().unwrap(), "/nonexistent/key.pem");
        assert!(result.is_err());

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "Expected 'not found' in error message, got: {}",
            err
        );

        let _ = std::fs::remove_file(&fake_cert);
    }

    #[test]
    fn test_tls13_secure_provider_cipher_suites() {
        let provider = tls13_secure_provider();
        assert_eq!(
            provider.cipher_suites.len(),
            3,
            "Should have exactly 3 TLS 1.3 cipher suites"
        );

        let names: Vec<_> = provider
            .cipher_suites
            .iter()
            .map(|cs| format!("{:?}", cs.suite()))
            .collect();

        assert!(
            names.iter().any(|n| n.contains("CHACHA20")),
            "Must include ChaCha20: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("AES_256_GCM")),
            "Must include AES-256-GCM: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("AES_128_GCM")),
            "Must include AES-128-GCM: {:?}",
            names
        );
    }
}
