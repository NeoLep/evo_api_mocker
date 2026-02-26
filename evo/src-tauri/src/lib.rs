mod server;
use server::{MockApi, AppState, ServerConfig};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use tauri::{State, Manager, AppHandle};

const DATA_FILE_NAME: &str = "mocks.json";
const DB_CONFIG_FILE_NAME: &str = "db_connections.json";
const SERVER_CONFIG_FILE_NAME: &str = "server_config.json";

fn get_data_path(app_handle: &AppHandle) -> Option<PathBuf> {
    app_handle.path().app_data_dir().ok().map(|p| p.join(DATA_FILE_NAME))
}

fn save_mocks(app_handle: &AppHandle, mocks: &HashMap<String, MockApi>) -> Result<(), String> {
    if let Some(path) = get_data_path(app_handle) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(mocks).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_mocks(app_handle: &AppHandle) -> HashMap<String, MockApi> {
    if let Some(path) = get_data_path(app_handle) {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(mocks) = serde_json::from_str(&content) {
                    return mocks;
                }
            }
        }
    }
    HashMap::new()
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct DbConfig {
    name: String,
    url: String,
}

fn get_db_config_path(app_handle: &AppHandle) -> Option<PathBuf> {
    app_handle.path().app_data_dir().ok().map(|p| p.join(DB_CONFIG_FILE_NAME))
}

fn save_db_configs(app_handle: &AppHandle, configs: &Vec<DbConfig>) -> Result<(), String> {
    if let Some(path) = get_db_config_path(app_handle) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(configs).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_db_configs(app_handle: &AppHandle) -> Vec<DbConfig> {
    if let Some(path) = get_db_config_path(app_handle) {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(configs) = serde_json::from_str(&content) {
                    return configs;
                }
            }
        }
    }
    Vec::new()
}

fn get_server_config_path(app_handle: &AppHandle) -> Option<PathBuf> {
    app_handle.path().app_data_dir().ok().map(|p| p.join(SERVER_CONFIG_FILE_NAME))
}

fn save_server_config(app_handle: &AppHandle, config: &ServerConfig) -> Result<(), String> {
    if let Some(path) = get_server_config_path(app_handle) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_server_config(app_handle: &AppHandle) -> ServerConfig {
    if let Some(path) = get_server_config_path(app_handle) {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
    }
    ServerConfig::default()
}

#[tauri::command]
async fn get_server_config(state: State<'_, AppState>) -> Result<ServerConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
async fn update_server_config(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    config: ServerConfig,
) -> Result<(), String> {
    // Update state
    {
        let mut state_config = state.config.lock().map_err(|e| e.to_string())?;
        *state_config = config.clone();
    }
    
    // Save to file
    save_server_config(&app_handle, &config)?;
    
    // Restart server logic handled in frontend or separate command?
    // Ideally we signal the server thread to restart.
    // For now, let's just emit an event or rely on the user to restart?
    // Actually, we can implement a restart mechanism using channels.
    
    app_handle.emit("server-config-changed", ()).map_err(|e| e.to_string())?;
    
    Ok(())
}

use sqlx::any::AnyPoolOptions;
use std::time::Duration;

#[tauri::command]
async fn add_db_connection(app_handle: AppHandle, state: State<'_, AppState>, name: String, url: String) -> Result<(), String> {
    // Validate connection first
    let _ = AnyPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&url)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    // If successful, create a lazy pool for storage
    // Use connect_lazy to be consistent with startup behavior
    let pool = AnyPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(30))
        .connect_lazy(&url)
        .map_err(|e| format!("Failed to create pool: {}", e))?;
    
    let mut connections = state.db_connections.lock().map_err(|e| e.to_string())?;
    connections.insert(name.clone(), pool);
    
    // Save config
    // We need to reconstruct the list from current connections + maybe a separate config storage?
    // Connection pool cannot be serialized. We need to store configs separately.
    // Let's load existing configs, update, and save.
    let mut configs = load_db_configs(&app_handle);
    // Remove existing if same name
    configs.retain(|c| c.name != name);
    configs.push(DbConfig { name, url });
    save_db_configs(&app_handle, &configs)?;
    
    Ok(())
}

#[tauri::command]
async fn remove_db_connection(app_handle: AppHandle, state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut connections = state.db_connections.lock().map_err(|e| e.to_string())?;
    connections.remove(&name);
    
    let mut configs = load_db_configs(&app_handle);
    configs.retain(|c| c.name != name);
    save_db_configs(&app_handle, &configs)?;
    
    Ok(())
}

#[tauri::command]
fn get_db_connections(app_handle: AppHandle) -> Result<Vec<DbConfig>, String> {
    Ok(load_db_configs(&app_handle))
}

#[tauri::command]
async fn test_db_connection(url: String) -> Result<String, String> {
    // Manually install SQLx drivers
    sqlx::any::install_default_drivers();
    
    match AnyPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&url)
        .await 
    {
        Ok(_) => Ok("Connection successful".to_string()),
        Err(e) => Err(format!("Connection failed: {}", e)),
    }
}

#[tauri::command]
fn add_mock_api(app_handle: AppHandle, state: State<'_, AppState>, path: String, method: String, response_body: String, status_code: u16, response_type: String) -> Result<(), String> {
    let mut mocks = state.mocks.lock().map_err(|e| e.to_string())?;
    let method = method.to_uppercase();
    // Ensure path starts with /
    let path = if path.starts_with('/') { path } else { format!("/{}", path) };
    let key = format!("{} {}", method, path);
    
    let mock = MockApi {
        id: key.clone(),
        path: path.clone(),
        method,
        response_body,
        status_code,
        response_type,
    };
    
    mocks.insert(key, mock);
    save_mocks(&app_handle, &mocks)?;
    Ok(())
}

