#!/usr/bin/env python3
"""
Benchmarking script for comparing Python, Node.js, and Rust implementations
"""
import json
import time
import statistics
import requests
from typing import List, Dict, Any
from concurrent.futures import ThreadPoolExecutor, as_completed

# Service configurations
SERVICES = {
    "Python FastAPI": "http://localhost:3000",
    "Node.js Fastify": "http://localhost:3001",
    "Rust Actix-web": "http://localhost:3002",
}

# Test cases
TEST_CASES = [
    {
        "name": "Simple arithmetic",
        "code": "INPUTS.x + INPUTS.y",
        "inputs": {"x": 20, "y": 22}
    },
    {
        "name": "String manipulation",
        "code": "INPUTS.text.toUpperCase()",
        "inputs": {"text": "hello world from benchmark"}
    },
    {
        "name": "Array operations",
        "code": "INPUTS.numbers.map(n => n * 2).reduce((a,b) => a + b, 0)",
        "inputs": {"numbers": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}
    },
    {
        "name": "Complex calculation",
        "code": "INPUTS.values.map(v => Math.sqrt(v)).filter(v => v > 5).length",
        "inputs": {"values": [4, 9, 16, 25, 36, 49, 64, 81, 100]}
    },
    {
        "name": "HTTP GET WireMock data",
        "code": "const response = httpGet('http://wiremock:8080/api/data'); response.data.message",
        "inputs": {}
    },
    {
        "name": "HTTP GET WireMock users",
        "code": "const response = httpGet('http://wiremock:8080/api/users'); response.data.length",
        "inputs": {}
    },
]

def benchmark_request(service_url: str, test_case: Dict[str, Any]) -> float:
    """Execute a single request and return the time taken in seconds."""
    start = time.perf_counter()
    try:
        response = requests.post(
            f"{service_url}/execute",
            json={"code": test_case["code"], "inputs": test_case["inputs"]},
            timeout=10
        )
        end = time.perf_counter()
        if response.status_code != 200:
            return -1
        return end - start
    except Exception as e:
        print(f"Error: {e}")
        return -1

def run_warmup(service_url: str, test_case: Dict[str, Any], count: int = 5):
    """Warm up the service with a few requests."""
    print(f"  Warming up with {count} requests...")
    for _ in range(count):
        benchmark_request(service_url, test_case)
    time.sleep(0.5)

def run_benchmark(
    service_name: str,
    service_url: str,
    test_case: Dict[str, Any],
    num_requests: int = 100
) -> Dict[str, Any]:
    """Run benchmark for a specific service and test case."""
    print(f"\nBenchmarking {service_name} - {test_case['name']}")
    
    # Warm up
    run_warmup(service_url, test_case)
    
    # Sequential requests
    print(f"  Running {num_requests} sequential requests...")
    times = []
    for i in range(num_requests):
        t = benchmark_request(service_url, test_case)
        if t > 0:
            times.append(t)
        if (i + 1) % 20 == 0:
            print(f"    Completed {i + 1}/{num_requests}")
    
    if not times:
        return {"error": "All requests failed"}
    
    # Calculate statistics
    results = {
        "service": service_name,
        "test_case": test_case["name"],
        "requests": len(times),
        "mean_ms": statistics.mean(times) * 1000,
        "median_ms": statistics.median(times) * 1000,
        "min_ms": min(times) * 1000,
        "max_ms": max(times) * 1000,
        "stdev_ms": statistics.stdev(times) * 1000 if len(times) > 1 else 0,
        "requests_per_second": 1 / statistics.mean(times) if times else 0,
    }
    
    return results

def run_concurrent_benchmark(
    service_name: str,
    service_url: str,
    test_case: Dict[str, Any],
    num_requests: int = 100,
    concurrency: int = 10
) -> Dict[str, Any]:
    """Run concurrent benchmark for a specific service and test case."""
    print(f"\nConcurrent benchmark {service_name} - {test_case['name']} (concurrency: {concurrency})")
    
    times = []
    start_time = time.perf_counter()
    
    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [
            executor.submit(benchmark_request, service_url, test_case)
            for _ in range(num_requests)
        ]
        
        for i, future in enumerate(as_completed(futures)):
            t = future.result()
            if t > 0:
                times.append(t)
            if (i + 1) % 20 == 0:
                print(f"    Completed {i + 1}/{num_requests}")
    
    end_time = time.perf_counter()
    total_time = end_time - start_time
    
    if not times:
        return {"error": "All requests failed"}
    
    return {
        "service": service_name,
        "test_case": test_case["name"],
        "concurrency": concurrency,
        "requests": len(times),
        "total_time_s": total_time,
        "mean_ms": statistics.mean(times) * 1000,
        "median_ms": statistics.median(times) * 1000,
        "throughput_rps": len(times) / total_time,
    }

