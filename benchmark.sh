#!/bin/bash

# Simple benchmark script using curl and time

echo "=================================================="
echo "Quick Benchmark - JavaScript Execution Services"
echo "=================================================="
echo ""

# Test cases
declare -a TEST_CASES=(
    '{"name":"Simple arithmetic","code":"INPUTS.x + INPUTS.y","inputs":{"x":20,"y":22}}'
    '{"name":"HTTP GET WireMock","code":"const response = httpGet('\''http://wiremock:8080/api/data'\''); response.data.message","inputs":{}}'
)

# Services
declare -A SERVICES=(
    ["Python FastAPI"]="http://localhost:3000"
    ["Node.js Fastify"]="http://localhost:3001"
    ["Rust Actix-web"]="http://localhost:3002"
)

# Number of requests per test
NUM_REQUESTS=10

echo "Number of requests per test: $NUM_REQUESTS"
echo ""

# Run tests for each service
for SERVICE_NAME in "${!SERVICES[@]}"; do
    SERVICE_URL="${SERVICES[$SERVICE_NAME]}"
    
    echo "Testing $SERVICE_NAME ($SERVICE_URL)..."
    
    # Check health
    if ! curl -s -f "${SERVICE_URL}/health" > /dev/null 2>&1; then
        echo "  ✗ Service not available"
        echo ""
        continue
    fi
    
    echo "  ✓ Service is healthy"
    
    # Run each test case
    for TEST_CASE in "${TEST_CASES[@]}"; do
        TEST_NAME=$(echo "$TEST_CASE" | python3 -c "import sys, json; print(json.load(sys.stdin)['name'])")
        CODE=$(echo "$TEST_CASE" | python3 -c "import sys, json; tc=json.load(sys.stdin); print(json.dumps({'code': tc['code'], 'inputs': tc['inputs']}))")
        
        echo ""
        echo "  Test: $TEST_NAME"
        
        # Warmup
        echo "    Warming up..."
        for i in {1..3}; do
            curl -s -X POST "${SERVICE_URL}/execute" \
                -H "Content-Type: application/json" \
                -d "$CODE" > /dev/null 2>&1
        done
        
        # Benchmark
        echo "    Running benchmark..."
        START=$(date +%s.%N)
        
        for i in $(seq 1 $NUM_REQUESTS); do
            curl -s -X POST "${SERVICE_URL}/execute" \
                -H "Content-Type: application/json" \
                -d "$CODE" > /dev/null 2>&1
        done
        
        END=$(date +%s.%N)
        DURATION=$(echo "$END - $START" | bc)
        RPS=$(echo "scale=2; $NUM_REQUESTS / $DURATION" | bc)
        AVG_MS=$(echo "scale=2; ($DURATION * 1000) / $NUM_REQUESTS" | bc)
        
        echo "    Results:"
        echo "      Total time: ${DURATION}s"
        echo "      Requests per second: ${RPS}"
        echo "      Average latency: ${AVG_MS}ms"
    done
    echo ""
done

echo "=================================================="
echo "Benchmark completed!"
echo "=================================================="
echo ""
echo "For more detailed benchmarks, run: python3 benchmark.py"
