use std::{net::UdpSocket, sync::{mpsc::{Receiver, Sender}, Arc}, thread::{self, JoinHandle}, time::Duration};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop}, window::{Window, WindowId}
};
use pixels::{Pixels, SurfaceTexture};

use crate::{models::structs::gpu_decoder::GpuDecoder, SERVER_ADDRESS};

use crate::GLOBAL_SORTED;
use crate::GLOBAL_BUFFER;
use crate::BUFFER_LEN_BEFORE_PROCESS;
use crate::MAX_UDP_PACKET_SIZE;

pub struct App <'a>{
    pub pixels: Option<Pixels<'a>>,
    pub decoder: GpuDecoder,
    pub window: Option<Arc<Window>>,
    pub socket: Arc<UdpSocket>,
    pub sort_sender: Sender<()>,
    pub sort_receiver: Receiver<()>,
    pub handler_receiver_thread: Option<JoinHandle<()>>,
    pub handler_sorter_thread: Option<JoinHandle<()>>
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
            let nal_type = nal_data[4] & 0x1F;

            if nal_type == 9 {
                break;
            }

            println!("Sending to decoder {}", nal_type);

            // Feed NAL to decoder
             match self.decoder.decode_udp_packet(nal_data) {
                    Ok(isSuccess) => {
                        println!("xxxxxxx Success decoding xxxxxx");
                    if isSuccess {
                        if let Some(pixels) = &mut self.pixels {
                            match self.decoder.get_rgba_data() {
                                Ok(mut rgba) => {
                                    let mut frame_buffer = pixels.frame_mut();
                                    let copy_len = frame_buffer.len().min(rgba.len());
                                    frame_buffer[..copy_len].copy_from_slice(&rgba[..copy_len]);
                                    thread::sleep(Duration::from_millis(33));
                                },
                                Err(err) => {
    
                                }
                            }
                        }
                        break; // Process one frame per update
                    } 
                    else {
                        //println!("Unsuccessful decoding");
                    }
                },
                Err(err) => {
                    //println!("Error decoding  : {}", err);
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
    // Function to subscribe to server
    pub fn subscribe_to_server(&self) {
        let opt = SERVER_ADDRESS.lock().unwrap().clone();
        if let Some(addr) = opt {
            let subscribe_message = [1u8]; // Single byte
            self.socket.send_to(&subscribe_message, addr).unwrap();
        }
    }
    // Spawn thread to receive data
    pub fn spawn_receiver_thread(&mut self, sort_sender: Sender<()>) {

        let copy_socket = self.socket.clone();

        let handler_receiver_thread = thread::spawn(move ||{
            let mut udp_buffer = vec![0u8; MAX_UDP_PACKET_SIZE];
            let mut chunk_buffer = Vec::new();

            loop {
                match copy_socket.recv(&mut udp_buffer) {
                    Ok(nb_bytes) => {
                    match Self::receive_packet(&sort_sender, &udp_buffer, &mut chunk_buffer, nb_bytes) {
                            Ok(()) => (),
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
        self.handler_receiver_thread = Some(handler_receiver_thread);
    }
    // Spawn thread to sort data
    pub fn spawn_sorter_thread(&mut self, sort_receiver: Receiver<()>) {
        let handler_sorter_thread = thread::spawn(move ||{
            loop {
                match sort_receiver.recv() {
                    Ok(()) => {
                        let mut buffer = GLOBAL_BUFFER.lock().unwrap();
                        if buffer.len() > BUFFER_LEN_BEFORE_PROCESS 
                        {
                            let mut global_buffer_drain: Vec<(u128, Vec<u8>)> = buffer.drain(0..31).collect();
                            drop(buffer);

                            global_buffer_drain.sort_by_key(|key| key.0);
                            GLOBAL_SORTED.lock().unwrap().extend(global_buffer_drain);
                        }
                    }, 
                    Err(err) => {
                        println!("Try receive error {}", err);
                    }
                } 
            }
        });
        self.handler_sorter_thread = Some(handler_sorter_thread);
    }
    // Receive packets
    fn receive_packet(sender: &Sender<()>, udp_buffer: &Vec<u8>, chunk_buffer: &mut Vec<u8>,nb_bytes: usize) -> Result<(), String> {
        //If chunked 
        if udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0x0F])
        || udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0xFF]) {
            //println!("First ten bytes: {:02x?}", &udp_buffer[..100]);

            let data: Vec<u8> = udp_buffer[4..nb_bytes].to_vec(); 
            if udp_buffer[3] == 0xFF {
                //println!("Data bytes {:02x?}",data);
                chunk_buffer.extend_from_slice(&data);
                let tuple = Self::parse_received_packet(chunk_buffer, chunk_buffer.len());
                *chunk_buffer = Vec::new();
                match Self::add_packet_to_receiver(sender, tuple) {
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
                println!("Size chunk - {}", chunk_buffer.len());
            }

            Ok(())
        }
        else {
            let tuple = Self::parse_received_packet(udp_buffer, nb_bytes);
            match Self::add_packet_to_receiver(sender, tuple) {
                Ok(()) => {
                    Ok(())
                },
                Err(error) => {
                    println!("Error : add_packet_to_receiver");
                    Err(error)
                }
            }
        }
    }
    // Parse received packets
    fn parse_received_packet(udp_buffer: &Vec<u8>, nb_bytes: usize) -> (u128, Vec<u8>) {
        let timestamp = u128::from_be_bytes(
            udp_buffer[0..16].try_into().unwrap()
        );
        
        let nal_data = udp_buffer[16..nb_bytes].to_vec();

        if nal_data[4] & 0x1F != 1 {
            
        }

        (timestamp, nal_data)
    }
    // Add packet to receiver buffer
    fn add_packet_to_receiver(sender: &Sender<()>,tuple: (u128, Vec<u8>)) -> Result<(), String> {
            match GLOBAL_BUFFER.lock() {
                Ok(mut global_buffer) => {
                    //println!("NAL Type [{}] - Push to global buffer", tuple.1[4] & 0x1F);
                    global_buffer.push(tuple);
                },
                Err(err) => {
                    return Err(err.to_string());
                }
            }

            match GLOBAL_BUFFER.lock() {
                Ok(global_buffer) => 
                {
                    if global_buffer.len() > BUFFER_LEN_BEFORE_PROCESS {
                        sender.send(()).ok();
                    }
                }, 
                Err(err) => {
                    return Err(err.to_string());
                }
            }
            Ok(())
    }

}

