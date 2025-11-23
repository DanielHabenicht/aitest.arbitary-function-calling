"""
Secure JavaScript code execution service using FastAPI and PyMiniRacer.
"""
import json
import asyncio
import base64
from typing import Dict, Any, Optional, List
from contextlib import asynccontextmanager
import threading

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
    Create a V8 context with INPUTS and httpRequest support.
    
    Args:
        inputs: The inputs to inject
        http_results: Optional cached HTTP results for immediate return
    
    Returns:
        Configured MiniRacer context
    """
    ctx = MiniRacer()
    
    # Safely inject INPUTS by passing it directly to the context
    ctx.eval("var INPUTS = " + json.dumps(inputs))
    
    if http_results is None:
        # Setup to collect HTTP requests
        ctx.eval("""
            var __httpRequests = [];
            function httpRequest(url, options) {
                __httpRequests.push({ url: url, options: options || {} });
                return undefined;
            }
            // Alias httpGet for backwards compatibility
            function httpGet(url, options) {
                return httpRequest(url, options);
            }
        """)
    else:
        # Return cached results
        ctx.eval("var __httpResults = " + json.dumps(http_results))
        ctx.eval("""
            function httpRequest(url, options) {
                // Match Python's key format with sorted keys
                var key = JSON.stringify({ 'options': options || {}, 'url': url });
                return __httpResults[key];
            }
            // Alias httpGet for backwards compatibility
            function httpGet(url, options) {
                return httpRequest(url, options);
            }
        """)
    
    return ctx


@app.post("/execute", response_model=ExecuteResponse, responses={400: {"model": ErrorResponse}, 500: {"model": ErrorResponse}})
async def execute_code(request: ExecuteRequest):
    """
    Execute user-provided JavaScript code in a secure V8 sandbox.
    
    The code can use:
    - INPUTS: Global variable containing the inputs
    - httpRequest(url, options): Function to make HTTP requests (supports all HTTP verbs)
    
    Optimized execution: Collects HTTP requests on first pass, executes them, then re-runs with results.
    """
    if not request.code or not isinstance(request.code, str):
        raise HTTPException(status_code=400, detail="Invalid code parameter")
    
    try:
        # Create context to collect HTTP requests
        ctx = create_context_with_http_support(request.inputs)
        
        try:
            # Try to execute - may fail if code depends on HTTP results
            result = ctx.eval(request.code)
            
            # Check if any HTTP requests were made
            http_requests_json = ctx.eval("JSON.stringify(__httpRequests)")
            http_requests = json.loads(http_requests_json)
            
            # If no HTTP requests, return result immediately
            if not http_requests:
                # Handle JSObject by stringifying via eval
                if hasattr(result, '__class__') and 'JSObject' in str(type(result)):
                    escaped_code = request.code.replace('`', '\\`').replace('$', '\\$')
                    result_json = ctx.eval(f"JSON.stringify(eval(`{escaped_code}`))")
                    result = json.loads(result_json) if result_json else result
                return ExecuteResponse(result=result)
        except Exception:
            # Code likely depends on HTTP results, that's okay
            http_requests_json = ctx.eval("JSON.stringify(__httpRequests)")
            http_requests = json.loads(http_requests_json)
            
            if not http_requests:
                # Real error - no HTTP requests and execution failed
                raise
        finally:
            del ctx
        
        # Execute all discovered HTTP requests in parallel
        tasks = [
            perform_fetch(req['url'], req.get('options'))
            for req in http_requests
        ]
        results = await asyncio.gather(*tasks)
        
        # Create results map (use compact JSON without spaces to match JavaScript's JSON.stringify)
        http_results = {}
        for req, result in zip(http_requests, results):
            key = json.dumps({'url': req['url'], 'options': req.get('options', {})}, sort_keys=True, separators=(',', ':'))
            http_results[key] = result
        
        # Execute with cached HTTP results
        ctx2 = create_context_with_http_support(request.inputs, http_results)
        
        try:
            # Execute code and convert to Python
            result = ctx2.eval(request.code)
            
            # Handle JSObject by stringifying via eval
            if hasattr(result, '__class__') and 'JSObject' in str(type(result)):
                # Use eval with template literal to handle both expressions and statements
                escaped_code = request.code.replace('`', '\\`').replace('$', '\\$')
                result_json = ctx2.eval(f"JSON.stringify(eval(`{escaped_code}`))")
                result = json.loads(result_json) if result_json else result
            
            return ExecuteResponse(result=result)
        finally:
            del ctx2
    
    except HTTPException:
        raise
    except Exception as e:
        # Log the full error for debugging
        import logging
        logging.error(f"Code execution error: {type(e).__name__}: {str(e)}", exc_info=True)
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
