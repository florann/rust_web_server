use std::sync::mpsc::Sender;
use std::time::Instant;

use openh264::encoder::Encoder;
use windows_capture::{frame::FrameBuffer, graphics_capture_api::InternalCaptureControl};

use crate::models::structs::screen_capture::ScreenCapture;
use crate::models::structs::rgba_pixel::RgbaPixel;

impl ScreenCapture {
    pub fn set_frame_sender(&mut self, frame_sender: Sender<Vec<u8>>){
        self.frame_sender = Some(frame_sender);
    }

    pub fn set_encoded_frame(&mut self, data: Vec<u8>) {
        self.bit_frame_encoded = data;
    }

    pub fn stop_capture(&self, capture_control: InternalCaptureControl) {
        capture_control.stop();
    }

    pub fn get_rgba_from_frame_buffer(frame_buffer: &mut FrameBuffer) -> Result<Vec<RgbaPixel>, String> {
        let raw_data = frame_buffer.as_raw_buffer();
        let mut vec_rgba_pixels: Vec<RgbaPixel> = Vec::new();
        
        /* [r, g, b, a, r, g, b, a....] */
        if raw_data.len() % 4 == 0 {
            for mut i in (0..raw_data.len()).step_by(4) {
                vec_rgba_pixels.push(RgbaPixel{
                    red: raw_data[i],
                    green: raw_data[i+1],
                    blue: raw_data[i+2],
                    alpha: raw_data[i+3]
                });
            } 
            return Ok(vec_rgba_pixels);
        } else {
            return Err("Wrong raw_data length".to_string())
        }

    }
    
    pub fn get_bgra_from_frame_buffer(frame_buffer: FrameBuffer) {
        /* [b, g, r, a, b, g, r, a....] */
    }
}
