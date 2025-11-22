# Secure Backend Service for JavaScript Code Execution

Three implementations of a secure backend service for executing user-defined JavaScript code in isolated sandboxes:

1. **Python FastAPI** - Using PyMiniRacer (V8 engine)
2. **Node.js Fastify** - Using QuickJS WebAssembly  
3. **Rust Actix-web** - Using Rust performance with HTTP support

All implementations expose the same REST API and support the same features.

## Repository Structure

```
.
├── python-fastapi/      # Python FastAPI implementation
├── nodejs-fastify/      # Node.js Fastify implementation  
├── rust-actix/          # Rust Actix-web implementation
├── wiremock/            # Preconfigured WireMock mappings
├── benchmark.py         # Comprehensive benchmarking script
├── benchmark.sh         # Quick benchmark script
├── docker-compose.yml   # Run all services together
├── COMPARISON.md        # Detailed comparison of implementations
├── BENCHMARKING.md      # Benchmarking guide
└── QUICKSTART.md        # Quick start guide
```

## Features

- **Secure Execution**: Code runs in isolated sandboxes (V8/QuickJS)
- **HTTP Support**: Provides `httpGet()` function for making web requests from user code
- **Input Injection**: Inject custom data as `INPUTS` global variable
- **Multi-Implementation**: Compare performance across Python, Node.js, and Rust
- **Containerized**: Docker and docker-compose.yml for easy deployment
- **WireMock Integration**: Preconfigured mock API endpoints for local testing
- **Benchmarking**: Comprehensive performance comparison tools

## API

### POST /execute

Executes user-defined JavaScript code in a secure sandbox.

**Request Body:**
```json
{
  "code": "string",     // JavaScript code to execute
  "inputs": {}          // Object to inject as INPUTS global variable
}
```

**Example 1: Basic Math**
```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"code": "INPUTS.x + INPUTS.y", "inputs": {"x": 5, "y": 10}}'
```

Response: `{"result": 15}`

**Example 2: HTTP Request**
```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"code": "const response = httpGet(\"http://api.example.com/data\"); response.data", "inputs": {}}'
```

Response: `{"result": {...}}`

### GET /health

Health check endpoint.

Response: `{"status": "ok"}`

## Quick Start with Docker Compose

Start all three implementations plus WireMock:

```bash
docker-compose up
```

This starts:
- **Python service** on port 3000
- **Node.js service** on port 3001  
- **Rust service** on port 3002
- **WireMock** on port 8080

## Individual Service Setup

Each implementation has its own directory with specific setup instructions:

### Python FastAPI (Port 3000)
```bash
cd python-fastapi
pip install -r requirements.txt
python -m uvicorn src.main:app --host 0.0.0.0 --port 3000
```

### Node.js Fastify (Port 3001)
```bash
cd nodejs-fastify
npm install
npm run build
npm start
```

### Rust Actix-web (Port 3002)
```bash
cd rust-actix
cargo build --release
cargo run --release
```

See individual README files in each directory for more details.

## Benchmarking

Compare performance across all three implementations:

### Comprehensive Benchmark (Python script)

```bash
# Start all services first
docker-compose up -d

# Run benchmark
python3 benchmark.py
```

This runs:
- Sequential requests (50 per test case)
- Concurrent requests (100 requests, 10 concurrent)
- Multiple test cases (arithmetic, strings, arrays, complex calculations)
- Generates detailed statistics (mean, median, min, max, RPS, throughput)

### Quick Benchmark (Shell script)

```bash
# Start all services first  
docker-compose up -d

# Run quick benchmark
./benchmark.sh
```

Simple benchmark with 10 requests per service.

## Performance Comparison

Each implementation has different characteristics:

- **Python FastAPI**: Easy development, good performance, excellent ecosystem
- **Node.js Fastify**: Fast, native JavaScript execution, mature QuickJS sandbox
- **Rust Actix-web**: Maximum performance, lowest memory footprint, type safety

Run the benchmarks to see actual performance metrics for your use case.

**See [COMPARISON.md](COMPARISON.md) for detailed comparison and [BENCHMARKING.md](BENCHMARKING.md) for benchmarking guide.**

## Security

- Code is executed in isolated sandboxes (V8 for Python, QuickJS for Node.js)
- No access to host APIs or file system from user code
- HTTP requests are proxied through the host with timeout protection
- All implementations passed CodeQL security scans

## Testing with WireMock

The service includes WireMock integration with preconfigured mappings for local development.

### Using Docker Compose (Recommended)

Start both services:
```bash
docker-compose up
```

### Preconfigured Mock Endpoints

The following endpoints are available in WireMock:

- `GET /api/todos/{id}` - Returns a todo item
- `GET /api/users` - Returns a list of users  
- `GET /api/data` - Returns sample data with message and value
- `POST /api/data` - Creates data (returns success message)

### Example Test

```bash
# Test with WireMock endpoint
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const response = httpGet(\"http://wiremock:8080/api/data\"); response.data.message",
    "inputs": {}
  }'
```

Expected response:
```json
{
  "result": "Hello from WireMock!"
}
```

### Adding Custom WireMock Mappings

Add JSON files to `wiremock/mappings/` directory. Example:

```json
{
  "request": {
    "method": "GET",
    "url": "/api/custom"
  },
  "response": {
    "status": 200,
    "jsonBody": {
      "custom": "data"
    }
  }
}
```

For more details, see the [tests/README.md](tests/README.md) file.

## API Functions Available in User Code

### `INPUTS`

Global variable containing the inputs passed in the request.

```javascript
const x = INPUTS.x;
const y = INPUTS.y;
```

### `httpGet(url, options?)`

Makes an HTTP request and returns the response.

**Parameters:**
- `url` (string): The URL to fetch
- `options` (object, optional): Request options (method, headers, body)

**Returns:**
```javascript
{
  ok: boolean,        // True if status 200-299
  status: number,     // HTTP status code
  statusText: string, // HTTP status text
  headers: object,    // Response headers
  data: any          // Response body (parsed JSON or text)
}
```

**Example:**
```javascript
const response = httpGet("https://api.example.com/users");
if (response.ok) {
  return response.data.length;
}
```