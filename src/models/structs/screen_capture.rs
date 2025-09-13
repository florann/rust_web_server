use std::{sync::mpsc::Sender, time::Instant};
use openh264::encoder::Encoder;

pub struct ScreenCapture {
    // The video encoder that will be used to encode the frames.
    pub encoder: Option<Encoder>,
    // To measure the time the capture has been running
    pub start: Instant,

    pub bit_frame_encoded: Vec<u8>,

    pub frame_sender: Option<Sender<Vec<u8>>>,
    pub frame_counter: u16
}