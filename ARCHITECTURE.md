# Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Docker Compose                           │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Python     │  │   Node.js    │  │     Rust     │          │
│  │   FastAPI    │  │   Fastify    │  │     Axum     │          │
│  │              │  │              │  │              │          │
│  │  Port 3000   │  │  Port 3001   │  │  Port 3002   │          │
│  │              │  │              │  │              │          │
│  │ PyMiniRacer  │  │   QuickJS    │  │  rquickjs    │          │
│  │    (V8)      │  │  (WASM)      │  │   (Simple)   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                  │
│         └──────────────────┴──────────────────┘                  │
│                            │                                      │
│                            │ HTTP Requests                        │
│                            ↓                                      │
│                   ┌────────────────┐                             │
│                   │    WireMock    │                             │
│                   │   Port 8080    │                             │
│                   │                │                             │
│                   │  Mock API      │                             │
│                   │  Endpoints     │                             │
│                   └────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

## Component Details

### Python FastAPI Service (Port 3000)

```
┌───────────────────────────────────────┐
│         FastAPI Application           │
├───────────────────────────────────────┤
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   POST /execute                 │ │
│  │   - Receives code + inputs      │ │
│  │   - First pass: discover HTTP   │ │
│  │   - Execute HTTP requests       │ │
│  │   - Second pass: with results   │ │
│  └─────────────────────────────────┘ │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   PyMiniRacer (V8)              │ │
│  │   - Full JavaScript support     │ │
│  │   - INPUTS injection            │ │
│  │   - httpGet() function          │ │
│  └─────────────────────────────────┘ │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   HTTPX Client                  │ │
│  │   - Async HTTP requests         │ │
│  │   - Timeout protection          │ │
│  └─────────────────────────────────┘ │
└───────────────────────────────────────┘
```

### Node.js Fastify Service (Port 3001)

```
┌───────────────────────────────────────┐
│      Fastify Application (TS)         │
├───────────────────────────────────────┤
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   POST /execute                 │ │
│  │   - Receives code + inputs      │ │
│  │   - First pass: discover HTTP   │ │
│  │   - Execute HTTP requests       │ │
│  │   - Second pass: with results   │ │
│  └─────────────────────────────────┘ │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   QuickJS (WASM)                │ │
│  │   - WebAssembly sandbox         │ │
│  │   - INPUTS injection            │ │
│  │   - httpGet() function          │ │
│  └─────────────────────────────────┘ │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   Node.js fetch                 │ │
│  │   - Native fetch API            │ │
│  │   - Timeout protection          │ │
│  └─────────────────────────────────┘ │
└───────────────────────────────────────┘
```

### Rust Axum Service (Port 3002)

```
┌───────────────────────────────────────┐
│      Axum Application                 │
├───────────────────────────────────────┤
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   POST /execute                 │ │
│  │   - Receives code + inputs      │ │
│  │   - First pass: discover HTTP   │ │
│  │   - Execute HTTP requests       │ │
│  │   - Second pass: with results   │ │
│  └─────────────────────────────────┘ │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   rquickjs (QuickJS)            │ │
│  │   - Full JavaScript support     │ │
│  │   - INPUTS injection            │ │
│  │   - httpGet() function          │ │
│  └─────────────────────────────────┘ │
│                                       │
│  ┌─────────────────────────────────┐ │
│  │   Reqwest Client                │ │
│  │   - Async HTTP with Tokio       │ │
│  │   - Timeout protection          │ │
│  └─────────────────────────────────┘ │
└───────────────────────────────────────┘
```

## Request Flow

### Simple Execution (No HTTP)

```
Client Request
     │
     ▼
┌─────────────────┐
│  Service        │
│  (Any Port)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Parse Request  │
│  code + inputs  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Create VM      │
│  Inject INPUTS  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Execute Code   │
│  Return Result  │
└────────┬────────┘
         │
         ▼
    Response
```

### Execution with HTTP (Two-Pass)

```
Client Request
     │
     ▼
┌──────────────────┐
│  Service         │
│  (Any Port)      │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  First Pass      │
│  - Create VM     │
│  - Execute code  │
│  - Collect URLs  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  HTTP Requests   │
│  - Parallel      │
│  - To WireMock   │
│  - Cache results │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Second Pass     │
│  - Fresh VM      │
│  - Inject cache  │
│  - Re-execute    │
└────────┬─────────┘
         │
         ▼
    Response
```

