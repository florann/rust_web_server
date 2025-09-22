use core::time;
use std::cell::LazyCell;
use std::collections::VecDeque;
use std::env::current_exe;
use std::process::exit;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{ net::UdpSocket, sync::mpsc, thread, time::Duration};

use openh264::{decoder, Timestamp};
use openh264::formats::YUVBuffer;
use rtp::packet::{self, Packet};
use webrtc_util::marshal::Unmarshal;

use once_cell::sync::Lazy;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};
use openh264::{decoder::Decoder};
use pixels::{Pixels, SurfaceTexture};

static GLOBAL_BUFFER: Lazy<Arc<Mutex<Vec<(u128,Vec<u8>)>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new())) 
});

static GLOBAL_SORTED: Lazy<Arc<Mutex<VecDeque<(u128,Vec<u8>)>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(VecDeque::new())) 
});


struct App <'a>{
    pixels: Option<Pixels<'a>>,
    decoder: Decoder,
    window: Option<Arc<Window>>
}

impl<'a>  ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(
            Window::default_attributes()
                .with_title("Video Stream")
        ).unwrap();
        
        let arc_window: Arc<Window> = Arc::new(window);
        
        // Create SurfaceTexture from window
        let surface_texture = SurfaceTexture::new(1920, 1080, arc_window.clone());
        
        // Pass SurfaceTexture to Pixels::new()
        let pixels = Pixels::new(1920, 1080, surface_texture).unwrap();
        

        self.pixels = Some(pixels);
        self.window = Some(arc_window);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Update your pixel buffer
        self.update();
        
        // Draw to the screen
        self.draw();
        
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
       match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            _ => {}
        }
    }
}

impl <'a> App <'a>{
        fn update(&mut self) {
        // Process NAL units from the sorted queue
        while let Some((timestamp, nal_data)) = GLOBAL_SORTED.lock().unwrap().pop_front() {
            let nal_type = nal_data[3] & 0x1F;

            if nal_type != 1 {
                
                println!("NAL Type : {}", nal_type);

            }
            // println!("Fourth first bytes: {:02x} {:02x} {:02x} {:02x} ", nal_data[0], nal_data[1], nal_data[2], nal_data[3]);

            // println!("Popping data sorted buffer");
            // Feed NAL to decoder
             match self.decoder.decode(&nal_data) {
                    Ok(Some(yuv_frame)) => {// Convert YUV to RGB and update pixel buffer


                    if let Some(pixels) = &mut self.pixels {
                        let frame_buffer = pixels.frame_mut();
                        yuv_frame.write_rgba8(frame_buffer);
                    }
                    break; // Process one frame per update
                },
                Ok(None) => {
                    println!("None return");
                }
                Err(err) => {
                    //println!("Decoding error : {}", err);
                }
            }

        }
    }

    fn draw(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            // Render the pixel buffer to screen
            pixels.render().unwrap();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let (frame_sender, frame_receiver) = mpsc::channel::<Vec<u8>>();
    let (sort_sender, sort_receiver) = mpsc::channel::<()>();


    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let server_addr = "127.0.0.1:8080";
    
    // Send 1 byte to subscribe
    let subscribe_message = [1u8]; // Single byte
    socket.send_to(&subscribe_message, server_addr).unwrap();

    //Thread to receive data 
        // Receive raw data 
        // Store them in global queue
            // -> Check the first 16 bytes ( timestamp of the frame ) Before check their orders
    //Thread to lookup and display frames
    let copy_sort_sender = sort_sender.clone();

    let handler_receiver_thread = thread::spawn(move ||{
        let mut udp_buffer = vec![0u8; 65535];
        
        loop {
            match socket.recv(&mut udp_buffer) {
                Ok(nb_bytes) => {
                    //println!("Receiving message size : {}", nb_bytes);
                    //Getting timestamp 
                    let timestamp = u128::from_be_bytes(
                        udp_buffer[0..16].try_into().unwrap()
                    );
                    
                    let nal_data = udp_buffer[16..nb_bytes].to_vec();

                    if nal_data[4] & 0x1F != 1 {
                        println!("NAL Type Received : {}", nal_data[4] & 0x1F);
                    }

                    let tuple = (timestamp, nal_data);
                    GLOBAL_BUFFER.lock().unwrap().push(tuple);

                    if GLOBAL_BUFFER.lock().unwrap().len() > 300 {
                        println!("Sorting");
                        copy_sort_sender.send(()).ok();
                    }
                },
                Err(error) => {
                      eprintln!("Socket recv error: {}", error);
                }
            }
        }
    });

    let handler_sorter_thread = thread::spawn(move ||{
        loop {
            match sort_receiver.recv() {
                Ok(()) => {
                    println!("thread sorting");
                    let mut buffer = GLOBAL_BUFFER.lock().unwrap();
                    if(buffer.len() > 300)
                    {
                        let mut global_buffer_drain: Vec<(u128, Vec<u8>)> = buffer.drain(0..301).collect();
                        drop(buffer);

                        global_buffer_drain.sort_by_key(|key| key.0);
                        println!("Filling sorted buffer");
                        GLOBAL_SORTED.lock().unwrap().extend(global_buffer_drain);
                    }
                }, 
                Err(err) => {
                    println!("Try receive error {}", err);
                }
            } 
        }
    });

    // Run GUI (blocks until window closes)
    let event_loop = EventLoop::new()?;
    let mut app = App { 
        pixels: None,
        decoder: Decoder::new().unwrap(),
        window: None
    };
    
    event_loop.run_app(&mut app)?;
    
    Ok(())
}

