use axum::{
    extract::State,
    http::{Method, StatusCode, Uri, HeaderMap},
    response::{IntoResponse, Response, Json, Html},
    Router,
    body::{Body, to_bytes},
};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, VecDeque}, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH, Instant}};
use tower_http::cors::CorsLayer;
use boa_engine::{Context, Source};
use sqlx::{Pool, Any};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MockApi {
    pub id: String,
    pub path: String,
    pub method: String,
    pub response_body: String,
    pub status_code: u16,
    pub response_type: String, // "json", "html", "raw", "js"
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RequestLog {
    pub id: String,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub timestamp: u64,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    // Key format: "METHOD /path"
    pub mocks: Arc<Mutex<HashMap<String, MockApi>>>,
    // Key: Connection ID (name)
    pub db_connections: Arc<Mutex<HashMap<String, Pool<Any>>>>,
    pub config: Arc<Mutex<ServerConfig>>,
    // Request Logs (Max 100)
    pub logs: Arc<Mutex<VecDeque<RequestLog>>>,
    // App handle for emitting events
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String, // "0.0.0.0" or "127.0.0.1"
    pub running: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
            running: true,
        }
    }
}

use tokio::sync::broadcast;

pub async fn start_server(state: AppState, mut shutdown_rx: broadcast::Receiver<()>) {
    let (config_port, config_host) = {
        let config = state.config.lock().unwrap();
        (config.port, config.host.clone())
    };

    let addr = format!("{}:{}", config_host, config_port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            println!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };

    println!("Server listening on {}", listener.local_addr().unwrap());

    let app = Router::new()
        .fallback(handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_rx.recv().await.ok();
            println!("Server shutting down...");
        })
        .await
        .unwrap();
}

#[axum::debug_handler]
async fn handler(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: String,
) -> Response {
    let start_time = Instant::now();
    let request_body_clone = body.clone();
    
    // Process request
    let response = process_request(state.clone(), method.clone(), uri.clone(), headers, body).await;
    
    // Log request
    let duration = start_time.elapsed().as_millis() as u64;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    let status_code = response.status().as_u16();
    
    // Try to capture response body for logging
    // Note: This consumes the response body, so we need to reconstruct it.
    // For now, let's just log "Response body logged" placeholder or try to peek if possible.
    // Actually, we can read the bytes, store them, and create a new body.
    
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap_or_default();
    let response_body_str = String::from_utf8(bytes.to_vec()).ok();
    
    let log = RequestLog {
        id: uuid::Uuid::new_v4().to_string(),
        method: method.to_string(),
        path: uri.path().to_string(),
        status_code,
        duration_ms: duration,
        timestamp,
        request_body: Some(request_body_clone),
        response_body: response_body_str.clone(),
    };
    
    // Store log
    if let Ok(mut logs) = state.logs.lock() {
        logs.push_front(log.clone());
        if logs.len() > 100 {
            logs.pop_back();
        }
    }
    
    // Emit event
    if let Ok(handle_guard) = state.app_handle.lock() {
        if let Some(app_handle) = handle_guard.as_ref() {
             use tauri::Emitter;
             if let Err(e) = app_handle.emit("new-request-log", log.clone()) {
                 println!("Failed to emit log: {}", e);
             }
        }
    }
    
    // Reconstruct response
    let response = Response::from_parts(parts, Body::from(bytes));
    response
}

