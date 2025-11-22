use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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

// Simplified JavaScript execution using basic string evaluation
// Note: In production, you'd want to use a proper V8 binding like rusty_v8
fn execute_js_simple(code: &str, inputs: &HashMap<String, Value>) -> std::result::Result<Value, String> {
    // This is a simplified implementation
    // For a real implementation, you would use rusty_v8 to create a V8 isolate
    // and properly execute JavaScript code
    
    // For now, we'll return a simple implementation note
    Ok(Value::String(format!("JavaScript execution would happen here with code: {} and inputs: {:?}", code, inputs)))
}

async fn execute_handler(req: web::Json<ExecuteRequest>) -> Result<HttpResponse> {
    if req.code.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid code parameter".to_string(),
            message: "Code cannot be empty".to_string(),
        }));
    }
    
    // Simplified execution - in a real implementation, this would use V8
    // First pass: discover HTTP calls (simplified - just look for httpGet patterns)
    let http_requests: Vec<(String, Option<HashMap<String, Value>>)> = vec![];
    
    // For demonstration, if code contains httpGet, we'll make a test call
    if req.code.contains("httpGet") {
        // In real implementation, we'd parse the JS to find actual URLs
        // For now, just execute the code without HTTP support
    }
    
    // Execute JavaScript (simplified)
    match execute_js_simple(&req.code, &req.inputs) {
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
