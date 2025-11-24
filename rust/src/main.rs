use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use rquickjs::{AsyncContext, AsyncRuntime};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ExecuteRequest {
    code: String,
    inputs: HashMap<String, Value>,
}

#[derive(Serialize)]
struct ExecuteResponse {
    result: Value,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Clone)]
struct HttpResult {
    ok: bool,
    status: u16,
    status_text: String,
    headers: HashMap<String, String>,
    data: Value,
}

impl Serialize for HttpResult {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("HttpResult", 5)?;
        state.serialize_field("ok", &self.ok)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("statusText", &self.status_text)?;
        state.serialize_field("headers", &self.headers)?;
        state.serialize_field("data", &self.data)?;
        state.end()
    }
}

// Helper function to evaluate code and return JSON stringified result
fn evaluate_and_stringify(ctx: &rquickjs::Ctx, code: &str) -> rquickjs::Result<String> {
    let result: rquickjs::Value = ctx.eval(code)?;
    ctx.globals().set("__result", result)?;
    ctx.eval::<String, _>("JSON.stringify(__result)")
}

async fn perform_fetch(url: String, options: Option<HashMap<String, Value>>) -> HttpResult {
    let client = reqwest::Client::new();
    
    let method = options
        .as_ref()
        .and_then(|o| o.get("method"))
        .and_then(|m| m.as_str())
        .unwrap_or("GET");
    
    let headers_map: HashMap<String, String> = options
        .as_ref()
        .and_then(|o| o.get("headers"))
        .and_then(|h| serde_json::from_value(h.clone()).ok())
        .unwrap_or_default();
    
    let body = options
        .as_ref()
        .and_then(|o| o.get("body").cloned());
    
    let mut request = match method {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => client.get(&url),
    };
    
    for (key, value) in headers_map {
        request = request.header(&key, &value);
    }
    
    if let Some(b) = body {
        if let Some(body_str) = b.as_str() {
            request = request.body(body_str.to_string());
        }
    }
    
    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let status_text = response.status().canonical_reason().unwrap_or("").to_string();
            let ok = response.status().is_success();
            
            let mut headers = HashMap::new();
            for (key, value) in response.headers() {
                headers.insert(
                    key.to_string(),
                    value.to_str().unwrap_or("").to_string(),
                );
            }
            
            let data = if let Ok(json) = response.json::<Value>().await {
                json
            } else {
                Value::String("".to_string())
            };
            
            HttpResult {
                ok,
                status,
                status_text,
                headers,
                data,
            }
        }
        Err(e) => HttpResult {
            ok: false,
            status: 0,
            status_text: "Error".to_string(),
            headers: HashMap::new(),
            data: Value::String(format!("Fetch failed: {}", e)),
        },
    }
}

