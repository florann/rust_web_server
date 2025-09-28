use std::process::{Command, Stdio};
use std::io::{Write, Read};

pub struct GpuEncoder {
    child: std::process::Child,
}

impl GpuEncoder {
    pub fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let child = Command::new("ffmpeg")  
            .args(&[
                "-f", "rawvideo",
                "-pix_fmt", "rgba", 
                "-s", &format!("{}x{}", width, height),
                "-r", "30",
                "-i", "-",           
                "-c:v", "h264_amf",  // AMD Encoder
                "-b:v", "5M",
                "-f", "h264",        
                "-"                  
            ])
            .stdin(Stdio::piped())   
            .stdout(Stdio::piped())  
            .stderr(Stdio::null())   
            .spawn()?;               
            
        Ok(Self { child })
    }
    
    pub fn encode_frame(&mut self, rgba_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Write raw RGBA frame to ffmpeg's stdin
        if let Some(stdin) = self.child.stdin.as_mut() {
            stdin.write_all(rgba_data)?;
            stdin.flush()?;
        }
        
        let mut encoded = Vec::new();
        if let Some(stdout) = self.child.stdout.as_mut() {
            let mut buffer =  [0u8; 65507];
            match stdout.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    encoded.extend_from_slice(&buffer[..n]);
                },
                _ => {}
            }
        }
        
        Ok(encoded)
    }
}