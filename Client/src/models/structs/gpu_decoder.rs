use ffmpeg_next as ffmpeg;

pub struct GpuDecoder {
    decoder: ffmpeg::codec::decoder::Video, 
    frame: ffmpeg::util::frame::video::Video
}

impl GpuDecoder {
    pub fn new(codec_id :ffmpeg::codec::Id) -> Result<Self, ffmpeg::Error> {
        ffmpeg::init()?;
        let codec = ffmpeg::codec::decoder::find(codec_id).unwrap();
        let context = ffmpeg::codec::Context::new_with_codec(codec);
        let decoder = context.decoder().video()?;

        Ok(GpuDecoder { 
            decoder,  
            frame: ffmpeg::util::frame::video::Video::empty()
        })
    }

    pub fn decode_udp_packet(&mut self, data: Vec<u8>) -> Result<bool, ffmpeg::Error> {
        let packet = ffmpeg::Packet::copy(&data);
     
        self.decoder.send_packet(&packet)?;

        Ok(self.decoder.receive_frame(&mut self.frame).is_ok())
    }

    pub fn get_rgba_data(&self) -> Result<Vec<u8>, ffmpeg::Error> {
        // Create scaler context to convert to RGBA
        let mut scaler = ffmpeg::software::scaling::context::Context::get(
            self.frame.format(),
            self.frame.width(),
            self.frame.height(),
            ffmpeg::util::format::Pixel::RGBA,
            self.frame.width(),
            self.frame.height(),
            ffmpeg::software::scaling::flag::Flags::BILINEAR,
        )?;
        
        // Create output frame for RGBA data
        let mut rgba_frame = ffmpeg::util::frame::video::Video::new(
            ffmpeg::util::format::Pixel::RGBA,
            self.frame.width(),
            self.frame.height()
        );
        
        // Convert frame to RGBA
        scaler.run(&self.frame, &mut rgba_frame)?;
        
        // Get the raw pixel data
        let data = rgba_frame.data(0); // plane 0 contains RGBA data
        Ok(data.to_vec())
    }
}