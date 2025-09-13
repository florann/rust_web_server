mod models;
use std::{io::Read, net::{SocketAddr, TcpListener, UdpSocket}, sync::{Mutex, OnceLock}, thread, time::Duration};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use windows_capture::{capture::GraphicsCaptureApiHandler, monitor::Monitor, settings::{ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings}};

use crate::models::structs::{http_message::HttpMessage, screen_capture::ScreenCapture};

static FRAME_SENDER: OnceLock<Mutex<mpsc::Sender<Vec<u8>>>> = OnceLock::new();

fn main() {

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
        let mut buf = [0u8; 2];
        let mut clients: Vec<SocketAddr> = Vec::new();
        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        
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

        FRAME_SENDER.set(Mutex::new(sender)).unwrap();
        
        let handle_thread_udp = thread::spawn(move ||{
            println!("Udp thread spawned");
            loop {
 
                match socket.recv_from(&mut buf) {
                    Ok((nbytes, client_addr)) => {
                        if nbytes == 1 {
                            clients.push(client_addr);
                        }
                        else if nbytes == 2 {
                            if let Some(client_position) = clients.iter().position(|client_stored| client_stored == &client_addr){
                                clients.remove(client_position);
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
