use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;

use crate::error::{ProxyError, Result};

/// Represents a connection to the upstream Redis server.
/// Can be either plain TCP or TLS-encrypted.
pub enum UpstreamConnection {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl UpstreamConnection {
    /// Connect to upstream Redis server over plain TCP.
    pub async fn connect_plain(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(UpstreamConnection::Plain(stream))
    }

    /// Connect to upstream Redis server over TLS.
    pub async fn connect_tls(addr: &str, hostname: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;

        // Use the system root certificates
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        let server_name = ServerName::try_from(hostname.to_string())
            .map_err(|_| ProxyError::Connection(format!("Invalid server name: {}", hostname)))?;

        let tls_stream = connector.connect(server_name, stream).await?;

        Ok(UpstreamConnection::Tls(tls_stream))
    }

    /// Connect to upstream based on configuration.
    pub async fn connect(addr: &str, use_tls: bool, hostname: &str) -> Result<Self> {
        if use_tls {
            Self::connect_tls(addr, hostname).await
        } else {
            Self::connect_plain(addr).await
        }
    }
}

impl AsyncRead for UpstreamConnection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            UpstreamConnection::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            UpstreamConnection::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for UpstreamConnection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            UpstreamConnection::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            UpstreamConnection::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            UpstreamConnection::Plain(stream) => Pin::new(stream).poll_flush(cx),
            UpstreamConnection::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            UpstreamConnection::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            UpstreamConnection::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
