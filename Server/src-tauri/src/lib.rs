use std::{arch::x86_64::_CMP_FALSE_OQ, collections::VecDeque, net::{SocketAddr, UdpSocket}, sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc, Mutex, OnceLock}, thread::JoinHandle};
use arc_swap::ArcSwapAny;
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

struct ThreadSupervisor {
    capture_thread: Option<JoinHandle<()>>,
    capture_thread_should_stop: Arc<AtomicBool>,
    emit_thread: Option<JoinHandle<()>>,
    emit_thread_should_stop: Arc<AtomicBool>,
}


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn run_capture_thread(
    app_core: State<'_, SingletonType>,
    thread_supervisor: State<'_, Arc<Mutex<ThreadSupervisor>>>,
    clients: State<'_, Arc<arc_swap::ArcSwapAny<Arc<Vec<SocketAddr>>>>>,
    udp_socket: State<'_, Arc<UdpSocket>>
) -> Result<bool, String> {

  let primary_monitor = Monitor::primary().expect("No primary monitor");
    let settings: Settings<Arc<AtomicBool>, Monitor> = Settings::new(
        // Item to capture
        primary_monitor,
        // Capture cursor settings
        CursorCaptureSettings::Default,
        // Draw border settings
        DrawBorderSettings::Default,
        // Secondary window settings, if you want to include secondary wsindows in the capture
        SecondaryWindowSettings::Default,
        // Minimum update interval, if you want to change the frame rate limit (default is 60 FPS or 16.67 ms)
        MinimumUpdateIntervalSettings::Default,
        // Dirty region settings,
        DirtyRegionSettings::Default,
        // The desired color format for the captured frame.
        ColorFormat::Rgba8,
        // Additional flags for the capture settings that will be passed to the user-defined `new` function.
        thread_supervisor.lock().unwrap().capture_thread_should_stop.clone(),
    );

    // Get application
    let guard = app_core.read().await;
    
    let mut lock_supervisor = thread_supervisor.lock().unwrap();
    let capture_thread_handler = guard.new_capture_thread(&settings);
    let emit_thread_handler = guard.new_emit_thread(clients.inner().clone(), 
    udp_socket.inner().clone(), lock_supervisor.emit_thread_should_stop.clone()); 


    lock_supervisor.capture_thread = Some(capture_thread_handler);
    lock_supervisor.emit_thread = Some(emit_thread_handler);

    Ok(true)
}

#[tauri::command]
async fn off_thread_capture(thread_supervisor: State<'_, Arc<Mutex<ThreadSupervisor>>>) -> Result<bool, String> {
    match thread_supervisor.lock()
    {
        Ok(mut locked_supervisor) => {
            locked_supervisor.capture_thread_should_stop.store(true, Ordering::Relaxed);
            locked_supervisor.emit_thread_should_stop.store(true, Ordering::Relaxed);
            if let Some(capture_thread) = locked_supervisor.capture_thread.take() {
                match capture_thread.join() {
                    Ok(_) => {
                        println!("Capture thread properly stopped");
                        locked_supervisor.capture_thread_should_stop.store(false, Ordering::Relaxed);
                    },
                    Err(err) => {
                        println!("Error while stoping capture thread {:?}", err);
                        return Err("Error while stoping capture thread".to_string())
                    }
                }
            }
            else {
                return Err("No capture thread handler".to_string())
            }

            if let Some(emit_thread) = locked_supervisor.emit_thread.take() {
                match emit_thread.join() {
                    Ok(_) => {
                        println!("Emit thread properly stopped");
                        locked_supervisor.emit_thread_should_stop.store(false, Ordering::Relaxed);
                    },
                    Err(err) => {
                        println!("Error while stoping emit thread {:?}", err);
                        return Err("Error while stoping emit thread".to_string())
                    }
                }
            }
            else {
                return Err("No emit thread handler".to_string())
            }


            return Ok(true)
        },
        Err(err) => {
            println!("Error while locking supervisor {:?}", err);
            Err("Error while locking supervisor".to_string())
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_core = Arc::new(RwLock::new(AppCore::new()));
            app.manage(app_core);

            app.manage(Arc::new(Mutex::new(ThreadSupervisor {
                capture_thread: None,
                capture_thread_should_stop: Arc::new(AtomicBool::new(false)),
                emit_thread: None,
                emit_thread_should_stop: Arc::new(AtomicBool::new(false)),
            })));

            // Clients storage 
            let clients: Arc<arc_swap::ArcSwapAny<Arc<Vec<SocketAddr>>>> = Arc::new(ArcSwapAny::new(Arc::new(Vec::new())));
            app.manage(clients);

            // Application socket
            let socket: Arc<UdpSocket> = Arc::new(UdpSocket::bind("0.0.0.0:0").unwrap());
            app.manage(socket);

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_capture_thread, off_thread_capture])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
