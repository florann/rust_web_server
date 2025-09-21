use std::io::{self, Write};
use std::os::raw;
use std::sync::Arc;
use std::time::Instant;
use openh264::encoder::{self, Encoder};
use openh264::formats::{RGBSource, RgbaSliceU8, YUVBuffer, YUVSource};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::settings::ColorFormat;
use crate::models::structs::rgba_pixel::RgbaPixel;
use crate::models::structs::screen_capture::ScreenCapture;
use crate::CLIENT_NUMBER_RECEIVER;
use crate::GLOBAL_QUEUE;

impl GraphicsCaptureApiHandler for ScreenCapture {
    // The type of flags used to get the values from the settings.
    type Flags = String;

    // The type of error that can be returned from `CaptureControl` and `start`
    // functions.
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create a new instance. The flags can be
    // passed from settings.
    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        println!("Created with Flags: {}", ctx.flags);

        Ok(Self {
            encoder: Some(Encoder::new().unwrap()),
            client_number: 0
        })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        io::stdout().flush()?;
        frame.color_format();

        let color_format = frame.color_format();
        let frame_width = frame.width() as usize;
        let frame_height = frame.height() as usize;
        
        if let Some(client_number_mutex) = CLIENT_NUMBER_RECEIVER.get() {
            if let Ok(client_number_receiver) = client_number_mutex.lock() {
                if let Ok(client_number) = client_number_receiver.try_recv() {
                    if client_number > self.client_number {
                         if let Some(encoder) = &mut self.encoder {
                            *encoder = Encoder::new().unwrap();
    
                                println!("");
                                println!("Encoder recreation");
                         }
                    } 
                }
            }
        }

        let mut frame_buffer = frame.buffer()?;
        if let Some(encoder) = &mut self.encoder {

            let mut yuv_source: YUVBuffer = YUVBuffer::new(frame_width, frame_height);
            let rgb_source: RgbaSliceU8 = RgbaSliceU8::new(frame_buffer.as_raw_buffer(), (frame_width, frame_height));

            if color_format == ColorFormat::Rgba8 {
                yuv_source.read_rgb(rgb_source);
            }
            else if color_format == ColorFormat::Rgba16F {
                return Ok(());
            }

            match encoder.encode(&yuv_source) {
                Ok(encoded_bit_stream) => {
                    
                    GLOBAL_QUEUE.lock().unwrap().push_back(encoded_bit_stream.to_vec());
                },
                Err(error) => {
                    println!("Encoding error: {}", error);
                }
            }
        }

        // if let Some(data) = encoded_data {
        //     // First check if data is empty
        //     if data.is_empty() {
        //         println!("Empty data received from encoder");
        //         return Ok(());
        //     }

        //     let mut pos: usize = 0;
        //     let mut nal_start: Option<usize> = None;

        //     while pos <= data.len().saturating_sub(4) {
        //         if Self::is_start_code(&data[pos..pos+4]) {
        //             if let Some(start) = nal_start {
        //                 let nal_data = data[start..pos].to_vec();
                        
        //                 if start + 4 < pos && start + 4 < data.len() {
        //                     let nal_type = data[start + 4] & 0x1F;
        //                     println!("NAL Type: {} - Size: {} bytes", nal_type, nal_data.len());
                            
        //                     GLOBAL_QUEUE.lock().unwrap().push_back(nal_data);
        //                 }
        //             }
                    
        //             nal_start = Some(pos);
        //         }
        //         pos += 1;
        //     }

        //     if let Some(start) = nal_start {
        //         if start < data.len() {
        //             let nal_data = data[start..].to_vec();
        //             if start + 4 < data.len() && !nal_data.is_empty() {
        //                 let nal_type = data[start + 4] & 0x1F;
        //                 println!("NAL Type: {} - Size: {} bytes", nal_type, nal_data.len());
        //                 // Send to buffer
        //                 GLOBAL_QUEUE.lock().unwrap().push_back(nal_data);
        //             } else if !nal_data.is_empty() {
        //                 println!("Sending NAL unit without type info - Size: {} bytes", nal_data.len());
        //                 // Send to buffer
        //                 GLOBAL_QUEUE.lock().unwrap().push_back(nal_data);
        //             }
        //         }
        //     }
        // }

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) is closed.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture session ended");

        Ok(())
    }
}