#[tauri::command]
fn get_mock_apis(state: State<'_, AppState>) -> Result<Vec<MockApi>, String> {
    let mocks = state.mocks.lock().map_err(|e| e.to_string())?;
    Ok(mocks.values().cloned().collect())
}

#[tauri::command]
fn remove_mock_api(app_handle: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut mocks = state.mocks.lock().map_err(|e| e.to_string())?;
    mocks.remove(&id);
    save_mocks(&app_handle, &mocks)?;
    Ok(())
}

#[tauri::command]
fn update_mock_api(app_handle: AppHandle, state: State<'_, AppState>, id: String, path: String, method: String, response_body: String, status_code: u16, response_type: String) -> Result<(), String> {
    let mut mocks = state.mocks.lock().map_err(|e| e.to_string())?;
    
    // If ID (method + path) changed, we need to remove the old one
    // But since ID is the key, and user might change method/path, 
    // we effectively do a remove + add, but frontend will pass the 'old' ID.
    
    if mocks.contains_key(&id) {
        mocks.remove(&id);
    }
    
    let method = method.to_uppercase();
    // Ensure path starts with /
    let path = if path.starts_with('/') { path } else { format!("/{}", path) };
    let key = format!("{} {}", method, path);
    
    mocks.insert(key.clone(), MockApi {
        id: key,
        path,
        method,
        response_body,
        status_code,
        response_type,
    });
    save_mocks(&app_handle, &mocks)?;
    Ok(())
}

use tauri::Emitter; // For emit
use tokio::sync::broadcast;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mocks = Arc::new(Mutex::new(HashMap::new()));
    let db_connections = Arc::new(Mutex::new(HashMap::new()));
    // Initial config
    let config = Arc::new(Mutex::new(ServerConfig::default()));
    
    // Broadcast channel for server shutdown
    let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);
    
    let app_state = AppState { 
        mocks: mocks.clone(),
        db_connections: db_connections.clone(),
        config: config.clone(),
    };
    
    // We need to clone app_state to pass to the server task
    let server_state = app_state.clone();
    let shutdown_tx_clone = shutdown_tx.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .manage(shutdown_tx) // Manage the shutdown sender
        .invoke_handler(tauri::generate_handler![
            add_mock_api, 
            get_mock_apis, 
            remove_mock_api, 
            update_mock_api,
            add_db_connection,
            remove_db_connection,
            get_db_connections,
            test_db_connection,
            get_server_config,
            update_server_config,
            restart_server,
            stop_server,
            start_server_cmd
        ])
        .setup(move |app| {
            // Manually install SQLx drivers
            sqlx::any::install_default_drivers();
            
            // Load mocks from file
            let loaded_mocks = load_mocks(app.handle());
            if !loaded_mocks.is_empty() {
                let mut state_mocks = mocks.lock().unwrap();
                *state_mocks = loaded_mocks;
            }
            
            // Load server config
            let loaded_config = load_server_config(app.handle());
            {
                let mut state_config = config.lock().unwrap();
                *state_config = loaded_config.clone();
            }

            // Load DB connections
            let loaded_configs = load_db_configs(app.handle());
            if !loaded_configs.is_empty() {
                let db_conns = db_connections.clone();
                // Connect lazily
                tauri::async_runtime::spawn(async move {
                    for config in loaded_configs {
                        // Use connect_lazy to avoid blocking and errors on startup
                        let pool = AnyPoolOptions::new()
                            .max_connections(20)
                            .acquire_timeout(Duration::from_secs(30))
                            .connect_lazy(&config.url);
                        
                        if let Ok(pool) = pool {
                                if let Ok(mut conns) = db_conns.lock() {
                                    conns.insert(config.name, pool);
                                }
                        } else if let Err(e) = pool {
                            println!("Failed to create lazy pool for DB '{}': {}", config.name, e);
                        }
                    }
                });
            }

            // Start server if configured to run
            if loaded_config.running {
                let rx = shutdown_tx_clone.subscribe();
                tauri::async_runtime::spawn(async move {
                    server::start_server(server_state, rx).await;
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn stop_server(
    shutdown_tx: State<'_, broadcast::Sender<()>>,
) -> Result<(), String> {
    // Signal shutdown
    let _ = shutdown_tx.send(());
    Ok(())
}

#[tauri::command]
async fn start_server_cmd(
    state: State<'_, AppState>,
    shutdown_tx: State<'_, broadcast::Sender<()>>,
) -> Result<(), String> {
    // Check if running already? 
    // We can assume frontend manages state, or we can use a mutex flag.
    // For now, let's just ensure we kill any old one first
    let _ = shutdown_tx.send(());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let rx = shutdown_tx.subscribe();
    let server_state = (*state).clone();
    tauri::async_runtime::spawn(async move {
        server::start_server(server_state, rx).await;
    });
    
    Ok(())
}

#[tauri::command]
async fn restart_server(
    state: State<'_, AppState>,
    shutdown_tx: State<'_, broadcast::Sender<()>>,
) -> Result<(), String> {
    // 1. Signal shutdown
    let _ = shutdown_tx.send(());
    
    // 2. Wait a bit for port to be released
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 3. Check if we should start
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    
    if config.running {
        let rx = shutdown_tx.subscribe();
        let server_state = (*state).clone();
        tauri::async_runtime::spawn(async move {
            server::start_server(server_state, rx).await;
        });
    }
    
    Ok(())
}
