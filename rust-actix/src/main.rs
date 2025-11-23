use actix_web::{web, App, HttpResponse, HttpServer, Result};
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

// Execute JavaScript code with QuickJS
async fn execute_js_with_quickjs(
    code: &str,
    inputs: &HashMap<String, Value>,
    http_results: Option<&HashMap<String, HttpResult>>,
) -> std::result::Result<Value, String> {
    let runtime = AsyncRuntime::new().map_err(|e| format!("Runtime error: {}", e))?;
    let context = AsyncContext::full(&runtime).await.map_err(|e| format!("Context error: {}", e))?;
    
    context.with(|ctx| {
        // Inject INPUTS object
        let inputs_json = serde_json::to_string(inputs).map_err(|e| e.to_string())?;
        ctx.eval::<(), _>(format!("var INPUTS = {};", inputs_json))
            .map_err(|e| format!("INPUTS injection error: {}", e))?;
        
        // Track HTTP requests if no results provided
        if http_results.is_none() {
            // First pass - set up httpGet to collect requests
            let setup_code = r#"
                var __httpRequests = [];
                function httpGet(url, options) {
                    __httpRequests.push({ url: url, options: options || {} });
                    return undefined;
                }
            "#;
            ctx.eval::<(), _>(setup_code)
                .map_err(|e| format!("Setup error: {}", e))?;
        } else {
            // Second pass - set up httpGet with actual results
            let results_map: HashMap<String, Value> = http_results
                .unwrap()
                .iter()
                .map(|(k, v)| {
                    let val = serde_json::json!({
                        "ok": v.ok,
                        "status": v.status,
                        "statusText": v.status_text,
                        "headers": v.headers,
                        "data": v.data,
                    });
                    (k.clone(), val)
                })
                .collect();
            
            let results_json = serde_json::to_string(&results_map).map_err(|e| e.to_string())?;
            ctx.eval::<(), _>(format!("var __httpResults = {};", results_json))
                .map_err(|e| format!("Results injection error: {}", e))?;
            
            let setup_code = r#"
                function httpGet(url, options) {
                    var key = JSON.stringify({ url: url, options: options || {} });
                    return __httpResults[key];
                }
            "#;
            ctx.eval::<(), _>(setup_code)
                .map_err(|e| format!("Setup error: {}", e))?;
        }
        
        // Execute the user code
        let result: rquickjs::Value = ctx.eval(code)
            .map_err(|e| format!("Execution error: {}", e))?;
        
        // Store result in a variable and stringify it
        let globals = ctx.globals();
        globals.set("__result", result).map_err(|e| format!("Set result error: {}", e))?;
        
        // Convert result to JSON string
        let json_str: String = ctx.eval("JSON.stringify(__result)")
            .map_err(|e| format!("JSON stringify error: {}", e))?;
        
        serde_json::from_str(&json_str).map_err(|e| e.to_string())
    }).await
}

// Extract HTTP requests from the first pass
async fn extract_http_requests(code: &str, inputs: &HashMap<String, Value>) -> std::result::Result<Vec<(String, Option<HashMap<String, Value>>)>, String> {
    let runtime = AsyncRuntime::new().map_err(|e| format!("Runtime error: {}", e))?;
    let context = AsyncContext::full(&runtime).await.map_err(|e| format!("Context error: {}", e))?;
    
    context.with(|ctx| {
        // Inject INPUTS
        let inputs_json = serde_json::to_string(inputs).map_err(|e| e.to_string())?;
        ctx.eval::<(), _>(format!("var INPUTS = {};", inputs_json))
            .map_err(|e| format!("INPUTS injection error: {}", e))?;
        
        // Set up request collection
        let setup_code = r#"
            var __httpRequests = [];
            function httpGet(url, options) {
                __httpRequests.push({ url: url, options: options || {} });
                return undefined;
            }
        "#;
        ctx.eval::<(), _>(setup_code)
            .map_err(|e| format!("Setup error: {}", e))?;
        
        // Execute code (may fail, that's ok)
        let _ = ctx.eval::<(), _>(code);
        
        // Extract requests
        let requests_json: String = ctx.eval("JSON.stringify(__httpRequests)")
            .map_err(|e| format!("Extract error: {}", e))?;
        
        let requests: Vec<Value> = serde_json::from_str(&requests_json)
            .map_err(|e| e.to_string())?;
        
        let mut result = Vec::new();
        for item in requests {
            if let Value::Object(obj) = item {
                let url = obj.get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let options = obj.get("options")
                    .and_then(|v| {
                        if let Value::Object(o) = v {
                            Some(o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        } else {
                            None
                        }
                    });
                if !url.is_empty() {
                    result.push((url, options));
                }
            }
        }
        
        Ok(result)
    }).await
}

async fn execute_handler(req: web::Json<ExecuteRequest>) -> Result<HttpResponse> {
    if req.code.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid code parameter".to_string(),
            message: "Code cannot be empty".to_string(),
        }));
    }
    
    // First pass: try to execute and collect HTTP requests
    let http_requests = match extract_http_requests(&req.code, &req.inputs).await {
        Ok(requests) => requests,
        Err(_) => {
            // If extraction fails, try direct execution (no HTTP calls)
            match execute_js_with_quickjs(&req.code, &req.inputs, None).await {
                Ok(result) => return Ok(HttpResponse::Ok().json(ExecuteResponse { result })),
                Err(e) => return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Execution failed".to_string(),
                    message: e,
                })),
            }
        }
    };
    
    // If no HTTP requests, execute directly
    if http_requests.is_empty() {
        match execute_js_with_quickjs(&req.code, &req.inputs, None).await {
            Ok(result) => return Ok(HttpResponse::Ok().json(ExecuteResponse { result })),
            Err(e) => return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Execution failed".to_string(),
                message: e,
            })),
        }
    }
    
    // Execute HTTP requests
    let mut http_results = HashMap::new();
    for (url, options) in http_requests {
        let result = perform_fetch(url.clone(), options.clone()).await;
        let key = serde_json::json!({
            "url": url,
            "options": options.unwrap_or_default()
        }).to_string();
        http_results.insert(key, result);
    }
    
    // Second pass: execute with HTTP results
    match execute_js_with_quickjs(&req.code, &req.inputs, Some(&http_results)).await {
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
