# Quick Start Guide

## Using Docker Compose (Recommended)

The easiest way to run all three implementations with WireMock:

```bash
docker-compose up
```

This starts:
- **Python FastAPI service** on http://localhost:3000
- **Node.js Fastify service** on http://localhost:3001
- **Rust Axum service** on http://localhost:3002
- **WireMock server** with preconfigured mappings on http://localhost:8080

## Testing the Services

### 1. Health Check

Test all three services:

```bash
# Python service
curl http://localhost:3000/health

# Node.js service
curl http://localhost:3001/health

# Rust service
curl http://localhost:3002/health
```

### 2. Basic Execution

Test any service (ports 3000, 3001, 3002):

```bash
# Python service
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "INPUTS.x + INPUTS.y",
    "inputs": {"x": 20, "y": 22}
  }'

# Node.js service
curl -X POST http://localhost:3001/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "INPUTS.x + INPUTS.y",
    "inputs": {"x": 20, "y": 22}
  }'

# Rust service
curl -X POST http://localhost:3002/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "INPUTS.x + INPUTS.y",
    "inputs": {"x": 20, "y": 22}
  }'
```

Expected output: `{"result": 42}`

### 3. With HTTP Request (using WireMock)

```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const response = httpGet(\"http://wiremock:8080/api/data\"); response.data.message",
    "inputs": {}
  }'
```

Expected output: `{"result": "Hello from WireMock!"}`

### 4. Complex Example

```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const users = httpGet(\"http://wiremock:8080/api/users\"); users.data.map(u => u.name).join(\", \")",
    "inputs": {}
  }'
```

Expected output: `{"result": "John Doe, Jane Smith"}`

## Available WireMock Endpoints

- `GET /api/todos/{id}` - Returns a todo item
- `GET /api/users` - Returns a list of users
- `GET /api/data` - Returns sample data
- `POST /api/data` - Creates data

## Benchmarking

Compare performance across all three implementations:

```bash
# Quick benchmark (10 requests each)
./benchmark.sh

# Comprehensive benchmark (50+ requests each with statistics)
python3 benchmark.py
```

## API Documentation

Python FastAPI provides automatic documentation:
- Swagger UI: http://localhost:3000/docs
- ReDoc: http://localhost:3000/redoc

## Local Development

Each implementation has its own setup. See README files in:
- `python-fastapi/README.md`
- `nodejs-fastify/README.md`
- `rust/README.md`

### Python Development
```bash
cd python-fastapi
pip install -r requirements-dev.txt
python -m uvicorn src.main:app --host 0.0.0.0 --port 3000 --reload
pytest tests/
```

### Node.js Development
```bash
cd nodejs-fastify
npm install
npm run build
npm start
```

### Rust Development
```bash
cd rust
cargo build --release
cargo run --release
```

## Stopping Services

```bash
docker-compose down
```
