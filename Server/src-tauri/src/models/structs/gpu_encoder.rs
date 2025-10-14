use std::process::{Command, Stdio};
use std::io::{Write, Read};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct GpuEncoder {
    child: std::process::Child,
    tx_frames: Sender<Vec<u8>>,   // frames -> writer thread
    rx_bits: Receiver<Vec<u8>>,   // encoded chunks <- reader thread
}

impl GpuEncoder {
    pub fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = Command::new("ffmpeg")
            .args([
                "-loglevel", "error",
                "-f", "rawvideo", "-pix_fmt", "rgba",
                "-s", &format!("{}x{}", width, height),
                "-r", "60",
                "-i", "-",
                "-vf", "format=nv12",
                "-c:v", "h264_amf",
                "-usage", "ultralowlatency",     // Changed from lowlatency
                "-quality", "speed",             // ADD: prioritize speed
                "-rc", "cqp",                    // Changed from cbr (try constant QP)
                "-qp_i", "23",                   // ADD: I-frame quality
                "-qp_p", "23",                   // ADD: P-frame quality
                "-bf", "0",
                "-gops_per_idr", "1",            // ADD: AMD-specific GOP setting
                "-header_insertion_mode", "idr", // ADD: force header insertion
                "-g", "60",
                "-sc_threshold", "0",
                "-fflags", "nobuffer",
                "-f", "h264",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        //frames to writer thread
        let (tx_frames, rx_frames) = mpsc::channel::<Vec<u8>>();
        //encoded bytes from reader thread
        let (tx_bits, rx_bits) = mpsc::channel::<Vec<u8>>();

       
        let mut stdin = child.stdin.take().expect("stdin piped");
        thread::spawn(move || {
            while let Ok(frame) = rx_frames.recv() {
                // write one tight RGBA frame
                if let Err(e) = stdin.write_all(&frame) {
                    eprintln!("ffmpeg stdin write error: {e}");
                    break;
                }
              
            }
        });

        let stdout = child.stdout.take().expect("stdout piped");
        thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stdout);
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        // forward chunk
                        let _ = tx_bits.send(buf[..n].to_vec());
                    }
                    Err(e) => {
                        eprintln!("ffmpeg stdout read error: {e}");
                        break;
                    }
                }
            }
        });

        Ok(Self { child, tx_frames, rx_bits })
    }

   
    pub fn enqueue_frame(&self, rgba: &[u8]) -> Result<(), Box<dyn std::error::Error>> {

        self.tx_frames.send(rgba.to_vec())?;
        Ok(())
    }


    pub fn take_available(&self) -> Vec<u8> {
        use std::sync::mpsc::TryRecvError;
        let mut out = Vec::new();
        loop {
            match self.rx_bits.try_recv() {
                Ok(chunk) => out.extend_from_slice(&chunk),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    pub fn finish(mut self) {
        drop(self.tx_frames.clone()); 
        let _ = self.child.wait();
    }
}