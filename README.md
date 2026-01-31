# Redis TLS Proxy

A lightweight TLS proxy for Redis connections written in Rust. It sits between Redis clients and servers, optionally terminating or initiating TLS connections while parsing and counting Redis commands passing through.

## Features

- **TLS Termination**: Accept TLS connections from clients and forward to plain TCP Redis servers
- **TLS Initiation**: Connect to TLS-enabled Redis servers (e.g., cloud Redis services)
- **Flexible Modes**: Support all combinations of TLS/plain TCP on both client and upstream sides
- **Command Counting**: Parse RESP protocol and track Redis command statistics
- **Graceful Shutdown**: Clean shutdown with command statistics summary on Ctrl+C

## Use Cases

- Add TLS to a local Redis instance without modifying Redis configuration
- Connect legacy non-TLS clients to TLS-only Redis services (AWS ElastiCache, Azure Cache, etc.)
- Monitor and count Redis commands in development/debugging
- Protocol-aware proxy for Redis traffic analysis

## Installation

Install [bun](https://bun.sh/docs/installation) for running test without needing redis dependencies:

```bash
curl -fsSL https://bun.sh/install | bash
```

Optional: Build redis proxy
```bash
cargo build --release
```

## Quickstart

Start local redis instance on port 6379:

```bash
docker compose up -d
```

Run proxy:

```bash
cargo run -- --no-tls
```

In a new terminal, run bench.ts or test.ts:

```bash
[ENV_VAR="here"] bun run scripts/bench.ts
```

Env vars for bench.ts:

```sh
PROXY_URL="redis://localhost:16379"
NUM_CLIENTS=10
OPS_PER_CLIENT=1000
```

## Usage

### Basic Examples

When you built the runtime, execute them inside `target/release/`.

**TLS proxy to local Redis:**
```bash
redis-tls-proxy \
  --listen 0.0.0.0:16379 \
  --upstream 127.0.0.1:6379 \
  --cert server.crt \
  --key server.key
```

**Plain TCP proxy to TLS Redis (e.g., AWS ElastiCache):**
```bash
redis-tls-proxy \
  --listen 127.0.0.1:6379 \
  --upstream my-redis.cache.amazonaws.com:6379 \
  --no-tls \
  --upstream-tls
```

**TLS-to-TLS proxy:**
```bash
redis-tls-proxy \
  --listen 0.0.0.0:16379 \
  --upstream my-redis.cache.amazonaws.com:6379 \
  --cert server.crt \
  --key server.key \
  --upstream-tls
```

**Plain TCP passthrough (for command counting only):**
```bash
redis-tls-proxy \
  --listen 127.0.0.1:16379 \
  --upstream 127.0.0.1:6379 \
  --no-tls
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --listen` | Address to listen on | `0.0.0.0:16379` |
| `-u, --upstream` | Upstream Redis server address | `127.0.0.1:6379` |
| `-c, --cert` | Path to TLS certificate (PEM) | Required unless `--no-tls` |
| `-k, --key` | Path to TLS private key (PEM) | Required unless `--no-tls` |
| `--no-tls` | Disable TLS on listening side | `false` |
| `--upstream-tls` | Enable TLS for upstream connection | `false` |
| `--upstream-tls-hostname` | Hostname for upstream TLS verification | Extracted from upstream address |

### Logging

Set the `RUST_LOG` environment variable to control log verbosity:

```bash
RUST_LOG=debug redis-tls-proxy --no-tls
RUST_LOG=redis_tls_proxy=trace redis-tls-proxy --no-tls
```

## Project Structure

```
redis-tls-proxy/
├── src/
│   ├── main.rs       # Entry point and orchestration
│   ├── config.rs     # CLI configuration
│   ├── error.rs      # Error types
│   ├── resp.rs       # RESP protocol parsing
│   ├── tls.rs        # TLS utilities
│   ├── server.rs     # TCP/TLS listener
│   ├── upstream.rs   # Upstream connection
│   ├── proxy.rs      # Bidirectional forwarding
│   └── stats.rs      # Command statistics
├── scripts/
│   ├── test.ts       # Functional tests
│   └── bench.ts      # Performance benchmarks
├── Cargo.toml
├── docker-compose.yml
└── README.md
```

## Architecture

```
┌────────────────┐      ┌──────────────────┐      ┌─────────────────┐
│  Redis Client  │─────>│  redis-tls-proxy │─────>│  Redis Server   │
│                │ TLS/ │                  │ TLS/ │                 │
│                │ TCP  │  (parses RESP)   │ TCP  │                 │
└────────────────┘      └──────────────────┘      └─────────────────┘
```

The proxy:
1. Accepts incoming connections (TLS or plain TCP)
2. Establishes a connection to the upstream Redis server
3. Bidirectionally forwards data between client and server
4. Parses the RESP protocol in client-to-server direction to count commands
5. Prints command statistics on shutdown

## Command Statistics

When the proxy shuts down (Ctrl+C), it prints a summary of all Redis commands seen:

```
=== Command Statistics ===
Total commands: 1523

Per-command breakdown:
  GET: 842
  SET: 456
  PING: 125
  HGET: 100
==========================
```

## License

MIT
