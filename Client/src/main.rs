use std::{net::UdpSocket, sync::mpsc, thread, time::Duration};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};
use openh264::{decoder::Decoder, encoder::{self, Encoder}};
use pixels::{Pixels, SurfaceTexture};


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

    let udp_thread = thread::spawn(move ||{
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        
        // Subscribe (send 1 byte)
        socket.send_to(&[1], "127.0.0.1:8080").unwrap();
        println!("Subscribed! Waiting for messages...\n");
        
        let mut buffer = [0u8; 1024 * 1024];
        let mut decoder =   Decoder::new().unwrap();
        let mut message_count = 0;

        //let mut frame_buffer = Vec::new();

        loop {
            thread::sleep(Duration::from_millis(33));
            match socket.recv(&mut buffer) {
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
                            if frame_sender.send(rgba_buffer).is_err() {
                                println!("Wola c'est l'erreur du 69");
                                break; // GUI closed, exit thread 
                            }

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

    // Run GUI (blocks until window closes)
    let event_loop = EventLoop::new()?;
    let mut app = App { 
        pixels: None,
        frame_receiver: Some(frame_receiver),
    };
    
    event_loop.run_app(&mut app)?;

    udp_thread.join().unwrap();
    
    Ok(())
}