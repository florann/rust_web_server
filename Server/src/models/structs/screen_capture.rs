use std::io::{self, Write};
use openh264::encoder::Encoder;
use openh264::formats::{RgbaSliceU8, YUVBuffer};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::settings::ColorFormat;
use crate::models::structs::stop_watch::StopWatch;
use crate::models::structs::gpu_encoder::GpuEncoder;
use crate::CLIENT_NUMBER_RECEIVER;
use crate::GLOBAL_QUEUE;


pub struct ScreenCapture {
    // The video encoder that will be used to encode the frames.
    pub encoder: Option<GpuEncoder>,
    pub client_number: usize,
    pub stop_watch: StopWatch,
    pub frame_counter: usize
}

impl ScreenCapture {
    pub fn is_start_code(data: &[u8]) -> bool {
        if *data == [0x00,0x00,0x00,0x01] {
            return true;
        }
        false
    }
}

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
            encoder: Some(GpuEncoder::new(1920,1080).unwrap()),
            client_number: 0,
            stop_watch: StopWatch::new(),
            frame_counter: 0
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

        if let Some(client_number_mutex) = CLIENT_NUMBER_RECEIVER.get() {
            if let Ok(client_number_receiver) = client_number_mutex.lock() {
                if let Ok(client_number) = client_number_receiver.try_recv() {
                    if client_number > self.client_number {
                         if let Some(encoder) = &mut self.encoder {
                            *encoder = GpuEncoder::new(1920,1080).unwrap();
    
                                println!("");
                                println!("Encoder recreation");
                         }
                    } 
                }
            }
        }
        
        let mut frame_buffer = frame.buffer()?;
        let encoded_data = if let Some(encoder) = &mut self.encoder {
            match encoder.encode_frame(&frame_buffer.as_raw_buffer()) {
                Ok(encoded_bit_stream) => {
                    println!("Size encoded - {}", encoded_bit_stream.len());
                    Some(encoded_bit_stream.to_vec())
                },
                Err(error) => {
                    println!("Encoding error: {}", error);
                    None
                }
            }
        }
        else {
            None
        };
   
        if let Some(data) = encoded_data {
            // First check if data is empty
            if data.is_empty() {
                println!("Empty data received from encoder");
                return Ok(());
            }
            // TODO ; Probably send full UDP packet, without splitting by NAL unit
            // gain of performance for encoding + sending, less global buffer locking
            let mut pos: usize = 0;
            let mut nal_start: Option<usize> = None;
            // Fragmentation per Unit type
            while pos <= data.len().saturating_sub(4) {
                if Self::is_start_code(&data[pos..pos+4]) {
                    if let Some(start) = nal_start {
                        let nal_data = data[start..pos].to_vec();
                        
                        if start + 4 < pos && start + 4 < data.len() {
                            let nal_type = data[start + 4] & 0x1F;
                            if nal_type == 1 {
                                self.frame_counter += 1;
                            }
                            println!("NAL Type: {} - Size: {} bytes", nal_type, nal_data.len());
                            
                            GLOBAL_QUEUE.lock().unwrap().push_back(nal_data);
                        }
                    }
                    
                    nal_start = Some(pos);
                }
                pos += 1;
            }
            
            // Handling last NAL Unit 
            if let Some(start) = nal_start {
                if start < data.len() {
                    let nal_data = data[start..].to_vec();
                    if start + 4 < data.len() && !nal_data.is_empty() {
                        let nal_type = data[start + 4] & 0x1F;
                        println!("NAL Type: {} - Size: {} bytes", nal_type, nal_data.len());
                        // Send to buffer
                        GLOBAL_QUEUE.lock().unwrap().push_back(nal_data);
                    } else if !nal_data.is_empty() {
                        println!("Sending NAL unit without type info - Size: {} bytes", nal_data.len());
                        // Send to buffer
                        GLOBAL_QUEUE.lock().unwrap().push_back(nal_data);
                    }
                }
            }
        }

        println!("Encoded frame number {}", self.frame_counter);

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) is closed.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture session ended");

        Ok(())
    }
}