// Execute JavaScript code with QuickJS - two-pass approach for synchronous-looking async code
async fn execute_js_with_quickjs(
    code: &str,
    inputs: &HashMap<String, Value>,
) -> std::result::Result<Value, String> {
    // FIRST PASS: Collect HTTP requests
    let runtime = AsyncRuntime::new().map_err(|e| format!("Runtime error: {}", e))?;
    let context = AsyncContext::full(&runtime).await.map_err(|e| format!("Context error: {}", e))?;
    
    // Inject INPUTS object
    let inputs_json = serde_json::to_string(inputs).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        ctx.eval::<(), _>(format!("var INPUTS = {};", inputs_json))
            .map_err(|e| format!("INPUTS injection error: {}", e))
    }).await?;
    
    // Setup to collect HTTP requests (httpGet returns undefined and records the request)
    context.with(|ctx| {
        ctx.eval::<(), _>(r#"
            var __httpRequests = [];
            function httpRequest(url, options) {
                __httpRequests.push({ url: url, options: options || {} });
                return undefined;
            }
            function httpGet(url, options) {
                return httpRequest(url, options);
            }
        "#).map_err(|e| format!("Failed to setup request collection: {:?}", e))
    }).await?;
    
    // Try to execute the code (may fail if it depends on HTTP results, that's okay)
    let http_requests_json = context.with(|ctx| {
        // Try to evaluate code - silently ignore errors as they're expected when code depends on HTTP results
        // Only genuine errors (like syntax errors) would fail before reaching HTTP calls
        if let Err(e) = ctx.eval::<rquickjs::Value, _>(code) {
            // Log for debugging but don't fail - this is expected if code depends on HTTP results
            eprintln!("First pass evaluation failed (expected if code uses HTTP results): {:?}", e);
        }
        
        // Get collected requests
        ctx.eval::<String, _>("JSON.stringify(__httpRequests)")
            .map_err(|e| format!("Failed to get HTTP requests: {:?}", e))
    }).await?;
    
    #[derive(Deserialize)]
    struct HttpRequest {
        url: String,
        options: HashMap<String, Value>,
    }
    
    let http_requests: Vec<HttpRequest> = serde_json::from_str(&http_requests_json)
        .map_err(|e| format!("Failed to parse requests: {}", e))?;
    
    // If no HTTP requests, return the result from first pass
    if http_requests.is_empty() {
        let result_json = context.with(|ctx| {
            evaluate_and_stringify(&ctx, code)
                .map_err(|e| format!("Evaluation error: {:?}", e))
        }).await?;
        
        return serde_json::from_str(&result_json).map_err(|e| e.to_string());
    }
    
    // Execute all HTTP requests in parallel
    let mut tasks = Vec::new();
    for req in &http_requests {
        let url = req.url.clone();
        let options = if req.options.is_empty() {
            None
        } else {
            Some(req.options.clone())
        };
        tasks.push(perform_fetch(url, options));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // Build results map using compact JSON (matching Python's approach)
    // Keys are ordered alphabetically: options, url
    let mut http_results = HashMap::new();
    for (req, result) in http_requests.iter().zip(results.iter()) {
        let key = serde_json::json!({
            "options": req.options,
            "url": req.url,
        });
        let key_str = serde_json::to_string(&key)
            .map_err(|e| format!("Failed to serialize request key: {}", e))?;
        let result_value = serde_json::json!({
            "ok": result.ok,
            "status": result.status,
            "statusText": result.status_text,
            "headers": result.headers,
            "data": result.data,
        });
        http_results.insert(key_str, result_value);
    }
    
    // SECOND PASS: Execute with cached results
    let runtime2 = AsyncRuntime::new().map_err(|e| format!("Runtime error: {}", e))?;
    let context2 = AsyncContext::full(&runtime2).await.map_err(|e| format!("Context error: {}", e))?;
    
    // Inject INPUTS object
    context2.with(|ctx| {
        ctx.eval::<(), _>(format!("var INPUTS = {};", inputs_json))
            .map_err(|e| format!("INPUTS injection error: {}", e))
    }).await?;
    
    // Setup httpGet to return cached results
    let results_json = serde_json::to_string(&http_results).map_err(|e| e.to_string())?;
    context2.with(|ctx| {
        ctx.eval::<(), _>(format!("var __httpResults = {};", results_json))
            .map_err(|e| format!("Failed to inject results: {:?}", e))?;
        
        ctx.eval::<(), _>(r#"
            function httpRequest(url, options) {
                // Key format matches Python's: alphabetically sorted
                var key = JSON.stringify({ options: options || {}, url: url });
                return __httpResults[key];
            }
            function httpGet(url, options) {
                return httpRequest(url, options);
            }
        "#).map_err(|e| format!("Failed to setup result lookup: {:?}", e))
    }).await?;
    
    // Execute the code and return the result
    let result_json = context2.with(|ctx| {
        evaluate_and_stringify(&ctx, code)
            .map_err(|e| format!("Evaluation error: {:?}", e))
    }).await?;
    
    serde_json::from_str(&result_json).map_err(|e| e.to_string())
}

async fn execute_handler(Json(req): Json<ExecuteRequest>) -> Response {
    if req.code.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid code parameter".to_string(),
                message: "Code cannot be empty".to_string(),
            }),
        ).into_response();
    }
    
    // Single-pass execution with async httpRequest function
    match execute_js_with_quickjs(&req.code, &req.inputs).await {
        Ok(result) => (StatusCode::OK, Json(ExecuteResponse { result })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Execution failed".to_string(),
                message: e,
            }),
        ).into_response(),
    }
}

async fn health_handler() -> Response {
    (StatusCode::OK, Json(HealthResponse {
        status: "ok".to_string(),
    })).into_response()
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    tracing::info!("Starting server on 0.0.0.0:{}", port);
    
    // Build our application with routes
    let app = Router::new()
        .route("/execute", post(execute_handler))
        .route("/health", get(health_handler));
    
    // Run the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    
    tracing::info!("Server listening on {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app).await.unwrap();
}
