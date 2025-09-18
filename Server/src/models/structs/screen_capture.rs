use std::{net::SocketAddr, sync::{Arc}};
use openh264::encoder::Encoder;

pub struct ScreenCapture {
    // The video encoder that will be used to encode the frames.
    pub encoder: Option<Encoder>,
    pub client_number: usize
}