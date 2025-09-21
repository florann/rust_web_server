use core::time;
use std::collections::VecDeque;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{ net::UdpSocket, sync::mpsc, thread, time::Duration};

use rtp::packet::Packet;
use webrtc_util::marshal::Unmarshal;

use once_cell::sync::Lazy;
use openh264::decoder;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};
use openh264::{decoder::Decoder};
use pixels::{Pixels, SurfaceTexture};

static GLOBAL_QUEUE: Lazy<Arc<Mutex<VecDeque<Vec<u8>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));

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
    let (worker_sender1, worker_receiver1) = mpsc::channel::<Vec<u8>>();
    let (worker_sender2, worker_receiver2) = mpsc::channel::<Vec<u8>>();


    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    
    // Server address (adjust IP if server is on different machine)
    let server_addr = "127.0.0.1:8080";
    
    // Send 1 byte to subscribe
    let subscribe_message = [1u8]; // Single byte
    socket.send_to(&subscribe_message, server_addr).unwrap();

    //App Sender
    let handler_display = new_display_stream_thread(frame_sender);
    //Dispatcher 
    let handler_receive = new_receive_thread(&socket, worker_sender1, worker_sender2);
    //Worker
    let handler_worker1 = new_decoder_thread(worker_receiver1);
    let handler_worker2 = new_decoder_thread(worker_receiver2);

    
    // Run GUI (blocks until window closes)
    let event_loop = EventLoop::new()?;
    let mut app = App { 
        pixels: None,
        frame_receiver: Some(frame_receiver),
    };
    
    event_loop.run_app(&mut app)?;
    
    Ok(())
}

fn new_display_stream_thread(frame_sender: mpsc::Sender<Vec<u8>>) -> JoinHandle<()> {
    let handler = thread::spawn(move ||{
        loop {
            thread::sleep(time::Duration::from_millis(33));
            let item = {
                let mut q = GLOBAL_QUEUE.lock().unwrap();
                q.pop_front()
            };
    
            if let Some(data) = item {
                println!("data -> sending");
                if frame_sender.send(data).is_err() {
                    exit(1); 
                }
            }
        }
    });
    handler
}

fn new_decoder_thread(receiver: mpsc::Receiver<Vec<u8>>) -> JoinHandle<()> {
    let handler = thread::spawn(move || {
    let mut decoder = Decoder::new().unwrap();
    loop {
        match receiver.recv_timeout(time::Duration::from_millis(100)) {
                Ok(packet_data) => {

                    let rtp_packet = Packet::unmarshal(&mut packet_data.as_slice());
                    match(decoder.decode(&packet_data)) {
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
                        }
                    }
            },
            Err(error) => {
                //println!("Error : {}", error);
            }
        }
    }});
    handler
}

fn new_receive_thread(socket: &UdpSocket, worker_1: mpsc::Sender<Vec<u8>>, worker_2: mpsc::Sender<Vec<u8>>) -> JoinHandle<()> {
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut message_count = 0;

    let thread_socket = socket.try_clone().unwrap();
    let handler = thread::spawn(move ||{
        loop {
            thread::sleep(Duration::from_millis(10));
            match thread_socket.recv(&mut buffer) {
                Ok(bytes_received) => {
                    if bytes_received == 0 {
                        continue;
                    }

                    message_count += 1;
                    println!("Message #{}: {} bytes", message_count, bytes_received);
                    
                    let packet_data = &buffer[..bytes_received];
                    if message_count % 2 == 0 {
                        worker_1.send(packet_data.to_vec());
                    } else {
                        worker_2.send(packet_data.to_vec());
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