def check_health(service_name: str, service_url: str) -> bool:
    """Check if service is healthy."""
    try:
        response = requests.get(f"{service_url}/health", timeout=5)
        if response.status_code == 200:
            print(f"✓ {service_name} is healthy")
            return True
        else:
            print(f"✗ {service_name} returned status {response.status_code}")
            return False
    except Exception as e:
        print(f"✗ {service_name} is not accessible: {e}")
        return False

def print_results_table(results: List[Dict[str, Any]], title: str):
    """Print benchmark results in a formatted table."""
    print(f"\n{'=' * 100}")
    print(f"{title:^100}")
    print(f"{'=' * 100}")
    print(f"{'Service':<20} {'Test Case':<25} {'Mean (ms)':<12} {'Median (ms)':<12} {'Min (ms)':<10} {'Max (ms)':<10} {'RPS':<10}")
    print(f"{'-' * 100}")
    
    for r in results:
        if "error" not in r:
            print(f"{r['service']:<20} {r['test_case']:<25} {r['mean_ms']:<12.2f} {r['median_ms']:<12.2f} {r['min_ms']:<10.2f} {r['max_ms']:<10.2f} {r['requests_per_second']:<10.1f}")

def print_concurrent_results(results: List[Dict[str, Any]], title: str):
    """Print concurrent benchmark results."""
    print(f"\n{'=' * 100}")
    print(f"{title:^100}")
    print(f"{'=' * 100}")
    print(f"{'Service':<20} {'Test Case':<25} {'Concurrency':<12} {'Total Time (s)':<15} {'Throughput (RPS)':<15}")
    print(f"{'-' * 100}")
    
    for r in results:
        if "error" not in r:
            print(f"{r['service']:<20} {r['test_case']:<25} {r['concurrency']:<12} {r['total_time_s']:<15.2f} {r['throughput_rps']:<15.1f}")

def main():
    """Main benchmarking function."""
    print("=" * 100)
    print("JavaScript Code Execution Service - Performance Benchmark".center(100))
    print("=" * 100)
    
    # Check health of all services
    print("\nChecking service health...")
    healthy_services = {}
    for name, url in SERVICES.items():
        if check_health(name, url):
            healthy_services[name] = url
    
    if not healthy_services:
        print("\n✗ No services are available. Please start services with docker-compose up")
        return
    
    print(f"\n✓ Found {len(healthy_services)} healthy service(s)")
    
    # Run sequential benchmarks
    sequential_results = []
    for test_case in TEST_CASES:
        for name, url in healthy_services.items():
            result = run_benchmark(name, url, test_case, num_requests=50)
            if "error" not in result:
                sequential_results.append(result)
    
    # Print sequential results
    print_results_table(sequential_results, "Sequential Benchmark Results (50 requests per test)")
    
    # Run concurrent benchmarks
    concurrent_results = []
    for test_case in TEST_CASES[:2]:  # Only first 2 test cases for concurrent
        for name, url in healthy_services.items():
            result = run_concurrent_benchmark(name, url, test_case, num_requests=100, concurrency=10)
            if "error" not in result:
                concurrent_results.append(result)
    
    # Print concurrent results
    print_concurrent_results(concurrent_results, "Concurrent Benchmark Results (100 requests, 10 concurrent)")
    
    # Summary comparison
    print(f"\n{'=' * 100}")
    print("Summary".center(100))
    print(f"{'=' * 100}")
    
    # Group by service
    for service_name in healthy_services.keys():
        service_results = [r for r in sequential_results if r["service"] == service_name]
        if service_results:
            avg_rps = statistics.mean([r["requests_per_second"] for r in service_results])
            avg_latency = statistics.mean([r["mean_ms"] for r in service_results])
            print(f"\n{service_name}:")
            print(f"  Average RPS: {avg_rps:.1f}")
            print(f"  Average Latency: {avg_latency:.2f} ms")
    
    print(f"\n{'=' * 100}")
    print("Benchmark completed!".center(100))
    print(f"{'=' * 100}\n")

if __name__ == "__main__":
    main()
