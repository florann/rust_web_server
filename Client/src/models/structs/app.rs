use std::sync::{Arc};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop}, window::{Window, WindowId}
};
use openh264::{decoder::Decoder};
use pixels::{Pixels, SurfaceTexture};

use crate::GLOBAL_SORTED;

pub struct App <'a>{
    pub pixels: Option<Pixels<'a>>,
    pub decoder: Decoder,
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
            // if nal_type != 1 {
            //     println!("Update frame - NAL Type : {}", nal_type);
            //     println!("Update frame - Timestamp : {}", timestamp);
            //     println!("Beginning nal_data: {:02x?},{:02x?},{:02x?},{:02x?},{:02x?},{:02x?},{:02x?}",
            // nal_data[0],nal_data[1],nal_data[2],nal_data[3],nal_data[4],nal_data[5],nal_data[6]);
            // }
            // println!("Fourth first bytes: {:02x} {:02x} {:02x} {:02x} ", nal_data[0], nal_data[1], nal_data[2], nal_data[3]);
            // Feed NAL to decoder
             match self.decoder.decode(&nal_data) {
                    Ok(Some(yuv_frame)) => {// Convert YUV to RGB and update pixel buffer
                        println!("xxxxxxx Success decoding xxxxxx");

                    if let Some(pixels) = &mut self.pixels {
                        let frame_buffer = pixels.frame_mut();
                        yuv_frame.write_rgba8(frame_buffer);
                    }
                    break; // Process one frame per update
                },
                Ok(None) => {
                    println!("--------------");
                }
                Err(err) => {
                    println!("Error decoding  : {}", err);
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

