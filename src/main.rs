mod config;
mod error;
mod proxy;
mod server;
mod stats;
mod upstream;

use anyhow::Result;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::config::Config;
use crate::server::run_server;
use crate::stats::Stats;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("redis_tls_proxy=info".parse()?))
        .init();

    let config = Config::parse_args().map_err(|e| anyhow::anyhow!(e))?;

    info!("Starting Redis TLS Proxy");

    // Create shared stats
    let stats = Stats::new();
    let stats_for_shutdown = stats.clone();

    // Run server with graceful shutdown
    tokio::select! {
        result = run_server(config, stats) => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
                return Err(e.into());
            }
        }
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal, stopping...");
        }
    }

    // Print stats on shutdown
    stats_for_shutdown.print_summary();

    info!("Server stopped");
    Ok(())
}
