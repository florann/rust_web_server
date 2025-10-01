use std::{collections::VecDeque, sync::{mpsc, Arc, Mutex, OnceLock}};

use once_cell::sync::Lazy;

mod models;

//Global configuration variables
static MAX_UDP_PACKET_SIZE: usize = 50000;

//Global usable variables
static CLIENT_NUMBER_SENDER: OnceLock<Mutex<mpsc::Sender<usize>>> = OnceLock::new();
static CLIENT_NUMBER_RECEIVER: OnceLock<Mutex<mpsc::Receiver<usize>>> = OnceLock::new();
static GLOBAL_QUEUE: Lazy<Arc<Mutex<VecDeque<Vec<u8>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
