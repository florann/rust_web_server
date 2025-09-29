use std::{sync::Arc, thread, time::Duration};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop}, window::{Window, WindowId}
};
use pixels::{Pixels, SurfaceTexture};

use crate::models::structs::gpu_decoder::GpuDecoder;

use crate::GLOBAL_SORTED;

pub struct App <'a>{
    pub pixels: Option<Pixels<'a>>,
    pub decoder: GpuDecoder,
    pub window: Option<Arc<Window>>
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
}

