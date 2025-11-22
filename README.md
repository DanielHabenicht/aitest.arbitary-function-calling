# Secure Backend Service for JavaScript Code Execution

A secure backend service built with Node.js, Fastify, and TypeScript that executes user-defined JavaScript code in a WebAssembly sandbox using QuickJS.

## Features

- **Secure Execution**: Uses `quickjs-emscripten` to run user code in an isolated WebAssembly sandbox
- **HTTP Support**: Provides `httpGet()` function for making web requests from user code
- **Input Injection**: Inject custom data as `INPUTS` global variable
- **Fast API**: Built on Fastify for high performance
- **Containerized**: Includes Dockerfile for easy deployment

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

- Node.js 20+ (for native fetch support)
- npm

### Install Dependencies

```bash
npm install
```

### Build

```bash
npm run build
```

### Start Server

```bash
npm start
```

The server will start on `http://0.0.0.0:3000` by default.

### Environment Variables

- `PORT`: Server port (default: 3000)
- `HOST`: Server host (default: 0.0.0.0)
- `NODE_ENV`: Environment mode (default: development)

## Docker

### Build Image

```bash
docker build -t js-execution-service .
```

### Run Container

```bash
docker run -p 3000:3000 js-execution-service
```

## Security

- Code is executed in a WebAssembly sandbox using QuickJS
- Execution timeout of 10 seconds to prevent infinite loops
- No access to Node.js APIs or file system from user code
- HTTP requests are  proxied through the host

## Testing

The service includes comprehensive test infrastructure with WireMock for local development without dependencies on third-party APIs.

### Running Tests

```bash
npm test
```

### Manual Testing with WireMock

1. Start WireMock:
```bash
docker run -it --rm -p 8080:8080 wiremock/wiremock:3.3.1
```

2. Set up a mock endpoint:
```bash
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {"method": "GET", "url": "/api/data"},
    "response": {
      "status": 200,
      "jsonBody": {"message": "Hello from WireMock!"}
    }
  }'
```

3. Test the service:
```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const response = httpGet(\"http://localhost:8080/api/data\"); response.data.message",
    "inputs": {}
  }'
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