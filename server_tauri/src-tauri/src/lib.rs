use std::{collections::VecDeque, sync::{mpsc, Arc, Mutex, OnceLock}};
use windows_capture::{monitor::Monitor, settings::{ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings}}; 
use tauri::State;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use tauri::{Manager};

mod models;
use crate::models::structs::app_core::AppCore;

//Global usable variables
static MAX_UDP_PACKET_SIZE: usize = 50000;
static CLIENT_NUMBER_SENDER: OnceLock<Mutex<mpsc::Sender<usize>>> = OnceLock::new();
static CLIENT_NUMBER_RECEIVER: OnceLock<Mutex<mpsc::Receiver<usize>>> = OnceLock::new();
static GLOBAL_QUEUE: Lazy<Arc<Mutex<VecDeque<Vec<u8>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));


type SingletonType = Arc<RwLock<AppCore>>;


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn tag_capture_thread_state(app_core: SingletonType) {
    let mut guard = app_core.write().await;
    guard.tag_capture_thread_state();
}

#[tauri::command]
async fn get_capture_thread_state(app_core: SingletonType) -> Option<bool> {
    let guard = app_core.read().await;
    Some(guard.get_capture_thread_state())
}


#[tauri::command]
async fn run_capture_thread(app_core: State<'_, SingletonType>) -> Result<(), String> {

  let primary_monitor = Monitor::primary().expect("No primary monitor");
    let settings = Settings::new(
        // Item to capture
        primary_monitor,
        // Capture cursor settings
        CursorCaptureSettings::Default,
        // Draw border settings
        DrawBorderSettings::Default,
        // Secondary window settings, if you want to include secondary windows in the capture
        SecondaryWindowSettings::Default,
        // Minimum update interval, if you want to change the frame rate limit (default is 60 FPS or 16.67 ms)
        MinimumUpdateIntervalSettings::Default,
        // Dirty region settings,
        DirtyRegionSettings::Default,
        // The desired color format for the captured frame.
        ColorFormat::Rgba8,
        // Additional flags for the capture settings that will be passed to the user-defined `new` function.
        "".to_string(),
    );

    let guard = app_core.read().await;
    guard.new_capture_thread(&settings);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_core = Arc::new(RwLock::new(AppCore::new()));
            app.manage(app_core);

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_capture_thread])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
