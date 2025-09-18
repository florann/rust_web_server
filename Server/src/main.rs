mod models;
use std::{collections::VecDeque, io::Read, net::{SocketAddr, TcpListener, UdpSocket}, sync::{Arc, Mutex, OnceLock}, thread::{self, JoinHandle}, time::Duration};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use arc_swap::ArcSwap;
use windows_capture::{capture::GraphicsCaptureApiHandler, monitor::Monitor, settings::{ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings}};

use crate::models::structs::{http_message::HttpMessage, record_buffer::RecordBuffer, screen_capture::ScreenCapture};

static FRAME_SENDER: OnceLock<Mutex<mpsc::Sender<Vec<u8>>>> = OnceLock::new();
static CLIENT_NUMBER_SENDER: OnceLock<Mutex<mpsc::Sender<usize>>> = OnceLock::new();
static CLIENT_NUMBER_RECEIVER: OnceLock<Mutex<mpsc::Receiver<usize>>> = OnceLock::new();

 fn new_capture_thread(settings: Settings<String, Monitor>) -> JoinHandle<()> {
    let handler = thread::spawn(move ||{
        println!("Thread capture started");
        let _ = ScreenCapture::start(settings);
    });

    return  handler;
 }

 fn new_emit_thread(clients: Arc<arc_swap::ArcSwapAny<Arc<Vec<SocketAddr>>>>, socket: UdpSocket) -> JoinHandle<()> {
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();
    let mut buf = [0u8; 2];
    let client_copy = Arc::clone(&clients);

    let handler = thread::spawn(move ||{
        println!("Udp thread spawned");
        loop {

            match socket.recv_from(&mut buf) {
                Ok((nbytes, client_addr)) => {
                    if nbytes == 1 {

                            let mut new_clients = (**client_copy.load()).clone();
                            new_clients.push(client_addr);
                            let clients_len = new_clients.len();
                            client_copy.store(Arc::new(new_clients));

                            if let Some(client_number_mutex) = CLIENT_NUMBER_SENDER.get() {     
                                if let Ok(client_number) = client_number_mutex.lock() {
                                    let _ = client_number.send(clients_len);
                                }
                            }

                    }
                    else if nbytes == 2 {

                        let mut new_clients = (**client_copy.load()).clone();
                        
                        if let Some(client_position) = new_clients.iter().position(|client_stored| client_stored == &client_addr){
                            new_clients.remove(client_position);
                            let clients_len = new_clients.len();
                            client_copy.store(Arc::new(new_clients));
                            
                            if let Some(client_number_mutex) = CLIENT_NUMBER_SENDER.get() {     
                                if let Ok(client_number) = client_number_mutex.lock() {
                                    let _ = client_number.send(clients_len);
                                }
                            }
                        }
                    }
                },
                Err(_) => {

                }
            }

       
            match receiver.try_recv() {
                Ok(frame) => {
                    for client in (**clients.load()).clone() {
                        socket.send_to(&frame, client);
                    }
                }, 
                Err(mpsc::TryRecvError::Empty) => {
                    //println!("received empty");
                },
                Err(mpsc::TryRecvError::Disconnected) => {
                    println!("disconnected");
                    break;
                }
            }


            thread::sleep(Duration::from_millis(33));
        }
    });

    handler
 }

fn main() {

    let arc_record_buffer = Arc::new(Mutex::new(RecordBuffer{ buffer : VecDeque::new() }));
    let arcswap_clients: Arc<arc_swap::ArcSwapAny<Arc<Vec<SocketAddr>>>> = Arc::new(ArcSwap::from_pointee(Vec::<SocketAddr>::new()));

    let tcp_listener = TcpListener::bind("127.0.0.1:1235").unwrap();
    let udp_listener = UdpSocket::bind("127.0.0.1:1235").unwrap();

        let handle_thread_tcp = thread::spawn(move ||{
            println!("Tcp thread spawned");
            loop {
                for stream in tcp_listener.incoming() {
                    match stream {
                        Ok(mut stream) => {
                            
                            let http_message = HttpMessage::new(stream);
                            match http_message {
                                Ok(http_message) => {
                                    println!("{:?}", &http_message.start_line);
                                    for header_field in &http_message.header_field {
                                        println!("{:?}", header_field);
                                    }
                                    let string = http_message.body;
                                    println!("{:?}", string);
                                },
                                Err(error) => {
                                    println!("{}", error)
                                }
                            }
                        },
                        Err(error) => {
        
                        }
                    }
                }   
            } 
        });

        let socket = UdpSocket::bind("0.0.0.0:8080").unwrap();
        socket.set_nonblocking(true).unwrap();
       
        let mut clients: Vec<SocketAddr> = Vec::new();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();
        FRAME_SENDER.set(Mutex::new(sender)).unwrap();
        
        let (client_number_sender, client_number_receiver) = mpsc::channel::<usize>();
        CLIENT_NUMBER_SENDER.set(Mutex::new(client_number_sender)).unwrap();
        CLIENT_NUMBER_RECEIVER.set(Mutex::new(client_number_receiver)).unwrap();
        let mut buf = [0u8; 2];
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

        let handle_thread_udp = thread::spawn(move ||{
            println!("Udp thread spawned");
            loop {
 
                match socket.recv_from(&mut buf) {
                    Ok((nbytes, client_addr)) => {
                        if nbytes == 1 {
                            clients.push(client_addr);
                            if let Some(client_number_mutex) = CLIENT_NUMBER_SENDER.get() {     
                                if let Ok(client_number) = client_number_mutex.lock() {
                                    let _ = client_number.send(clients.len());
                                }
                            }
                        }
                        else if nbytes == 2 {
                            if let Some(client_position) = clients.iter().position(|client_stored| client_stored == &client_addr){
                                clients.remove(client_position);
                                if let Some(client_number_mutex) = CLIENT_NUMBER_SENDER.get() {     
                                    if let Ok(client_number) = client_number_mutex.lock() {
                                        let _ = client_number.send(clients.len());
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {

                    }
                }

                match receiver.try_recv() {
                    Ok(frame) => {
                        for client in &clients {
                            socket.send_to(&frame, client);
                        }
                    }, 
                    Err(mpsc::TryRecvError::Empty) => {
                        //println!("received empty");
                    },
                    Err(mpsc::TryRecvError::Disconnected) => {
                        println!("disconnected");
                        break;
                    }
                }


                thread::sleep(Duration::from_millis(33));
            }
        });

        let handle_thread_screen_capture = thread::spawn(move ||{
            let _ = ScreenCapture::start(settings);
        });

        handle_thread_tcp.join().unwrap();
        handle_thread_udp.join().unwrap();
        handle_thread_screen_capture.join().unwrap();

        
       // let graphics_capture_handler = GraphicsCaptureApiHandler::new();
    }
