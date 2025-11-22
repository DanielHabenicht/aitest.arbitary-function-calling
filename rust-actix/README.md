# Rust Actix-web Implementation

Secure JavaScript code execution service using Rust and Actix-web.

## Features

- Actix-web for high performance
- Async/await with Tokio
- Type-safe API with Serde
- Low memory footprint

## Quick Start

### Build

```bash
cargo build --release
```

### Run Server

```bash
cargo run --release
```

### Test

```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"code": "INPUTS.x + INPUTS.y", "inputs": {"x": 20, "y": 22}}'
```

### Run with Docker

```bash
docker build -t rust-js-service .
docker run -p 3000:3000 rust-js-service
```

## Note

This implementation uses a simplified JavaScript execution model.
For production use, integrate with rusty_v8 for full V8 JavaScript engine support.
