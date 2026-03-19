//! SSL/TLS Configuration for CleanServe
//!
//! SECURITY: Production enforces TLS 1.3 exclusively (no TLS 1.0/1.1/1.2).
//! Development mode generates self-signed certs with 365-day validity.
//! See `crates/cleanserve-proxy/src/tls_config.rs` for TLS protocol enforcement.

use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType};
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub struct SslManager {
    cert_dir: PathBuf,
}

impl SslManager {
    pub fn new() -> anyhow::Result<Self> {
        let cert_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
            .join(".cleanserve")
            .join("certs");

        fs::create_dir_all(&cert_dir)?;
        Ok(Self { cert_dir })
    }

    pub fn get_or_create_cert(&self, domain: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
        let key_path = self.cert_dir.join(format!("{}.key", domain));
        let cert_path = self.cert_dir.join(format!("{}.crt", domain));

        if key_path.exists() && cert_path.exists() {
            return Ok((key_path, cert_path));
        }

        info!("🔐 Generating self-signed certificate for {}", domain);

        let mut cert_params = CertificateParams::default();
        cert_params.is_ca = rcgen::IsCa::NoCa;

        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, domain);
        distinguished_name.push(DnType::OrganizationName, "CleanServe");
        distinguished_name.push(DnType::CountryName, "XX");
        cert_params.distinguished_name = distinguished_name;

        let not_before = time::OffsetDateTime::now_utc();
        let not_after = not_before + time::Duration::days(365);
        cert_params.not_before = not_before;
        cert_params.not_after = not_after;

        let cert = Certificate::from_params(cert_params)?;
        let pem_cert = cert.serialize_pem()?;
        let pem_key = cert.serialize_private_key_pem();

        fs::write(&cert_path, pem_cert)?;
        fs::write(&key_path, pem_key)?;

        info!("✅ Certificate generated: {}", cert_path.display());
        Ok((key_path, cert_path))
    }
}
