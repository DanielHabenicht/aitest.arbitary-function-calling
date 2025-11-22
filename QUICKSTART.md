# Quick Start Guide

## Using Docker Compose (Recommended)

The easiest way to run the service with WireMock:

```bash
docker-compose up
```

This starts:
- JavaScript execution service on http://localhost:3000
- WireMock server with preconfigured mappings on http://localhost:8080

## Testing the Service

### 1. Health Check

```bash
curl http://localhost:3000/health
```

### 2. Basic Execution

```bash
curl -X POST http://localhost:3000/execute \
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

## API Documentation

When running, visit:
- Swagger UI: http://localhost:3000/docs
- ReDoc: http://localhost:3000/redoc

## Local Development

### Install Dependencies

```bash
pip install -r requirements-dev.txt
```

### Run Server

```bash
python -m uvicorn src.main:app --host 0.0.0.0 --port 3000 --reload
```

### Run Tests

```bash
pytest tests/
```

## Stopping Services

```bash
docker-compose down
```
