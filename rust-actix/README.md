# Rust Actix-web Implementation

Secure JavaScript code execution service using Rust, Actix-web, and rquickjs (QuickJS engine).

## Features

- Actix-web for high performance
- rquickjs (QuickJS) for JavaScript execution
- Two-pass HTTP execution model
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

## JavaScript Engine

Uses rquickjs, a Rust binding for the QuickJS JavaScript engine. This provides:
- Full ECMAScript 2020 support
- Low memory overhead
- Fast startup time
- Sandboxed execution
- Same engine as the Node.js implementation (QuickJS)
