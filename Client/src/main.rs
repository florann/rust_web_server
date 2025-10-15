mod models;
use std::collections::VecDeque;
use std::sync::{Arc, LazyLock, Mutex};
use std::{ net::UdpSocket, sync::mpsc};
use clap::Parser;
use winit::{event_loop::{EventLoop}
};
use ffmpeg_next as ffmpeg;

use crate::models::structs::app::{App};
use crate::models::structs::gpu_decoder::GpuDecoder;
use crate::models::structs::cli::Cli;

//Global server address
static SERVER_ADDRESS: LazyLock<Arc<Mutex<Option<String>>>> = 
    LazyLock::new(|| Arc::new(Mutex::new(None)));
    
//Global configuration variables
static MAX_UDP_PACKET_SIZE:usize = 65536;
static BUFFER_LEN_BEFORE_PROCESS: usize = 60;

//Global usable variables
static GLOBAL_BUFFER: LazyLock<Arc<Mutex<Vec<(u128,Vec<u8>)>>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(Vec::new())) 
});
static GLOBAL_SORTED: LazyLock<Arc<Mutex<VecDeque<(u128,Vec<u8>)>>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(VecDeque::new())) 
});



fn main() -> Result<(), Box<dyn std::error::Error>> {

    // TMP: CLI integration will be erased
    let cli = match Cli::try_parse() {
        Ok(cli) => {
            cli.ensure_argument_integrity();
            cli
        },
        Err(err) => {
            eprintln!("Unable to parse arguments : {}", err);
            std::process::exit(1);
        }
    };

    // Channel creation of threads
    let (sort_sender, sort_receiver) = mpsc::channel::<()>();
    // Socket creation
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").unwrap());
    let server_ip = cli.server_ip.to_string();
    let server_port = cli.port.to_string();
    // Building server address
    *SERVER_ADDRESS.lock().unwrap() = Some(format!("{}:{}",server_ip, server_port));


    // Run GUI (blocks until window closes)
    let event_loop = EventLoop::new()?;
    let mut app = App { 
        pixels: None,
        decoder: GpuDecoder::new(ffmpeg::codec::Id::H264).unwrap(),
        window: None,
        socket: socket,
        sort_receiver: sort_receiver,
        sort_sender: sort_sender,
        handler_sorter_thread: None,
        handler_receiver_thread: None
    };
    
    event_loop.run_app(&mut app)?;
    
    Ok(())
}

