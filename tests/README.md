# Testing with WireMock

This directory contains documentation for using WireMock to mock external HTTP APIs, enabling local development and testing without dependencies on third-party services.

## Quick Start with Docker Compose

The easiest way to test with WireMock:

```bash
# From project root
docker-compose up
```

This automatically starts:
- JavaScript execution service on port 3000
- WireMock server with preconfigured mappings on port 8080

## Preconfigured Mappings

The following mappings are available in `wiremock/mappings/`:

### GET /api/todos/{id}
Returns a todo item with dynamic ID.

### GET /api/users
Returns a list of users.

### GET /api/data
Returns sample data with message and value.

### POST /api/data
Creates data (returns success response).

## Manual Testing with WireMock

You can test the service manually with WireMock:

### 1. Start WireMock

```bash
docker run -it --rm -p 8080:8080 wiremock/wiremock:3.3.1
```

### 2. Set up a mock endpoint

```bash
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {
      "method": "GET",
      "url": "/api/data"
    },
    "response": {
      "status": 200,
      "headers": {
        "Content-Type": "application/json"
      },
      "jsonBody": {
        "message": "Hello from WireMock!",
        "timestamp": 1234567890
      }
    }
  }'
```

### 3. Start the execution service

```bash
npm start
```

### 4. Test the execution endpoint

```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const response = httpGet(\"http://localhost:8080/api/data\"); response.data.message",
    "inputs": {}
  }'
```

Expected response:
```json
{
  "result": "Hello from WireMock!"
}
```

## Example Test Scenarios

### Testing with Mock JSON API

```bash
# Set up mock user endpoint
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {
      "method": "GET",
      "url": "/api/users/1"
    },
    "response": {
      "status": 200,
      "headers": {
        "Content-Type": "application/json"
      },
      "jsonBody": {
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
      }
    }
  }'

# Test with the execution service
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const user = httpGet(\"http://localhost:8080/api/users/1\"); user.data.name",
    "inputs": {}
  }'
```

### Testing with Conditional Logic

```bash
# Set up mock endpoint
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {
      "method": "GET",
      "url": "/api/status"
    },
    "response": {
      "status": 200,
      "headers": {
        "Content-Type": "application/json"
      },
      "jsonBody": {
        "active": true,
        "count": 42
      }
    }
  }'

# Test with conditional code
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "code": "const status = httpGet(\"http://localhost:8080/api/status\"); status.data.active ? status.data.count * 2 : 0",
    "inputs": {}
  }'
```

## WireMock Admin API

### List all mappings

```bash
curl http://localhost:8080/__admin/mappings
```

### Clear all mappings

```bash
curl -X DELETE http://localhost:8080/__admin/mappings
```

### View requests

```bash
curl http://localhost:8080/__admin/requests
```

## Benefits of WireMock Testing

1. **No External Dependencies**: Test without relying on third-party APIs
2. **Consistent Results**: Mock responses are predictable and repeatable
3. **Fast Tests**: No network latency from real API calls
4. **Edge Case Testing**: Easily test error scenarios and edge cases
5. **Offline Development**: Work without internet connection
6. **Cost Effective**: No API rate limits or usage costs

## Advanced WireMock Features

### Simulating Delays

```bash
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {
      "method": "GET",
      "url": "/api/slow"
    },
    "response": {
      "status": 200,
      "fixedDelayMilliseconds": 3000,
      "jsonBody": {
        "message": "Slow response"
      }
    }
  }'
```

### Simulating Errors

```bash
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {
      "method": "GET",
      "url": "/api/error"
    },
    "response": {
      "status": 500,
      "jsonBody": {
        "error": "Internal Server Error"
      }
    }
  }'
```

### Pattern Matching

```bash
curl -X POST http://localhost:8080/__admin/mappings \
  -H "Content-Type: application/json" \
  -d '{
    "request": {
      "method": "GET",
      "urlPattern": "/api/users/[0-9]+"
    },
    "response": {
      "status": 200,
      "jsonBody": {
        "id": "{{request.path.[2]}}",
        "name": "User {{request.path.[2]}}"
      },
      "transformers": ["response-template"]
    }
  }'
```
