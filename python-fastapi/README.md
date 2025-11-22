# Python FastAPI Implementation

Secure JavaScript code execution service using Python FastAPI and PyMiniRacer (V8).

## Features

- FastAPI with automatic OpenAPI documentation
- PyMiniRacer for V8 JavaScript execution
- Two-pass HTTP execution model
- Async/await support

## Quick Start

### Install Dependencies

```bash
pip install -r requirements.txt
```

### Run Server

```bash
python -m uvicorn src.main:app --host 0.0.0.0 --port 3000 --reload
```

### Test

```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"code": "INPUTS.x + INPUTS.y", "inputs": {"x": 20, "y": 22}}'
```

### Run with Docker

```bash
docker build -t python-js-service .
docker run -p 3000:3000 python-js-service
```

## API Documentation

- Swagger UI: http://localhost:3000/docs
- ReDoc: http://localhost:3000/redoc
