mod models;
use std::collections::VecDeque;
use std::sync::mpsc::{Sender};
use std::sync::{Arc, Mutex};
use std::{ net::UdpSocket, sync::mpsc, thread};
use once_cell::sync::Lazy;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};
use openh264::{decoder::Decoder};
use pixels::{Pixels, SurfaceTexture};
use ffmpeg_next as ffmpeg;

use crate::models::structs::app::{self, App};
use crate::models::structs::gpu_decoder::GpuDecoder;

//Global configuration variables
static MAX_UDP_PACKET_SIZE:usize = 65535;
static BUFFER_LEN_BEFORE_PROCESS: usize = 60;

//Global usable variables
static GLOBAL_BUFFER: Lazy<Arc<Mutex<Vec<(u128,Vec<u8>)>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new())) 
});
static GLOBAL_SORTED: Lazy<Arc<Mutex<VecDeque<(u128,Vec<u8>)>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(VecDeque::new())) 
});



fn receive_packet(sender: &Sender<()>, udp_buffer: &Vec<u8>, chunk_buffer: &mut Vec<u8>,nb_bytes: usize) -> Result<(), String> {
        //If chunked 
        if udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0x0F])
        || udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0xFF]) {
            //println!("First ten bytes: {:02x?}", &udp_buffer[..100]);

            let data: Vec<u8> = udp_buffer[4..nb_bytes].to_vec(); 
            if udp_buffer[3] == 0xFF {
                //println!("Data bytes {:02x?}",data);
                chunk_buffer.extend_from_slice(&data);
                println!("Size chunk final  - {}", chunk_buffer.len());
                //println!("Chunkbuffer bytes {:02x?}", &chunk_buffer[0..100]);

                let tuple = parse_received_packet(chunk_buffer, chunk_buffer.len());
                chunk_buffer.clear();
                match add_packet_to_receiver(sender, tuple) {
                    Ok(()) => 
                    {
                        //println!("Succes : add_packet_to_receiver - CHUNK");
                        return Ok(())
                    },
                    Err(error) => {
                        //println!("Error : add_packet_to_receiver - CHUNK");
                        return Err(error);
                    }
                }
               
            }
            else {
                println!("Size chunk - {}", chunk_buffer.len());
                chunk_buffer.extend_from_slice(&data);
                println!("Size chunk - {}", chunk_buffer.len());
            }

            Ok(())
        }
        else {
            let tuple = parse_received_packet(udp_buffer, nb_bytes);
            match add_packet_to_receiver(sender, tuple) {
                Ok(()) => {
                    println!("Succes : add_packet_to_receiver");
                    Ok(())
                },
                Err(error) => {
                    println!("Error : add_packet_to_receiver");
                    Err(error)
                }
            }
        }
}

fn parse_received_packet(udp_buffer: &Vec<u8>, nb_bytes: usize) -> (u128, Vec<u8>) {
    let timestamp = u128::from_be_bytes(
        udp_buffer[0..16].try_into().unwrap()
    );
    
    let nal_data = udp_buffer[16..nb_bytes].to_vec();

    if nal_data[4] & 0x1F != 1 {
        println!("NAL Type Received : {}", nal_data[4] & 0x1F);
    }

    (timestamp, nal_data)
}

fn add_packet_to_receiver(sender: &Sender<()>,tuple: (u128, Vec<u8>)) -> Result<(), String> {
        match GLOBAL_BUFFER.lock() {
            Ok(mut global_buffer) => {
                println!("NAL Type [{}] - Push to global buffer", tuple.1[4] & 0x1F);
                global_buffer.push(tuple);
            },
            Err(err) => {
                return Err(err.to_string());
            }
        }

        match GLOBAL_BUFFER.lock() {
            Ok(global_buffer) => 
            {
                if global_buffer.len() > BUFFER_LEN_BEFORE_PROCESS {
                    println!("Sorting");
                    sender.send(()).ok();
                }
            }, 
            Err(err) => {
                return Err(err.to_string());
            }
        }
        Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let (frame_sender, frame_receiver) = mpsc::channel::<Vec<u8>>();
    let (sort_sender, sort_receiver) = mpsc::channel::<()>();


    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let server_addr = "192.168.1.190:8080";
    
    // Send 1 byte to subscribe
    let subscribe_message = [1u8]; // Single byte
    socket.send_to(&subscribe_message, server_addr).unwrap();

    //Thread to receive data 
        // Receive raw data 
        // Store them in global queue
            // -> Check the first 16 bytes ( timestamp of the frame ) Before check their orders
    //Thread to lookup and display frames
    let copy_sort_sender = sort_sender.clone();

    let handler_receiver_thread = thread::spawn(move ||{
        let mut udp_buffer = vec![0u8; MAX_UDP_PACKET_SIZE];
        let mut chunk_buffer = Vec::new();
        let mut packet_number: usize = 0;

        loop {
            match socket.recv(&mut udp_buffer) {
                Ok(nb_bytes) => {
                    packet_number += 1;
                   match receive_packet(&copy_sort_sender, &udp_buffer, &mut chunk_buffer, nb_bytes) {
                        Ok(()) => println!("Success : Receive packet OK : {}", packet_number),
                        Err(err) => {
                            println!("Error : Receive packet {}", err);
                        } 
                      
                   }
                },
                Err(error) => {
                      eprintln!("Socket recv error: {}", error);
                }
            }
        }
    });

    let handler_sorter_thread = thread::spawn(move ||{
        loop {
            match sort_receiver.recv() {
                Ok(()) => {
                    let mut buffer = GLOBAL_BUFFER.lock().unwrap();
                    if buffer.len() > BUFFER_LEN_BEFORE_PROCESS 
                    {
                        let mut global_buffer_drain: Vec<(u128, Vec<u8>)> = buffer.drain(0..31).collect();
                        drop(buffer);

                        global_buffer_drain.sort_by_key(|key| key.0);
                        println!("Filling sorted buffer");
                        GLOBAL_SORTED.lock().unwrap().extend(global_buffer_drain);
                    }
                }, 
                Err(err) => {
                    println!("Try receive error {}", err);
                }
            } 
        }
    });

    // Run GUI (blocks until window closes)
    let event_loop = EventLoop::new()?;
    let mut app = App { 
        pixels: None,
        decoder: GpuDecoder::new(ffmpeg::codec::Id::H264).unwrap(),
        window: None
    };
    
    event_loop.run_app(&mut app)?;
    
    Ok(())
}