## WireMock Integration

```
┌─────────────────────────────────────┐
│          WireMock Server            │
│           Port 8080                 │
├─────────────────────────────────────┤
│                                     │
│  ┌───────────────────────────────┐ │
│  │  GET /api/todos/{id}          │ │
│  │  → Dynamic todo items         │ │
│  └───────────────────────────────┘ │
│                                     │
│  ┌───────────────────────────────┐ │
│  │  GET /api/users               │ │
│  │  → User list                  │ │
│  └───────────────────────────────┘ │
│                                     │
│  ┌───────────────────────────────┐ │
│  │  GET /api/data                │ │
│  │  → Sample data                │ │
│  └───────────────────────────────┘ │
│                                     │
│  ┌───────────────────────────────┐ │
│  │  POST /api/data               │ │
│  │  → Create response            │ │
│  └───────────────────────────────┘ │
│                                     │
│  Mappings from: wiremock/mappings/│
└─────────────────────────────────────┘
```

## Benchmarking Architecture

```
┌──────────────────────────────────────┐
│      benchmark.py / benchmark.sh     │
└────────────┬─────────────────────────┘
             │
             ├──► Python Service (3000)
             │    └─► Execute requests
             │        └─► Measure latency
             │
             ├──► Node.js Service (3001)
             │    └─► Execute requests
             │        └─► Measure latency
             │
             └──► Rust Service (3002)
                  └─► Execute requests
                      └─► Measure latency
                      
           Results & Statistics
                    │
                    ▼
         ┌─────────────────────┐
         │  Comparison Tables   │
         │  - Mean latency     │
         │  - Throughput (RPS) │
         │  - Consistency      │
         └─────────────────────┘
```

## Security Model

```
┌──────────────────────────────────────┐
│         Client Request               │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│      Validation Layer                │
│      - Check code parameter          │
│      - Validate JSON                 │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│      Sandbox Environment             │
│      - Isolated VM (V8/QuickJS)      │
│      - No file system access         │
│      - No process access             │
│      - Limited APIs                  │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│      HTTP Proxy Layer                │
│      - Controlled HTTP access        │
│      - Timeout protection            │
│      - Host validation               │
└────────────┬─────────────────────────┘
             │
             ▼
         External APIs
     (WireMock or Internet)
```

## Deployment Options

### Development
```
docker-compose up
```

### Production - Individual Services
```
Python:   docker run -p 3000:3000 python-js-service
Node.js:  docker run -p 3001:3000 nodejs-js-service
Rust:     docker run -p 3002:3000 rust-js-service
```

### Production - Kubernetes
```
┌─────────────────────────────────────┐
│         Load Balancer               │
└────────────┬────────────────────────┘
             │
    ┌────────┴────────┬────────────┐
    │                 │            │
    ▼                 ▼            ▼
┌────────┐      ┌────────┐   ┌────────┐
│Python  │      │Node.js │   │ Rust   │
│Pod(s)  │      │Pod(s)  │   │Pod(s)  │
└────────┘      └────────┘   └────────┘
```

## Technology Stack Summary

| Layer | Python | Node.js | Rust |
|-------|--------|---------|------|
| **Web Framework** | FastAPI | Fastify | Axum |
| **Runtime** | CPython 3.11+ | Node.js 20+ | Native |
| **JS Engine** | PyMiniRacer (V8) | QuickJS | rquickjs (QuickJS) |
| **Async** | asyncio | Event loop | Tokio (multi-threaded) |
| **HTTP Client** | httpx | fetch | reqwest |
| **Serialization** | Pydantic | TypeScript | Serde |
| **Container Base** | python:3.11-slim | node:20-alpine | debian:bookworm-slim |

## Performance Characteristics

```
                Low Latency ────────────► High Latency
                      │                         │
    Rust ────────────►│                         │
    Node.js ──────────►                         │
    Python ───────────────────────────────────►│
                      
                High Throughput ────────► Low Throughput
                      │                         │
    Rust ────────────►│                         │
    Node.js ──────────►                         │
    Python ────────────────────►                │
                      
                Low Memory ──────────────► High Memory
                      │                         │
    Rust ────────────►│                         │
    Node.js ───────────────►                    │
    Python ────────────────────────────────────►│
```

Run benchmarks to see actual numbers for your environment!
