/// TLS certificate and key loading utilities.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;

use crate::config::Config;
use crate::error::{ProxyError, Result};

/// Load TLS certificates from a PEM file.
pub fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path).map_err(|e| {
        ProxyError::CertificateLoad(format!("Failed to open certificate file: {}", e))
    })?;
    let mut reader = BufReader::new(file);

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| ProxyError::CertificateLoad(format!("Failed to parse certificates: {}", e)))?;

    if certs.is_empty() {
        return Err(ProxyError::CertificateLoad(
            "No certificates found in file".to_string(),
        ));
    }

    Ok(certs)
}

/// Load a private key from a PEM file.
pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = File::open(path).map_err(|e| {
        ProxyError::PrivateKeyLoad(format!("Failed to open private key file: {}", e))
    })?;
    let mut reader = BufReader::new(file);

    loop {
        match rustls_pemfile::read_one(&mut reader) {
            Ok(Some(rustls_pemfile::Item::Pkcs1Key(key))) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Ok(Some(rustls_pemfile::Item::Pkcs8Key(key))) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            Ok(Some(rustls_pemfile::Item::Sec1Key(key))) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Ok(Some(_)) => continue, // Skip other items like certificates
            Ok(None) => {
                return Err(ProxyError::PrivateKeyLoad(
                    "No private key found in file".to_string(),
                ))
            }
            Err(e) => {
                return Err(ProxyError::PrivateKeyLoad(format!(
                    "Failed to parse private key: {}",
                    e
                )))
            }
        }
    }
}

/// Build TLS server configuration from certificate and key files.
pub fn build_server_config(config: &Config) -> Result<ServerConfig> {
    let cert_path = config.cert.as_ref().expect("cert required for TLS");
    let key_path = config.key.as_ref().expect("key required for TLS");

    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(ProxyError::Tls)?;

    Ok(tls_config)
}
