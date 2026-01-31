/// Bidirectional proxy between client and upstream Redis connections.

use std::sync::Arc;

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{debug, error};

use crate::resp::parse_commands;
use crate::stats::Stats;

/// Proxy data bidirectionally between client and upstream connections,
/// counting Redis commands in the client->upstream direction.
pub async fn proxy_connection<C, U>(mut client: C, mut upstream: U, stats: Arc<Stats>)
where
    C: AsyncRead + AsyncWrite + Unpin,
    U: AsyncRead + AsyncWrite + Unpin,
{
    let mut client_buf = BytesMut::with_capacity(8192);
    let mut upstream_buf = BytesMut::with_capacity(8192);
    let mut client_temp = [0u8; 8192];
    let mut upstream_temp = [0u8; 8192];

    loop {
        tokio::select! {
            // Client -> Upstream (parse commands)
            result = client.read(&mut client_temp) => {
                match result {
                    Ok(0) => {
                        debug!("Client disconnected");
                        break;
                    }
                    Ok(n) => {
                        client_buf.extend_from_slice(&client_temp[..n]);

                        // Parse and count commands
                        let (commands, _consumed) = parse_commands(&client_buf);
                        for cmd in &commands {
                            debug!("Command: {}", cmd);
                            stats.record_command(cmd);
                        }

                        // Forward all data to upstream
                        if let Err(e) = upstream.write_all(&client_buf).await {
                            error!("Failed to write to upstream: {}", e);
                            break;
                        }
                        client_buf.clear();
                    }
                    Err(e) => {
                        error!("Failed to read from client: {}", e);
                        break;
                    }
                }
            }

            // Upstream -> Client (pass through)
            result = upstream.read(&mut upstream_temp) => {
                match result {
                    Ok(0) => {
                        debug!("Upstream disconnected");
                        break;
                    }
                    Ok(n) => {
                        upstream_buf.extend_from_slice(&upstream_temp[..n]);
                        if let Err(e) = client.write_all(&upstream_buf).await {
                            error!("Failed to write to client: {}", e);
                            break;
                        }
                        upstream_buf.clear();
                    }
                    Err(e) => {
                        error!("Failed to read from upstream: {}", e);
                        break;
                    }
                }
            }
        }
    }

    // Flush any remaining data
    let _ = client.flush().await;
    let _ = upstream.flush().await;
}
