use std::time::Instant;
use windows_capture::encoder::VideoEncoder;

pub struct ScreenCapture {
    // The video encoder that will be used to encode the frames.
    pub encoder: Option<VideoEncoder>,
    // To measure the time the capture has been running
    pub start: Instant,
}