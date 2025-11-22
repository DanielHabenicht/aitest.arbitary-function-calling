# Secure Backend Service for JavaScript Code Execution

A secure backend service built with Python, FastAPI that executes user-defined JavaScript code in a V8 sandbox using PyMiniRacer.

## Features

- **Secure Execution**: Uses PyMiniRacer (V8 engine) to run user code in an isolated sandbox
- **HTTP Support**: Provides `httpGet()` function for making web requests from user code
- **Input Injection**: Inject custom data as `INPUTS` global variable
- **Fast API**: Built on FastAPI for high performance and automatic API documentation
- **Containerized**: Includes Dockerfile and docker-compose.yml with WireMock for testing
- **WireMock Integration**: Preconfigured mock API endpoints for local development

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

## Development

### Prerequisites

- Python 3.11+
- pip

### Install Dependencies

```bash
pip install -r requirements.txt
```

### Start Server

```bash
python -m uvicorn src.main:app --host 0.0.0.0 --port 3000 --reload
```

The server will start on `http://0.0.0.0:3000` by default.

### API Documentation

FastAPI automatically generates interactive API documentation:
- Swagger UI: http://localhost:3000/docs
- ReDoc: http://localhost:3000/redoc

### Environment Variables

- `PORT`: Server port (default: 3000)

## Docker & Docker Compose

### Using Docker Compose (Recommended)

The easiest way to run the service with WireMock:

```bash
docker-compose up
```

This starts:
- JavaScript execution service on port 3000
- WireMock server with preconfigured mappings on port 8080

### Build and Run with Docker

```bash
docker build -t js-execution-service .
docker run -p 3000:3000 js-execution-service
```

## Security

- Code is executed in a V8 sandbox using PyMiniRacer
- Isolated execution environment with no access to Python APIs or file system
- HTTP requests are proxied through the host with timeout protection

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