use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::proxy::proxy_connection;
use crate::stats::Stats;
use crate::upstream::UpstreamConnection;

/// Load TLS certificates from a PEM file.
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
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
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
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
fn build_tls_config(config: &Config) -> Result<ServerConfig> {
    let cert_path = config.cert.as_ref().expect("cert required for TLS");
    let key_path = config.key.as_ref().expect("key required for TLS");

    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| ProxyError::Tls(e))?;

    Ok(tls_config)
}

/// Run the proxy server (TLS or plain TCP based on config).
pub async fn run_server(config: Config, stats: Arc<Stats>) -> Result<()> {
    let listener = TcpListener::bind(&config.listen).await?;

    if config.no_tls {
        info!("Listening on {} (plain TCP)", config.listen);
    } else {
        info!("Listening on {} (TLS)", config.listen);
    }
    info!(
        "Forwarding to {} ({})",
        config.upstream,
        if config.upstream_tls { "TLS" } else { "plain TCP" }
    );

    if config.no_tls {
        run_plain_server(listener, config, stats).await
    } else {
        run_tls_server(listener, config, stats).await
    }
}

/// Run the server accepting plain TCP connections.
async fn run_plain_server(
    listener: TcpListener,
    config: Config,
    stats: Arc<Stats>,
) -> Result<()> {
    loop {
        let (tcp_stream, peer_addr) = listener.accept().await?;
        let upstream_addr = config.upstream.clone();
        let upstream_tls = config.upstream_tls;
        let upstream_hostname = config.upstream_hostname();
        let stats = stats.clone();

        tokio::spawn(async move {
            info!("New connection from {}", peer_addr);

            // Connect to upstream
            let upstream = match UpstreamConnection::connect(
                &upstream_addr,
                upstream_tls,
                &upstream_hostname,
            )
            .await
            {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to connect to upstream {}: {}", upstream_addr, e);
                    return;
                }
            };

            // Proxy the connection
            proxy_connection(tcp_stream, upstream, stats).await;
            info!("Connection from {} closed", peer_addr);
        });
    }
}

/// Run the server accepting TLS connections.
async fn run_tls_server(
    listener: TcpListener,
    config: Config,
    stats: Arc<Stats>,
) -> Result<()> {
    let tls_config = build_tls_config(&config)?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    loop {
        let (tcp_stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let upstream_addr = config.upstream.clone();
        let upstream_tls = config.upstream_tls;
        let upstream_hostname = config.upstream_hostname();
        let stats = stats.clone();

        tokio::spawn(async move {
            info!("New connection from {}", peer_addr);

            // Accept TLS connection from client
            let tls_stream = match acceptor.accept(tcp_stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("TLS handshake failed for {}: {}", peer_addr, e);
                    return;
                }
            };

            // Connect to upstream
            let upstream = match UpstreamConnection::connect(
                &upstream_addr,
                upstream_tls,
                &upstream_hostname,
            )
            .await
            {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to connect to upstream {}: {}", upstream_addr, e);
                    return;
                }
            };

            // Proxy the connection
            proxy_connection(tls_stream, upstream, stats).await;
            info!("Connection from {} closed", peer_addr);
        });
    }
}
