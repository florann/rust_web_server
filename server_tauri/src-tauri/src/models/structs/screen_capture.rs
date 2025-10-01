use core::panic;
use std::io::{self, Write};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;

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

    fn process_nals(&mut self, data: &[u8]) {
        let mut pos: usize = 0;
        let mut nal_start: Option<usize> = None;

        while pos + 4 <= data.len() {
            if Self::is_start_code(&data[pos..pos + 4]) {
                if let Some(start) = nal_start {
                    let nal = &data[start..pos];
                    if start + 4 < pos {
                        let nal_type = data[start + 4] & 0x1F;
                        if nal_type == 1 { self.frame_counter += 1; }
                            // enqueue nal
                            GLOBAL_QUEUE.lock().unwrap().push_back(nal.to_vec());
                    }
                }
                nal_start = Some(pos);
            }
            pos += 1;
        }
        if let Some(start) = nal_start {
                if start < data.len() {
                    GLOBAL_QUEUE.lock().unwrap().push_back(data[start..].to_vec());
            }
        }
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

        let decoder = match GpuEncoder::new(1920,1080) {
            Ok(decoder) => {
                decoder
            },
            Err(err) => {
                println!("Error : {}", err);
                panic!("Decoder not initialized");
            }
        };

        Ok(Self {
            encoder: Some(decoder),
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
        let rgba = frame_buffer.as_raw_buffer();
        if let Some(enc) = &self.encoder {
            match enc.enqueue_frame(rgba) {
                Ok(()) => (),
                Err(err) => {
                    println!("Error {}", err);
                }
            }
            // pull whatever bytes are available right now (non-blocking)
            let data = enc.take_available();

            if !data.is_empty() {
                // Your existing NAL parsing path
                println!("Process data");
                self.process_nals(&data);
            }
        }
        //println!("Encoded frame number {}", self.frame_counter);

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) is closed.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture session ended");

        Ok(())
    }
}