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

- **main.rs** - Entry point, logging setup, graceful shutdown handling with Ctrl+C
- **config.rs** - CLI argument parsing via clap with validation (cert/key required unless `--no-tls`)
- **server.rs** - TCP/TLS listener setup, accepts connections and spawns per-connection tasks
- **upstream.rs** - `UpstreamConnection` enum abstracting plain TCP vs TLS connections to Redis, implements `AsyncRead`/`AsyncWrite`
- **proxy.rs** - Bidirectional data forwarding with RESP protocol parsing to extract command names
- **stats.rs** - Thread-safe command counter using `AtomicU64` and `RwLock<HashMap>`
- **error.rs** - Custom `ProxyError` type using thiserror

### Data Flow

1. `server.rs` accepts client connection (TLS via `TlsAcceptor` or plain TCP)
2. Creates `UpstreamConnection` to Redis server
3. `proxy_connection()` runs bidirectional copy loop using `tokio::select!`
4. Client→upstream direction: parses RESP to count commands via `parse_commands()`
5. Upstream→client direction: passthrough only
6. On shutdown, `Stats::print_summary()` outputs command breakdown

### RESP Parsing

The `proxy.rs` module parses Redis Serialization Protocol (RESP) to extract command names:
- Handles RESP arrays (`*<count>\r\n`) with bulk string elements (`$<len>\r\n`)
- Also handles inline commands (space-separated, ending `\r\n`)
- Only parses the first element (command name) of each array, skips arguments
