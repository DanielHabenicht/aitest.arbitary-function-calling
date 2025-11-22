# Node.js Fastify Implementation

Secure JavaScript code execution service using Node.js, Fastify, and QuickJS.

## Features

- Fastify for high performance
- quickjs-emscripten for WebAssembly sandbox
- Two-pass HTTP execution model
- TypeScript support

## Quick Start

### Install Dependencies

```bash
npm install
```

### Build

```bash
npm run build
```

### Run Server

```bash
npm start
```

### Test

```bash
curl -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"code": "INPUTS.x + INPUTS.y", "inputs": {"x": 20, "y": 22}}'
```

### Run with Docker

```bash
docker build -t nodejs-js-service .
docker run -p 3000:3000 nodejs-js-service
```
