use std::time::Instant;
use openh264::encoder::Encoder;

pub struct ScreenCapture {
    // The video encoder that will be used to encode the frames.
    pub encoder: Option<Encoder>,
    // To measure the time the capture has been running
    pub start: Instant,
}