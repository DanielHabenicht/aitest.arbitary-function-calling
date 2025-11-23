use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use rquickjs::{AsyncContext, AsyncRuntime, async_with, function::{Func, Async}, Object};
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

// Execute JavaScript code with QuickJS - true single pass with async HTTP execution
async fn execute_js_with_quickjs(
    code: &str,
    inputs: &HashMap<String, Value>,
) -> std::result::Result<Value, String> {
    let runtime = AsyncRuntime::new().map_err(|e| format!("Runtime error: {}", e))?;
    let context = AsyncContext::full(&runtime).await.map_err(|e| format!("Context error: {}", e))?;
    
    // Inject INPUTS object
    let inputs_json = serde_json::to_string(inputs).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        ctx.eval::<(), _>(format!("var INPUTS = {};", inputs_json))
            .map_err(|e| format!("INPUTS injection error: {}", e))
    }).await?;
    
    // Register async httpRequest function using Func::from(Async(...))
    async_with!(context => |ctx| {
        // Define the async function that will be called from JavaScript
        async fn http_request_impl<'js>(url: String, options: Object<'js>) -> rquickjs::Result<String> {
            // Convert options object to JSON string
            let ctx = options.ctx().clone();
            let json_mod: Object = ctx.globals().get("JSON")?;
            let stringify: rquickjs::Function = json_mod.get("stringify")?;
            let options_json: String = stringify.call((options,))?;
            
            // Parse options from JSON string
            let opts: Option<HashMap<String, Value>> = serde_json::from_str(&options_json).ok();
            
            // Perform the HTTP request
            let result = perform_fetch(url, opts).await;
            
            // Return the result as JSON string
            Ok(serde_json::to_string(&serde_json::json!({
                "ok": result.ok,
                "status": result.status,
                "statusText": result.status_text,
                "headers": result.headers,
                "data": result.data,
            })).unwrap_or_else(|_| "{}".to_string()))
        }
        
        // Register the async function using Func::from(Async(...))
        ctx.globals().set("__httpRequestAsync", Func::from(Async(http_request_impl)))
            .map_err(|e| format!("Failed to set httpRequest: {:?}", e))?;
        
        // Create a JavaScript wrapper that parses the JSON result
        ctx.eval::<(), _>(r#"
            async function httpRequest(url, options) {
                const resultJson = await __httpRequestAsync(url, options || {});
                return JSON.parse(resultJson);
            }
        "#).map_err(|e| format!("Failed to create httpRequest wrapper: {:?}", e))?;
        
        Ok::<(), String>(())
    }).await?;
    
    // Execute the user code - evaluate directly as async code (like Node.js does)
    // The user's code should contain 'await' keywords where needed
    let code_owned = code.to_string();
    let result_json = async_with!(context => |ctx| {
        // Wrap in async IIFE to allow top-level await
        // For code with statements, find the last semicolon and wrap what comes after in return
        let trimmed = code_owned.trim();
        let wrapped_code = if let Some(last_semi) = trimmed.rfind(';') {
            // Has statements - split at last semicolon
            let statements = &trimmed[..=last_semi];
            let last_expr = trimmed[last_semi + 1..].trim();
            if last_expr.is_empty() {
                // Ends with semicolon, no expression to return
                format!("(async () => {{ {} }})()", statements)
            } else {
                // Return the last expression
                format!("(async () => {{ {} return ({}); }})()", statements, last_expr)
            }
        } else {
            // Single expression, wrap in return
            format!("(async () => {{ return ({}); }})()", trimmed)
        };
        
        // Evaluate and get the promise
        let promise: rquickjs::Promise = ctx.eval(wrapped_code.as_str())
            .map_err(|e| format!("Evaluation error: {:?}", e))?;
        
        // Await the promise to get the result
        let result = promise.into_future::<rquickjs::Value>().await
            .map_err(|e| format!("Promise resolution error: {:?}", e))?;
        
        // Store and stringify result
        ctx.globals().set("__result", result)
            .map_err(|e| format!("Set result error: {:?}", e))?;
        
        let json_str: String = ctx.eval("JSON.stringify(__result)")
            .map_err(|e| format!("JSON stringify error: {:?}", e))?;
        
        Ok::<String, String>(json_str)
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