async fn process_request(
    state: AppState,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: String,
) -> Response {
    let path = uri.path();
    let key = format!("{} {}", method, path);
    
    // Check exact match first
    let mock_opt = {
        let mocks = state.mocks.lock().unwrap();
        // Try specific method first
        if let Some(mock) = mocks.get(&key) {
            Some(mock.clone())
        } else {
            // Try ANY method
            let any_key = format!("ANY {}", path);
            mocks.get(&any_key).cloned()
        }
    };

    if let Some(mock) = mock_opt {
        let response_body = mock.response_body.clone();
        let status = StatusCode::from_u16(mock.status_code).unwrap_or(StatusCode::OK);
        
        return match mock.response_type.as_str() {
            "json" => {
                 match serde_json::from_str::<serde_json::Value>(&response_body) {
                    Ok(json) => (status, Json(json)).into_response(),
                    Err(_) => (status, response_body).into_response(),
                 }
            },
            "html" => (status, Html(response_body)).into_response(),
            "js" => {
                // Execute JS code
                // Use spawn_blocking to run JS logic without blocking async runtime
                let body = body.clone();
                let method = method.to_string();
                let path = path.to_string();
                let headers_vec: Vec<(String, String)> = headers.iter().filter_map(|(k, v)| {
                    v.to_str().ok().map(|val| (k.to_string(), val.to_string()))
                }).collect();
                let db_connections = state.db_connections.clone();
                
                let result = tokio::task::spawn_blocking(move || {
                    let mut context = Context::default();
                    
                    // Manually map missing MySql Tiny (i8) type support in Any driver
                    // We can't easily change sqlx internals here.
                    // But we can catch the specific error and try to explain?
                    // No, the error happens inside fetch_all.
                    // The only way is to use a specific pool (MySqlPool) if we know it's MySQL.
                    // But we are using AnyPool for flexibility.
                    // Wait, sqlx 0.8 Any driver *should* support basic types.
                    // The error says "Any driver does not support MySql type ... Tiny".
                    // This implies AnyRow doesn't know how to map it.
                    // Workaround: CAST(column AS SIGNED) or CAST(column AS UNSIGNED) in SQL might promote it to standard int?
                    // OR we can try to patch the query execution? No.
                    
                    // Let's rely on user to cast in SQL for now if they hit this?
                    // "SELECT CAST(tiny_col AS SIGNED) FROM table"
                    // But that's bad UX.
                    
                    // Alternative: We can try to use `sqlx::query_as` with a struct? No, dynamic.
                    
                    // Actually, the previous fix `row.try_get::<i8>` was removing the decoding attempt.
                    // But the error "Any driver does not support..." comes from `fetch_all` or `try_get`?
                    // It likely comes from `row.columns()` iteration or when `AnyRow` is constructed?
                    // No, `fetch_all` returns `Vec<AnyRow>`.
                    // If `fetch_all` fails, it means `Any` driver failed to map the type definition from the DB.
                    // This is a known limitation in sqlx::Any for some MySql types.
                    
                    // For now, let's keep the `try_get` logic but maybe we need to wrap `fetch_all` in a way?
                    // If `fetch_all` fails with that specific error, we can't do much from Rust side easily without patching sqlx.
                    // BUT, wait. `sqlx::Any` *does* support bool for TINYINT(1).
                    // If it's TINYINT(>1), it might fail.
                    
                    // Let's assume the previous `try_get` removal of `i8` was correct for *decoding*,
                    // but if the error happens *before* decoding (during fetch), we are stuck.
                    // However, usually `fetch_all` succeeds and gives us `AnyRow`.
                    // The error `Any driver does not support...` usually happens when we try to `get` a value and the driver doesn't know how to convert the raw bytes to the requested type via AnyValue?
                    // OR it happens during `AnyRow` construction.
                    
                    // If it happens during `fetch_all`, we might be in trouble.
                    // Let's assume it happens during `fetch_all`.
                    // https://github.com/launchbadge/sqlx/issues/1441
                    // Seems `Any` has issues with some types.
                    
                    // Let's try to proceed. If `fetch_all` fails, we return the error string.
                    // Maybe we can suggest the user to use CAST.
                    
                    // Prepare request object
                    let mut headers_obj = boa_engine::object::ObjectInitializer::new(&mut context);
                    for (k, v) in headers_vec {
                        headers_obj.property(
                            boa_engine::JsString::from(k),
                            boa_engine::JsString::from(v),
                            boa_engine::property::Attribute::READONLY
                        );
                    }
                    let headers_js = headers_obj.build();

                    let request_obj = boa_engine::object::ObjectInitializer::new(&mut context)
                        .property(
                            boa_engine::JsString::from("headers"),
                            headers_js,
                            boa_engine::property::Attribute::READONLY
                        )
                        .property(
                            boa_engine::JsString::from("body"),
                            boa_engine::JsString::from(body),
                            boa_engine::property::Attribute::READONLY
                        )
                        .property(
                            boa_engine::JsString::from("method"),
                            boa_engine::JsString::from(method),
                            boa_engine::property::Attribute::READONLY
                        )
                        .property(
                            boa_engine::JsString::from("path"),
                            boa_engine::JsString::from(path),
                            boa_engine::property::Attribute::READONLY
                        )
                        .build();

                    if let Err(e) = context.register_global_property(
                        boa_engine::JsString::from("request"),
                        request_obj,
                        boa_engine::property::Attribute::READONLY
                    ) {
                         return (StatusCode::INTERNAL_SERVER_ERROR, format!("JS Error: {}", e)).into_response();
                    }

                    // Create a response object with setStatusCode method
                    let status_code_ref = Arc::new(Mutex::new(status));
                    let status_code_clone = status_code_ref.clone();
                    
                    use boa_engine::{JsResult, JsValue, NativeFunction, JsError};
                
                // --- Database Object ---
                    let db_connections_ref = db_connections.clone();
                    
                    let query_fn = unsafe {
                        let db_connections = db_connections_ref.clone();
                        NativeFunction::from_closure(move |_this, args, context| -> JsResult<JsValue> {
                            let conn_name = args.get(0).and_then(|v| v.as_string()).ok_or_else(|| JsError::from_opaque(JsValue::new(boa_engine::JsString::from("Missing connection name"))))?;
                            let sql = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| JsError::from_opaque(JsValue::new(boa_engine::JsString::from("Missing SQL"))))?;
                            let _params_val = args.get(2); // Optional params array
    
                            let conn_name_str = conn_name.to_std_string().unwrap();
                            let sql_str = sql.to_std_string().unwrap();
                            
                            // Extract params (skipped for now)
    
                            // Execute query in blocking thread
                            // Since we are already in spawn_blocking, we can use block_on locally?
                            // Or use a new runtime.
                            // To be safe and independent, creating a runtime is fine, but overhead.
                            // But let's keep it for now as it works if not blocking the main thread.
                            
                            let db_connections_inner = db_connections.clone();
                            let result: Result<Vec<serde_json::Value>, String> = {
                                 let rt = tokio::runtime::Builder::new_current_thread()
                                     .enable_all()
                                     .build()
                                     .unwrap();
                                 
                                 rt.block_on(async {
                                     // Clone the pool from the map to avoid holding the lock during query
                                     let pool = {
                                         let conns = db_connections_inner.lock().unwrap();
                                         conns.get(&conn_name_str).cloned()
                                     };
    
                                     if let Some(pool) = pool {
                                         println!("[DB] Executing query on '{}': {}", conn_name_str, sql_str);
                                         
                                         let rows = sqlx::query(&sql_str)
                                             .fetch_all(&pool)
                                             .await
                                             .map_err(|e| e.to_string())?;
                                         
                                         // Convert rows to JSON
                                         let mut json_rows = Vec::new();
                                         for row in rows {
                                             use sqlx::{Row, Column};
                                             let mut row_obj = serde_json::Map::new();
                                             for col in row.columns() {
                                                let name = col.name();
                                                let val_json = if let Ok(v) = row.try_get::<String, _>(name) {
                                                serde_json::Value::String(v)
                                            } else if let Ok(v) = row.try_get::<i64, _>(name) {
                                                serde_json::Value::Number(v.into())
                                            } else if let Ok(v) = row.try_get::<f64, _>(name) {
                                                serde_json::Number::from_f64(v).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null)
                                            } else if let Ok(v) = row.try_get::<bool, _>(name) {
                                                serde_json::Value::Bool(v)
                                            // Any driver doesn't support i8 directly, map to i16 or i32
                                            } else if let Ok(v) = row.try_get::<i16, _>(name) {
                                                serde_json::Value::Number(v.into())
                                            } else if let Ok(v) = row.try_get::<i32, _>(name) {
                                                serde_json::Value::Number(v.into())
                                            } else {
                                                serde_json::Value::Null
                                            };
                                                row_obj.insert(name.to_string(), val_json);
                                            }
                                             json_rows.push(serde_json::Value::Object(row_obj));
                                         }
                                         Ok(json_rows)
                                     } else {
                                         Err(format!("Connection '{}' not found", conn_name_str))
                                     }
                                 })
                            };
    
                            match result {
                                Ok(rows) => {
                                    let json_str = serde_json::to_string(&rows).unwrap();
                                    let json_obj = context.global_object().get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("JSON")), context).unwrap();
                                    let parse = json_obj.as_object().unwrap().get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("parse")), context).unwrap();
                                    let js_str = boa_engine::JsString::from(json_str);
                                    parse.as_callable().unwrap().call(&json_obj, &[JsValue::new(js_str)], context)
                                },
                                Err(e) => Err(JsError::from_opaque(JsValue::new(boa_engine::JsString::from(e))))
                            }
                        })
                    };
                    
                    let execute_fn = unsafe {
                        let db_connections = db_connections_ref.clone();
                        NativeFunction::from_closure(move |_this, args, _context| -> JsResult<JsValue> {
                             let conn_name = args.get(0).and_then(|v| v.as_string()).ok_or_else(|| JsError::from_opaque(JsValue::new(boa_engine::JsString::from("Missing connection name"))))?;
                             let sql = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| JsError::from_opaque(JsValue::new(boa_engine::JsString::from("Missing SQL"))))?;
                             
                             let conn_name_str = conn_name.to_std_string().unwrap();
                             let sql_str = sql.to_std_string().unwrap();
                             
                             let db_connections_inner = db_connections.clone();
                             let result: Result<u64, String> = {
                                 let rt = tokio::runtime::Builder::new_current_thread()
                                     .enable_all()
                                     .build()
                                     .unwrap();
                                 
                                 rt.block_on(async {
                                     let pool = {
                                         let conns = db_connections_inner.lock().unwrap();
                                         conns.get(&conn_name_str).cloned()
                                     };
    
                                     if let Some(pool) = pool {
                                         println!("[DB] Executing command on '{}': {}", conn_name_str, sql_str);
                                         let result = sqlx::query(&sql_str)
                                             .execute(&pool)
                                             .await
                                             .map_err(|e| e.to_string())?;
                                         Ok(result.rows_affected())
                                     } else {
                                         Err(format!("Connection '{}' not found", conn_name_str))
                                     }
                                 })
                             };
                             
                             match result {
                                 Ok(count) => Ok(JsValue::new(count as i32)), 
                                 Err(e) => Err(JsError::from_opaque(JsValue::new(boa_engine::JsString::from(e))))
                             }
                        })
                    };
    
                    let db_obj = boa_engine::object::ObjectInitializer::new(&mut context)
                        .function(query_fn, boa_engine::JsString::from("query"), 2)
                        .function(execute_fn, boa_engine::JsString::from("execute"), 2)
                        .build();
                    
                    if let Err(e) = context.register_global_property(
                        boa_engine::JsString::from("db"),
                        db_obj,
                        boa_engine::property::Attribute::READONLY
                    ) {
                         return (StatusCode::INTERNAL_SERVER_ERROR, format!("JS Error: {}", e)).into_response();
                    }
    
                    let set_status_code = unsafe {
                        NativeFunction::from_closure(move |_this, args, _ctx| -> JsResult<JsValue> {
                            if let Some(arg) = args.get(0) {
                                if let Some(code) = arg.as_number() {
                                    if let Ok(mut status) = status_code_clone.lock() {
                                        if let Ok(s) = StatusCode::from_u16(code as u16) {
                                            *status = s;
                                        }
                                    }
                                }
                            }
                            Ok(JsValue::undefined())
                        })
                    };
    
                    // --- Console Object ---
                    let console_log = unsafe {
                        NativeFunction::from_closure(move |_this, args, context| -> JsResult<JsValue> {
                            let mut output = String::new();
                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 {
                                    output.push(' ');
                                }
                                
                                if arg.is_string() {
                                    output.push_str(&arg.as_string().unwrap().to_std_string().unwrap());
                                } else {
                                    let json_key = boa_engine::property::PropertyKey::from(boa_engine::JsString::from("JSON"));
                                    let stringify_key = boa_engine::property::PropertyKey::from(boa_engine::JsString::from("stringify"));
                                    
                                    if let Ok(json_obj) = context.global_object().get(json_key, context) {
                                        if let Some(json_obj) = json_obj.as_object() {
                                             if let Ok(stringify) = json_obj.get(stringify_key, context) {
                                                 if let Ok(s) = stringify.as_callable().unwrap().call(&JsValue::from(json_obj.clone()), &[arg.clone()], context) {
                                                     if let Some(str_val) = s.as_string() {
                                                         if let Ok(utf8) = str_val.to_std_string() {
                                                             output.push_str(&utf8);
                                                             continue;
                                                         }
                                                     }
                                                 }
                                             }
                                        }
                                    }
                                    output.push_str(&format!("{:?}", arg));
                                }
                            }
                            println!("[JS Console] {}", output);
                            Ok(JsValue::undefined())
                        })
                    };
    
                    let console_obj = boa_engine::object::ObjectInitializer::new(&mut context)
                        .function(console_log, boa_engine::JsString::from("log"), 0)
                        .build();
    
                    if let Err(e) = context.register_global_property(
                        boa_engine::JsString::from("console"),
                        console_obj,
                        boa_engine::property::Attribute::READONLY
                    ) {
                         return (StatusCode::INTERNAL_SERVER_ERROR, format!("JS Error: {}", e)).into_response();
                    }
    
                    let response_obj = boa_engine::object::ObjectInitializer::new(&mut context)
                        .function(
                            set_status_code,
                            boa_engine::JsString::from("setStatusCode"),
                            1
                        )
                        .build();
    
                    if let Err(e) = context.register_global_property(
                        boa_engine::JsString::from("response"),
                        response_obj,
                        boa_engine::property::Attribute::READONLY
                    ) {
                         return (StatusCode::INTERNAL_SERVER_ERROR, format!("JS Error: {}", e)).into_response();
                    }
                    
                    let code = format!(
                        "
                        (function(request) {{
                            {}
                        }})(request);
                        ",
                        response_body
                    );
    
                    match context.eval(Source::from_bytes(code.as_bytes())) {
                        Ok(res) => {
                             let final_status = *status_code_ref.lock().unwrap();
    
                             if let Some(s) = res.as_string() {
                                 if let Ok(utf8) = s.to_std_string() {
                                     if let Ok(json) = serde_json::from_str::<serde_json::Value>(&utf8) {
                                         return (final_status, Json(json)).into_response();
                                     }
                                     return (final_status, utf8).into_response();
                                 }
                             }
                             if res.is_object() {
                                 let json_key = boa_engine::property::PropertyKey::from(boa_engine::JsString::from("JSON"));
                                 let stringify_key = boa_engine::property::PropertyKey::from(boa_engine::JsString::from("stringify"));
                                 
                                 let json_obj = context.global_object().get(json_key, &mut context).unwrap();
                                 let stringify = json_obj.as_object().unwrap().get(stringify_key, &mut context).unwrap();
                                 let res_clone = res.clone(); 
                                 if let Ok(s) = stringify.as_callable().unwrap().call(&json_obj, &[res], &mut context) {
                                     if let Some(str_val) = s.as_string() {
                                         if let Ok(utf8) = str_val.to_std_string() {
                                             return (final_status, utf8).into_response();
                                         }
                                     }
                                 }
                                 return (final_status, format!("{:?}", res_clone)).into_response();
                             }
                             
                             (final_status, format!("{:?}", res)).into_response()
                        },
                        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("JS Error: {}", e)).into_response()
                    }
                }).await.unwrap();
                
                return result;
            },
            "proxy" => {
                // If the user registered path "/api/v1/*", then `key` is "METHOD /api/v1/*"
                // But the actual request path is "/api/v1/users".
                // So the exact match `mocks.get(&key)` failed earlier.
                // However, if the user registered "/api/v1/users" with type proxy, we hit here.
                // In that case, we just forward to target + path suffix (empty)
                
                // mock.response_body is the target URL (e.g. http://localhost:8080/api/v1/users)
                let target_url = mock.response_body.clone();
                println!("[PROXY] {} => {}", path, target_url);
                
                // Forward request
                let client = reqwest::Client::new();
                let mut req_builder = client.request(method.clone(), &target_url);
                
                // Forward headers
                for (k, v) in headers.iter() {
                     if k != "host" {
                         req_builder = req_builder.header(k, v);
                     }
                }
                
                // Forward body
                req_builder = req_builder.body(body.clone());
                
                match req_builder.send().await {
                    Ok(res) => {
                        let status = res.status();
                        let mut response_builder = Response::builder().status(status);
                        
                        if let Some(headers_mut) = response_builder.headers_mut() {
                            for (k, v) in res.headers().iter() {
                                headers_mut.insert(k, v.clone());
                            }
                        }
                        
                        let bytes = res.bytes().await.unwrap_or_default();
                        return response_builder.body(Body::from(bytes)).unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response());
                    },
                    Err(e) => {
                        return (StatusCode::BAD_GATEWAY, format!("Proxy Error: {}", e)).into_response();
                    }
                }
            },
            "raw" => (status, response_body).into_response(),
            _ => (status, response_body).into_response(),
        };
    }
    
    // If exact match failed, try to find a proxy rule (wildcard match)
    // We iterate over all mocks that are of type "proxy" and have a wildcard path
    let proxy_match = {
        let mocks = state.mocks.lock().unwrap();
        mocks.values().find_map(|mock| {
            if mock.response_type == "proxy" && mock.path.ends_with('*') {
                if mock.method == "ANY" || mock.method == method.to_string() {
                    let prefix = &mock.path[..mock.path.len() - 1];
                    if path.starts_with(prefix) {
                        return Some((mock.response_body.clone(), prefix.len()));
                    }
                }
            }
            None
        })
    };

    if let Some((target_base, prefix_len)) = proxy_match {
        // Match found!
        // Construct target URL
        // mock.response_body is the target base URL, e.g. "http://localhost:8080"
        // We need to append the suffix
        let suffix = &path[prefix_len..];
        
        let target_base_trimmed = target_base.trim_end_matches('/');
        let suffix_trimmed = suffix.trim_start_matches('/');
        
        let target_url = if suffix_trimmed.is_empty() {
             target_base_trimmed.to_string()
        } else {
             format!("{}/{}", target_base_trimmed, suffix_trimmed)
        };
        
        println!("[PROXY] {} => {}", path, target_url);
        
        // Forward request
        let client = reqwest::Client::new();
        let mut req_builder = client.request(method.clone(), &target_url);
        
        // Forward headers
        for (k, v) in headers.iter() {
             // Skip host header to avoid issues
             if k != "host" {
                 req_builder = req_builder.header(k, v);
             }
        }
        
        // Forward body
        req_builder = req_builder.body(body.clone());
        
        match req_builder.send().await {
            Ok(res) => {
                let status = res.status();
                let mut response_builder = Response::builder().status(status);
                
                // Forward response headers
                if let Some(headers_mut) = response_builder.headers_mut() {
                    for (k, v) in res.headers().iter() {
                        headers_mut.insert(k, v.clone());
                    }
                }
                
                let bytes = res.bytes().await.unwrap_or_default();
                return response_builder.body(Body::from(bytes)).unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response());
            },
            Err(e) => {
                return (StatusCode::BAD_GATEWAY, format!("Proxy Error: {}", e)).into_response();
            }
        }
    }

    (StatusCode::NOT_FOUND, format!("Not Found: {}", key)).into_response()
}
