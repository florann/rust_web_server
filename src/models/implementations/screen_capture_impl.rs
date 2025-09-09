use windows_capture::graphics_capture_api::InternalCaptureControl;

use crate::models::structs::screen_capture::ScreenCapture;

impl ScreenCapture {
    pub fn stop_capture(&self, capture_control: InternalCaptureControl) {
        capture_control.stop();
    }
}
