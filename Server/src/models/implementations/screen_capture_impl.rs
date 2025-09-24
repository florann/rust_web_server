use crate::models::structs::screen_capture::ScreenCapture;

impl ScreenCapture {
    pub fn is_start_code(data: &[u8]) -> bool {
        if *data == [0x00,0x00,0x00,0x01] {
            return true;
        }
        false
    }
}
