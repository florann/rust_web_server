use std::collections::VecDeque;
use std::process::exit;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{fs::OpenOptions, net::UdpSocket, sync::mpsc, thread, time::Duration};
use std::io::Write; 

use once_cell::sync::Lazy;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};
use openh264::{decoder::Decoder};
use pixels::{Pixels, SurfaceTexture};

static GLOBAL_QUEUE: Lazy<Arc<Mutex<VecDeque<Vec<u8>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));

fn log_to_file(message: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)  // Append instead of overwrite
        .open("./log/debug.log")
        .unwrap();
    
    writeln!(file, "{}", message).unwrap();
}

fn log_to_file_vec(message: &Vec<u8>) {
    let message_clone = message.clone();
    let handle = thread::spawn(move|| {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)  // Append instead of overwrite
            .open("./log/debug.log")
            .unwrap();
        
        println!("Log in file");
        writeln!(file, "{:?}", message_clone).unwrap();
        writeln!(file, "---").unwrap(); // Separator line
        file.flush().unwrap(); // Force write to disk
    });

    //handle.join().unwrap(); // Wait for thread to finish
}

struct App <'a>{
      pixels: Option<Pixels<'a>>,
      frame_receiver: Option<mpsc::Receiver<Vec<u8>>>,
}

impl<'a>  ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(
            Window::default_attributes()
                .with_title("Video Stream")
        ).unwrap();
        
             // Create SurfaceTexture from window
        let surface_texture = SurfaceTexture::new(1920, 1080, window);
        
        // Pass SurfaceTexture to Pixels::new()
        let pixels = Pixels::new(1920, 1080, surface_texture).unwrap();

        self.pixels = Some(pixels);
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // Handle events
            if let Some(pixels) = &mut self.pixels {
                // Check for new frames
                if let Some(receiver) = &self.frame_receiver {
                    if let Ok(frame_data) = receiver.try_recv() {
                        // Update pixel buffer with new frame
                        pixels.frame_mut().copy_from_slice(&frame_data);
                    }
                }
                pixels.render().unwrap();
            }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (frame_sender, frame_receiver) = mpsc::channel::<Vec<u8>>();
    let shared_sender = Arc::new(Mutex::new(frame_sender));

    // Run GUI (blocks until window closes)
    let event_loop = EventLoop::new()?;
    let mut app = App { 
        pixels: None,
        frame_receiver: Some(frame_receiver),
    };

    let frame_sender_clone = shared_sender.clone();
    let handler_display = new_display_stream_thread(frame_sender_clone);

     
    let handler_receive = new_receive_thread();
    let handler_receive2 = new_receive_thread();
    
    handler_display.join().unwrap();
    handler_receive.join().unwrap();
    handler_receive2.join().unwrap();

    event_loop.run_app(&mut app)?;
    
    Ok(())
}

fn new_display_stream_thread(frame_sender: Arc<Mutex<Sender<Vec<u8>>>>) -> JoinHandle<()> {
    let handler = thread::spawn(move ||{
        loop {
            let item = {
                let mut q = GLOBAL_QUEUE.lock().unwrap();
                q.pop_front()
            };
    
            if let Some(data) = item {
                if frame_sender.lock().unwrap().send(data).is_err() {
                    println!("Wola c'est l'erreur du 69");
                    exit(1); 
                }
            }
        }
    });
    handler
}

fn new_receive_thread() -> JoinHandle<()> {
    let mut buffer = [0u8; 1024 * 1024];
    let mut message_count = 0;
    let mut decoder =   Decoder::new().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

    let thread_socket = socket.try_clone().unwrap();
    let handler = thread::spawn(move ||{
        loop {
            thread::sleep(Duration::from_millis(33));
            match thread_socket.recv(&mut buffer) {
                Ok(bytes_received) => {
                    if bytes_received == 0 {
                        continue;
                    }

                    message_count += 1;
                    println!("Message #{}: {} bytes", message_count, bytes_received);
                    
                    let packet_data = &buffer[..bytes_received];
                    
                    match(decoder.decode(packet_data)) {
                        Ok(Some(yuv_data)) => {

                            println!("Frame successfully decoded: {}x{}", 
                            (yuv_data.dimensions_uv().0 * 2), (yuv_data.dimensions_uv().1 * 2));

                            let (height, width) = yuv_data.dimensions_uv();
                            let mut rgba_buffer = vec![0u8; height * width * 4 *2 *2];

                            yuv_data.write_rgba8(&mut rgba_buffer);
                            GLOBAL_QUEUE.lock().unwrap().push_back(rgba_buffer);
                        },
                        Ok(None) => {
                            println!("Frame noot decoded");
                        }
                        Err(error) => {
                            println!("Decoding error: {}", error);
                            //println!("Entire buffer dump {:?}", buffer.to_vec());

                            // Check if this looks like H.264 data
                            if bytes_received >= 4 {
                                let nal_header = &buffer[0..4];
                                if nal_header == &[0x00, 0x00, 0x00, 0x01] {
                                    println!("Found H.264 start code");
                                    let nal_type = buffer[4] & 0x1F;
                                    println!("NAL type: {}", nal_type);
                                    log_to_file_vec(&buffer.to_vec());
                                    // break;
                                    
                                } else {
                                    println!("No H.264 start code found");
                                }
                            }
                        }
                    }


                    println!("\n");
                },
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
    });
    handler
}