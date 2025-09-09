use std::intrinsics::fabsf128;
use std::io::{self, Write};
use std::os::raw;
use std::sync::Arc;
use std::time::Instant;
use openh264::encoder::{self, Encoder};
use openh264::formats::YUVSource;
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use crate::models::structs::screen_capture::ScreenCapture;

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

        let encoder = Encoder::new().unwrap();

        Ok(Self { encoder: Some(encoder), start: Instant::now() })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        print!("\rRecording for: {} seconds", self.start.elapsed().as_secs());
        io::stdout().flush()?;

        let raw_data = frame.buffer()?;
        if let Some(encoder) = &mut self.encoder {

            //let yuv_source: YUVSource = YUVSource 
            /*
            TODO
            Y = 0.299*R + 0.587*G + 0.114*B
            U = -0.147*R - 0.289*G + 0.436*B + 128
            V = 0.615*R - 0.515*G - 0.100*B + 128

            Interleaved → Planar
[RGBA,RGBA,RGBA...] → [YYY...][UUU...][VVV...]
             */
            encoder.encode(raw_data.as_raw_buffer());
        }

        // Send the frame to the video encoder
        //self.encoder.as_mut().unwrap().send_frame(frame)?;

        // Note: The frame has other uses too, for example, you can save a single frame
        // to a file, like this: frame.save_as_image("frame.png", ImageFormat::Png)?;
        // Or get the raw data like this so you have full
        // control: let data = frame.buffer()?;

        // Stop the capture after 6 seconds
        // if self.start.elapsed().as_secs() >= 6 {
        //     // Finish the encoder and save the video.
        //     self.encoder.take().unwrap().finish()?;

        //     capture_control.stop();

        //     // Because the previous prints did not include a newline.
        //     println!();
        // }

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) is closed.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture session ended");

        Ok(())
    }
}