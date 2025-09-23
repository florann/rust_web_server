use core::time;
use std::cell::LazyCell;
use std::collections::VecDeque;
use std::env::current_exe;
use std::process::exit;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{ net::UdpSocket, sync::mpsc, thread, time::Duration};

use openh264::{decoder, Error, Timestamp};
use openh264::formats::YUVBuffer;
use rtp::packet::{self, Packet};
use webrtc_util::marshal::Unmarshal;

use once_cell::sync::Lazy;
use webrtc_util::vnet::chunk;
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
            // TODO : Analyze frame to see SPS PPS NAL 7 8
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
                    println!("Decoding error : {}", err);
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


fn receive_packet(sender: &Sender<()>, udp_buffer: &Vec<u8>, chunk_buffer: &mut Vec<u8>,nb_bytes: usize) -> Result<(), String> {
        //If chunked 
        if udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0x0F])
        || udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0xFF]) {
            //println!("First ten bytes: {:02x?}", &udp_buffer[..100]);

            let data: Vec<u8> = udp_buffer[4..].to_vec(); 
            if udp_buffer[3] == 0xFF {
                //println!("Data bytes {:02x?}",data);
                chunk_buffer.extend_from_slice(&data);

                //println!("Chunkbuffer bytes {:02x?}", &chunk_buffer[0..100]);

                let tuple = parse_received_packet(chunk_buffer, chunk_buffer.len());
                chunk_buffer.clear();
                match add_packet_to_receiver(sender, tuple) {
                    Ok(()) => 
                    {
                        //println!("Succes : add_packet_to_receiver - CHUNK");
                        return Ok(())
                    },
                    Err(error) => {
                        //println!("Error : add_packet_to_receiver - CHUNK");
                        return Err(error);
                    }
                }
               
            }
            else {
                chunk_buffer.extend_from_slice(&data);
            }

            Ok(())
        }
        else {
            let tuple = parse_received_packet(udp_buffer, nb_bytes);
            match add_packet_to_receiver(sender, tuple) {
                Ok(()) => {
                    println!("Succes : add_packet_to_receiver");
                    Ok(())
                },
                Err(error) => {
                    println!("Error : add_packet_to_receiver");
                    Err(error)
                }
            }
        }
}

fn parse_received_packet(udp_buffer: &Vec<u8>, nb_bytes: usize) -> (u128, Vec<u8>) {
    let timestamp = u128::from_be_bytes(
        udp_buffer[0..16].try_into().unwrap()
    );
    
    let nal_data = udp_buffer[16..nb_bytes].to_vec();

    if nal_data[4] & 0x1F != 1 {
        println!("NAL Type Received : {}", nal_data[4] & 0x1F);
    }

    (timestamp, nal_data)
}

fn add_packet_to_receiver(sender: &Sender<()>,tuple: (u128, Vec<u8>)) -> Result<(), String> {
        match GLOBAL_BUFFER.lock() {
            Ok(mut global_buffer) => {
                global_buffer.push(tuple);
                println!("Data push global buffer");
            },
            Err(err) => {
                return Err(err.to_string());
            }
        }

        match GLOBAL_BUFFER.lock() {
            Ok(global_buffer) => 
            {
                if global_buffer.len() > 30 {
                    println!("Sorting");
                    sender.send(()).ok();
                }
            }, 
            Err(err) => {
                return Err(err.to_string());
            }
        }
        Ok(())
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
        let mut chunk_buffer = Vec::new();
        let mut packet_number: usize = 0;

        loop {
            match socket.recv(&mut udp_buffer) {
                Ok(nb_bytes) => {
                    packet_number += 1;
                   match receive_packet(&copy_sort_sender, &udp_buffer, &mut chunk_buffer, nb_bytes) {
                        Ok(()) => println!("Success : Receive packet OK : {}", packet_number),
                        Err(err) => {
                            println!("Error : Receive packet {}", err);
                        } 
                      
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
                    let mut buffer = GLOBAL_BUFFER.lock().unwrap();
                    if buffer.len() > 30 
                    {
                        let mut global_buffer_drain: Vec<(u128, Vec<u8>)> = buffer.drain(0..31).collect();
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

