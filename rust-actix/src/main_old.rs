use actix_web::{web, App, HttpResponse, HttpServer, Result};
use rquickjs::{AsyncContext, AsyncRuntime, function::Func};
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

// Execute JavaScript code with QuickJS using single-pass async approach
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
    
    // Create a JavaScript function that will call back to Rust for HTTP requests
    // We use a promise-based approach where JavaScript can await our Rust HTTP calls
    context.with(|ctx| {
        // Register a function that will be called from JavaScript
        let http_func = Func::from(|_ctx: rquickjs::Ctx, url: String, options_str: String| {
            // Parse options from JSON string
            let options: Option<HashMap<String, Value>> = if options_str.is_empty() || options_str == "{}" {
                None
            } else {
                serde_json::from_str(&options_str).ok()
            };
            
            // Create a promise that will be resolved with the HTTP result
            // Since we can't directly await in rquickjs, we return undefined and use a callback pattern
            // Actually, let's use a simpler synchronous approach with blocking
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    perform_fetch(url, options).await
                })
            });
            
            // Convert result to JSON string
            let result_json = serde_json::json!({
                "ok": result.ok,
                "status": result.status,
                "statusText": result.status_text,
                "headers": result.headers,
                "data": result.data,
            });
            
            serde_json::to_string(&result_json).unwrap_or_else(|_| "{}".to_string())
        });
        
        ctx.globals().set("__httpRequestSync", http_func)
            .map_err(|e| format!("Failed to set HTTP function: {}", e))?;
        
        // Create JavaScript wrapper
        ctx.eval::<(), _>(r#"
            function httpRequest(url, options) {
                options = options || {};
                const optionsJson = JSON.stringify(options);
                const resultJson = __httpRequestSync(url, optionsJson);
                return JSON.parse(resultJson);
            }
        "#).map_err(|e| format!("Failed to create httpRequest wrapper: {}", e))
    }).await?;
    
    // Execute the user code
    let result_value = context.with(|ctx| {
        // Execute code
        let result: rquickjs::Value = ctx.eval(code)
            .map_err(|e| format!("Execution error: {}", e))?;
        
        // Store result and stringify it
        let globals = ctx.globals();
        globals.set("__result", result).map_err(|e| format!("Set result error: {}", e))?;
        
        let json_str: String = ctx.eval("JSON.stringify(__result)")
            .map_err(|e| format!("JSON stringify error: {}", e))?;
        
        Ok::<String, String>(json_str)
    }).await?;
    
    serde_json::from_str(&result_value).map_err(|e| e.to_string())
}

async fn execute_handler(req: web::Json<ExecuteRequest>) -> Result<HttpResponse> {
    if req.code.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid code parameter".to_string(),
            message: "Code cannot be empty".to_string(),
        }));
    }
    
    // Single-pass execution with async httpRequest function
    match execute_js_with_quickjs(&req.code, &req.inputs).await {
        Ok(result) => Ok(HttpResponse::Ok().json(ExecuteResponse { result })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Execution failed".to_string(),
            message: e,
        })),
    }
}

async fn health_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    log::info!("Starting server on 0.0.0.0:{}", port);
    
    HttpServer::new(|| {
        App::new()
            .route("/execute", web::post().to(execute_handler))
            .route("/health", web::get().to(health_handler))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
