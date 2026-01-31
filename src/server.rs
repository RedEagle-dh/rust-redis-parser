/// TCP/TLS server implementation for accepting client connections.

use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

use crate::config::Config;
use crate::error::Result;
use crate::proxy::proxy_connection;
use crate::stats::Stats;
use crate::tls::build_server_config;
use crate::upstream::UpstreamConnection;

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
    let tls_config = build_server_config(&config)?;
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
