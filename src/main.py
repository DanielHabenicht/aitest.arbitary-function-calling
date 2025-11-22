"""
Secure JavaScript code execution service using FastAPI and PyMiniRacer.
"""
import json
import asyncio
from typing import Dict, Any, Optional, List
from contextlib import asynccontextmanager

import httpx
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from py_mini_racer import MiniRacer


class ExecuteRequest(BaseModel):
    """Request model for code execution."""
    code: str
    inputs: Dict[str, Any] = {}


class ExecuteResponse(BaseModel):
    """Response model for code execution."""
    result: Any


class ErrorResponse(BaseModel):
    """Error response model."""
    error: str
    message: str


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Lifespan context manager for the application."""
    # Startup
    app.state.http_client = httpx.AsyncClient(timeout=30.0)
    yield
    # Shutdown
    await app.state.http_client.aclose()


app = FastAPI(
    title="JavaScript Code Execution Service",
    description="Secure JavaScript code execution using V8 sandbox",
    version="1.0.0",
    lifespan=lifespan
)


async def perform_fetch(url: str, options: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """
    Perform HTTP request and return serialized result.
    
    Args:
        url: The URL to fetch
        options: Optional request options (method, headers, body)
    
    Returns:
        Dictionary with response data
    """
    if options is None:
        options = {}
    
    try:
        method = options.get('method', 'GET')
        headers = options.get('headers', {})
        body = options.get('body')
        
        response = await app.state.http_client.request(
            method=method,
            url=url,
            headers=headers,
            content=body
        )
        
        # Try to parse as JSON, otherwise return text
        try:
            data = response.json()
        except Exception:
            data = response.text
        
        return {
            'ok': response.is_success,
            'status': response.status_code,
            'statusText': response.reason_phrase,
            'headers': dict(response.headers),
            'data': data
        }
    except Exception as e:
        return {
            'ok': False,
            'status': 0,
            'statusText': 'Error',
            'headers': {},
            'error': str(e)
        }


def create_context_with_http_support(
    inputs: Dict[str, Any],
    http_results: Optional[Dict[str, Any]] = None
) -> MiniRacer:
    """
    Create a V8 context with INPUTS and httpGet support.
    
    Args:
        inputs: The inputs to inject
        http_results: Optional HTTP results cache
    
    Returns:
        Configured MiniRacer context
    """
    ctx = MiniRacer()
    
    # Inject INPUTS
    ctx.eval(f"var INPUTS = {json.dumps(inputs)};")
    
    # Track HTTP requests if no results provided
    if http_results is None:
        # First pass - collect HTTP requests
        ctx.eval("""
            var __httpRequests = [];
            function httpGet(url, options) {
                __httpRequests.push({ url: url, options: options || {} });
                return undefined;
            }
        """)
    else:
        # Second pass - return cached results
        ctx.eval(f"var __httpResults = {json.dumps(http_results)};")
        ctx.eval("""
            var __httpRequestIndex = 0;
            function httpGet(url, options) {
                var key = JSON.stringify({ url: url, options: options || {} });
                return __httpResults[key];
            }
        """)
    
    return ctx


@app.post("/execute", response_model=ExecuteResponse, responses={400: {"model": ErrorResponse}, 500: {"model": ErrorResponse}})
async def execute_code(request: ExecuteRequest):
    """
    Execute user-provided JavaScript code in a secure V8 sandbox.
    
    The code can use:
    - INPUTS: Global variable containing the inputs
    - httpGet(url, options): Function to make HTTP requests
    
    The execution uses a two-pass model:
    1. First pass discovers httpGet() calls
    2. HTTP requests are executed in parallel
    3. Second pass executes with cached HTTP results
    """
    if not request.code or not isinstance(request.code, str):
        raise HTTPException(status_code=400, detail="Invalid code parameter")
    
    try:
        # First pass: Execute code to discover HTTP requests
        ctx = create_context_with_http_support(request.inputs)
        
        try:
            result = ctx.eval(request.code)
            
            # Check if any HTTP requests were made
            http_requests_json = ctx.eval("JSON.stringify(__httpRequests)")
            http_requests = json.loads(http_requests_json)
            
            # If no HTTP requests, return result immediately
            if not http_requests:
                return ExecuteResponse(result=result)
        except Exception as e:
            # Expected to fail if code depends on HTTP results
            http_requests_json = ctx.eval("JSON.stringify(__httpRequests)")
            http_requests = json.loads(http_requests_json)
            
            # If no HTTP requests were discovered, this is a real error
            if not http_requests:
                raise
        finally:
            del ctx
        
        # Execute all discovered HTTP requests in parallel
        tasks = [
            perform_fetch(req['url'], req.get('options'))
            for req in http_requests
        ]
        results = await asyncio.gather(*tasks)
        
        # Create results map
        http_results = {}
        for req, result in zip(http_requests, results):
            key = json.dumps({'url': req['url'], 'options': req.get('options', {})})
            http_results[key] = result
        
        # Second pass: Execute with cached HTTP results
        ctx2 = create_context_with_http_support(request.inputs, http_results)
        
        try:
            result = ctx2.eval(request.code)
            return ExecuteResponse(result=result)
        finally:
            del ctx2
    
    except Exception as e:
        raise HTTPException(
            status_code=500,
            detail={"error": "Execution failed", "message": str(e)}
        )


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "ok"}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=3000)
