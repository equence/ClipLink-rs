pub mod app_state;
pub mod clipboard;
pub mod commands;
pub mod connection;
pub mod discovery;
pub mod protocol;
pub mod relay;

use clipboard::NativeClipboard;
use commands::{AppStatusDto, CommandRuntime};
use connection::ConnectionManager;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::Mutex;
use uuid::Uuid;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

type SharedRuntime = Mutex<CommandRuntime<NativeClipboard>>;

#[tauri::command]
async fn get_status(runtime: tauri::State<'_, SharedRuntime>) -> Result<AppStatusDto, String> {
    Ok(runtime.lock().await.status())
}

#[tauri::command]
async fn set_auto_write_remote_text(
    runtime: tauri::State<'_, SharedRuntime>,
    enabled: bool,
) -> Result<AppStatusDto, String> {
    Ok(runtime.lock().await.set_auto_write_remote_text(enabled))
}

#[tauri::command]
async fn start_relay(
    runtime: tauri::State<'_, SharedRuntime>,
    bind: String,
) -> Result<String, String> {
    let bind = bind
        .parse::<SocketAddr>()
        .map_err(|error| error.to_string())?;
    let local_addr = runtime
        .lock()
        .await
        .start_relay(bind)
        .await
        .map_err(|error| error.to_string())?;
    Ok(local_addr.to_string())
}

#[tauri::command]
async fn connect_relay(
    runtime: tauri::State<'_, SharedRuntime>,
    address: String,
) -> Result<AppStatusDto, String> {
    let address = address
        .parse::<SocketAddr>()
        .map_err(|error| error.to_string())?;
    runtime
        .lock()
        .await
        .connect_relay(address, 5, Duration::from_millis(300))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn send_text(runtime: tauri::State<'_, SharedRuntime>, text: String) -> Result<(), String> {
    runtime
        .lock()
        .await
        .send_text(text)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn copy_cached_image(
    runtime: tauri::State<'_, SharedRuntime>,
    image_id: String,
) -> Result<AppStatusDto, String> {
    let image_id = image_id
        .parse::<Uuid>()
        .map_err(|error| error.to_string())?;
    runtime
        .lock()
        .await
        .copy_cached_image(image_id)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let native_clipboard =
        NativeClipboard::new().expect("error while initializing native clipboard access");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(CommandRuntime::new(
            native_clipboard,
            ConnectionManager::new(),
        )))
        .invoke_handler(tauri::generate_handler![
            greet,
            get_status,
            set_auto_write_remote_text,
            start_relay,
            connect_relay,
            send_text,
            copy_cached_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
