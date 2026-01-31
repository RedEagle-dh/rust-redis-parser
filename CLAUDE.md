# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run -- --no-tls    # Run with plain TCP (no certs needed)
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format code
```

## Architecture

This is a Redis TLS proxy that sits between clients and Redis servers, supporting any combination of TLS/plain TCP on both sides while parsing RESP protocol to count commands.

### Module Overview

```
src/
├── main.rs       - Entry point, logging setup, graceful shutdown with Ctrl+C
├── config.rs     - CLI argument parsing via clap with validation
├── error.rs      - Custom `ProxyError` type using thiserror
├── resp.rs       - RESP protocol parsing to extract command names
├── tls.rs        - TLS certificate/key loading and server config
├── server.rs     - TCP/TLS listener setup, spawns per-connection tasks
├── upstream.rs   - `UpstreamConnection` enum for plain TCP vs TLS to Redis
├── proxy.rs      - Bidirectional data forwarding between client and upstream
└── stats.rs      - Thread-safe command counter using AtomicU64 and RwLock

scripts/
├── test.ts       - Functional tests using Bun
└── bench.ts      - Performance benchmarks
```

### Data Flow

1. `server.rs` accepts client connection (TLS via `TlsAcceptor` or plain TCP)
2. Creates `UpstreamConnection` to Redis server
3. `proxy_connection()` runs bidirectional copy loop using `tokio::select!`
4. Client→upstream direction: parses RESP via `resp.rs` to count commands
5. Upstream→client direction: passthrough only
6. On shutdown, `Stats::print_summary()` outputs command breakdown

### RESP Parsing

The `resp.rs` module parses Redis Serialization Protocol (RESP) to extract command names:
- Handles RESP arrays (`*<count>\r\n`) with bulk string elements (`$<len>\r\n`)
- Also handles inline commands (space-separated, ending `\r\n`)
- Only parses the first element (command name) of each array, skips arguments
- Includes unit tests for parsing validation

### TLS Configuration

The `tls.rs` module handles TLS setup:
- `load_certs()` - Loads PEM certificates from file
- `load_private_key()` - Loads PEM private keys (PKCS1, PKCS8, SEC1)
- `build_server_config()` - Creates rustls `ServerConfig`
