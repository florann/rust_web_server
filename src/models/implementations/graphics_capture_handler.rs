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
use crate::FRAME_SENDER;

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
            start: Instant::now(),
            bit_frame_encoded: Vec::new(),
            frame_sender: None, // ✅ Start with None (no sender)
        })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        print!("\rRecording for: {} seconds", self.start.elapsed().as_secs());
        io::stdout().flush()?;
        frame.color_format();

        let color_format = frame.color_format();
        let frame_width = frame.width() as usize;
        let frame_height = frame.height() as usize;

        let mut frame_buffer = frame.buffer()?;
        let encoded_data = if let Some(encoder) = &mut self.encoder {

            //let yuv_source: YUVSource = YUVSource 
            /*
                TODO
                Y = 0.299*R + 0.587*G + 0.114*B
                U = -0.147*R - 0.289*G + 0.436*B + 128
                V = 0.615*R - 0.515*G - 0.100*B + 128

                Interleaved → Planar
                [RGBA,RGBA,RGBA...] → [YYY...][UUU...][VVV...]
             */


            let mut yuv_source: YUVBuffer = YUVBuffer::new(frame_width, frame_height);
            let rgb_source: RgbaSliceU8 = RgbaSliceU8::new(frame_buffer.as_raw_buffer(), (frame_width, frame_height));

            if color_format == ColorFormat::Rgba8 {
                yuv_source.read_rgb(rgb_source);
            }
            else if color_format == ColorFormat::Rgba16F {

            }

            let result_encoded_bit_stream = encoder.encode(&yuv_source);
            match result_encoded_bit_stream {
                Ok(encoded_bit_stream) => {
                    Some(encoded_bit_stream.to_vec())
                },
                Err(error) => {
                    eprintln!("Encoding error: {}", error);
                    None
                }
            }
        }
        else {
            None
        };

        if let Some(data) = encoded_data {
           if let Some(sender_mutex) = FRAME_SENDER.get() {
            if let Ok(sender) = sender_mutex.lock() {
                let _ = sender.send(data);
            }
        }
        }

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) is closed.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture session ended");

        Ok(())
    }
}