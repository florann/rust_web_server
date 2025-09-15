use std::{sync::mpsc::Sender, time::Instant};
use openh264::encoder::Encoder;

pub struct ScreenCapture {
    // The video encoder that will be used to encode the frames.
    pub encoder: Option<Encoder>,
    pub frame_counter: u16,
    pub client_number: usize
}