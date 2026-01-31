use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "redis-tls-proxy")]
#[command(about = "A TLS proxy for Redis connections")]
pub struct Config {
    /// Address to listen on (e.g., 0.0.0.0:16379)
    #[arg(short, long, default_value = "0.0.0.0:16379")]
    pub listen: String,

    /// Upstream Redis server address (e.g., 127.0.0.1:6379)
    #[arg(short, long, default_value = "127.0.0.1:6379")]
    pub upstream: String,

    /// Path to TLS certificate file (PEM format). Required unless --no-tls is set.
    #[arg(short, long)]
    pub cert: Option<PathBuf>,

    /// Path to TLS private key file (PEM format). Required unless --no-tls is set.
    #[arg(short, long)]
    pub key: Option<PathBuf>,

    /// Disable TLS on the listening side (for local development)
    #[arg(long, default_value = "false")]
    pub no_tls: bool,

    /// Enable TLS for upstream connection
    #[arg(long, default_value = "false")]
    pub upstream_tls: bool,

    /// Upstream server hostname for TLS verification (defaults to upstream host)
    #[arg(long)]
    pub upstream_tls_hostname: Option<String>,
}

impl Config {
    pub fn parse_args() -> Result<Self, String> {
        let config = Config::parse();
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if !self.no_tls {
            if self.cert.is_none() {
                return Err("--cert is required when TLS is enabled (use --no-tls to disable)".to_string());
            }
            if self.key.is_none() {
                return Err("--key is required when TLS is enabled (use --no-tls to disable)".to_string());
            }
        }
        Ok(())
    }

    pub fn upstream_hostname(&self) -> String {
        self.upstream_tls_hostname
            .clone()
            .unwrap_or_else(|| {
                self.upstream
                    .split(':')
                    .next()
                    .unwrap_or("localhost")
                    .to_string()
            })
    }
}
