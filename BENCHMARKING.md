# Benchmarking Guide

This repository includes comprehensive benchmarking tools to compare the performance of Python, Node.js, and Rust implementations.

## Benchmark Tools

### 1. Comprehensive Python Benchmark (`benchmark.py`)

Full-featured benchmarking with detailed statistics.

**Features:**
- Sequential request testing (50 requests per test case)
- Concurrent request testing (100 requests, 10 concurrent workers)
- Multiple test cases covering different workloads
- Detailed statistics: mean, median, min, max, standard deviation
- Throughput measurement (requests per second)
- Formatted table output

**Usage:**
```bash
# Start all services
docker-compose up -d

# Run benchmark
python3 benchmark.py
```

**Output:**
- Sequential benchmark results table
- Concurrent benchmark results table
- Summary comparison by service
- Average RPS and latency per service

### 2. Quick Shell Benchmark (`benchmark.sh`)

Simple, fast benchmark using curl and basic timing.

**Features:**
- 10 requests per service
- Simple timing measurement
- Basic RPS calculation
- No external dependencies (just bash, curl, bc)

**Usage:**
```bash
# Start all services
docker-compose up -d

# Run quick benchmark
./benchmark.sh
```

## Test Cases

The comprehensive benchmark includes these test cases:

1. **Simple Arithmetic**
   - Code: `INPUTS.x + INPUTS.y`
   - Tests basic execution overhead

2. **String Manipulation**
   - Code: `INPUTS.text.toUpperCase()`
   - Tests string operations

3. **Array Operations**
   - Code: `INPUTS.numbers.map(n => n * 2).reduce((a,b) => a + b, 0)`
   - Tests array processing

4. **Complex Calculation**
   - Code: `INPUTS.values.map(v => Math.sqrt(v)).filter(v => v > 5).length`
   - Tests mathematical operations and filtering

## Understanding Results

### Metrics Explained

- **Mean (ms)**: Average response time across all requests
- **Median (ms)**: Middle value, less affected by outliers
- **Min (ms)**: Fastest request
- **Max (ms)**: Slowest request
- **Stdev (ms)**: Standard deviation, measures consistency
- **RPS**: Requests per second (throughput)
- **Throughput (concurrent)**: Total requests per second under load

### What to Look For

1. **Latency** (Mean/Median): Lower is better
   - How long does a single request take?
   - Python typically has slightly higher latency due to interpreter overhead
   - Node.js and Rust are generally faster for pure execution
   - Rust often has the lowest latency

2. **Throughput** (RPS): Higher is better
   - How many requests can the service handle?
   - Rust typically has highest throughput
   - Node.js performs well with async/event-driven model
   - Python is competitive with async/await

3. **Consistency** (Stdev): Lower is better
   - How predictable are response times?
   - Lower standard deviation = more consistent performance
   - Important for user-facing applications

4. **Concurrency**: How well does it scale?
   - Concurrent benchmarks show behavior under load
   - Rust often handles concurrency best
   - Node.js event loop handles concurrency well
   - Python with uvicorn/FastAPI also scales well

## Sample Results

Expected performance characteristics:

```
Language    Typical Latency    Typical RPS    Memory    Cold Start
Python      5-15ms            100-200        ~50MB     ~2s
Node.js     3-10ms            150-300        ~30MB     ~1s
Rust        2-8ms             200-400        ~10MB     ~0.5s
```

*Actual results vary based on hardware, workload, and configuration.*

## Running Custom Benchmarks

### Modify Test Cases

Edit `benchmark.py` to add your own test cases:

```python
TEST_CASES = [
    {
        "name": "Your custom test",
        "code": "your JavaScript code here",
        "inputs": {"your": "inputs"}
    },
]
```

### Adjust Request Counts

In `benchmark.py`:
```python
# Change from default 50
run_benchmark(name, url, test_case, num_requests=100)

# Change concurrent from default 100/10
run_concurrent_benchmark(name, url, test_case, num_requests=200, concurrency=20)
```

### Benchmark Individual Services

```python
# Only benchmark specific service
SERVICES = {
    "Python FastAPI": "http://localhost:3000",
}
```

## Troubleshooting

### Services Not Available

If benchmark shows "Service not available":
```bash
# Check if services are running
docker-compose ps

# View logs
docker-compose logs python-service
docker-compose logs nodejs-service
docker-compose logs rust-service

# Restart services
docker-compose restart
```

### Slow Performance

First few requests may be slow due to cold start:
- Services need to initialize
- First V8/QuickJS context creation
- First HTTP connection

The benchmark includes warmup requests to mitigate this.

### High Variance

If you see high standard deviation:
- System may be under load from other processes
- Network latency if services aren't local
- Try increasing warmup requests
- Run benchmark multiple times and average

## Best Practices

1. **Close other applications** during benchmarking
2. **Run warmup** before measuring (included in scripts)
3. **Multiple runs** - Run benchmark 3-5 times, use median results
4. **Consistent environment** - Same hardware, same Docker resources
5. **Monitor resources** - Check CPU, memory during benchmark
6. **Test realistic workloads** - Use your actual use cases

## Interpreting for Your Use Case

Choose implementation based on your priorities:

- **Need lowest latency?** → Rust or Node.js
- **Need highest throughput?** → Rust
- **Need easiest development?** → Python
- **Need best JavaScript compatibility?** → Node.js
- **Need lowest memory usage?** → Rust
- **Need fastest cold start?** → Rust or Node.js

Remember: **The best choice depends on your specific requirements!**
