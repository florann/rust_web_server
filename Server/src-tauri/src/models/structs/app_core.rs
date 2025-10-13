use std::{collections::VecDeque, net::{SocketAddr, UdpSocket}, sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc, Mutex, OnceLock}, thread::{self, JoinHandle}, time::{Duration, SystemTime}};
use std::time::{UNIX_EPOCH};

use once_cell::sync::Lazy;
use tokio::io::Join;
use windows_capture::{capture::GraphicsCaptureApiHandler, monitor::Monitor, settings::Settings};

use crate::models::structs::screen_capture::ScreenCapture;
use crate::CLIENT_NUMBER_SENDER;
use crate::GLOBAL_QUEUE;
use crate::MAX_UDP_PACKET_SIZE;



pub struct AppCore {
}

impl AppCore {
    pub fn new() -> Self {
        AppCore {

        }
    }

    pub fn new_capture_thread(&self, 
        settings: &Settings<Arc<AtomicBool>, Monitor>) -> JoinHandle<()> {

        let settings_clone = settings.clone();
        let handler = thread::spawn(move ||{
            println!("Thread capture started");
            let _ = ScreenCapture::start(settings_clone);
        });

        handler
    }

     pub fn new_emit_thread(&self, clients: Arc<arc_swap::ArcSwapAny<Arc<Vec<SocketAddr>>>>, 
        socket: Arc<UdpSocket>,
        should_stop: Arc<AtomicBool>) -> JoinHandle<()> {

        let mut buf = [0u8; 2];
        let client_copy = Arc::clone(&clients);

        let handler = thread::spawn(move ||{
            println!("Udp thread spawned");
            loop {
                if should_stop.load(Ordering::Relaxed) {
                    break
                }

                match socket.recv_from(&mut buf) {
                    Ok((nbytes, client_addr)) => {
                        if nbytes == 1 {

                                let mut new_clients = (**client_copy.load()).clone();
                                new_clients.push(client_addr);
                                let clients_len = new_clients.len();
                                client_copy.store(Arc::new(new_clients));

                                if let Some(client_number_mutex) = CLIENT_NUMBER_SENDER.get() {     
                                    if let Ok(client_number) = client_number_mutex.lock() {
                                        let _ = client_number.send(clients_len);
                                    }
                                }

                        }
                        else if nbytes == 2 {

                            let mut new_clients = (**client_copy.load()).clone();
                            
                            if let Some(client_position) = new_clients.iter().position(|client_stored| client_stored == &client_addr){
                                new_clients.remove(client_position);
                                let clients_len = new_clients.len();
                                client_copy.store(Arc::new(new_clients));
                                
                                if let Some(client_number_mutex) = CLIENT_NUMBER_SENDER.get() {     
                                    if let Ok(client_number) = client_number_mutex.lock() {
                                        let _ = client_number.send(clients_len);
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {

                    }
                }

                let item: Option<Vec<u8>> = {
                    let mut q = GLOBAL_QUEUE.lock().unwrap();
                    q.pop_front()
                };

                if let Some(data) = item {
                    // if data[4] & 0x1F != 1 {
                    //     println!("Fourth first bytes {:02x} {:02x} {:02x} {:02x} {:02x}", data[0], data[1], data[2] ,data[3],data[4]);
                    //     println!("NAL Type {} sent", data[4] & 0x1F);
                    //     println!("NAL data {} size", data.len());
                    // }

                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    let mut timestamped_data: Vec<u8> = Vec::new();
                    timestamped_data.extend_from_slice(&timestamp.to_be_bytes());
                    timestamped_data.extend_from_slice(&data);

                    // If packet above UDP limit, chunk
                    if timestamped_data.len() > MAX_UDP_PACKET_SIZE {
        
                        let header_chunk_begin: [u8; 4] = [0x01, 0x01, 0x01, 0x0F];
                        let header_chunk_end: [u8; 4] = [0x01, 0x01, 0x01, 0xFF];
                        let mut header_chunk: [u8;4];

                        while timestamped_data.len() > 0 {
                            let mut chunk_size = 0;
                            
                            if timestamped_data.len() > MAX_UDP_PACKET_SIZE  
                            { 
                                chunk_size = MAX_UDP_PACKET_SIZE;
                                header_chunk = header_chunk_begin;
                            } 
                            else { 
                                chunk_size = timestamped_data.len();
                                header_chunk = header_chunk_end;
                            };

                            let mut chunk: Vec<u8> = Vec::new();
                            let drained: Vec<u8> = timestamped_data.drain(0..chunk_size).collect();

                            chunk.extend_from_slice(&header_chunk);
                            //chunk.push(chunk_count);
                            chunk.extend_from_slice(&drained);

                            println!("Size chunk - {}", chunk.len());
                            println!("Timestamp chunk - {}", timestamp);

                            Self::send_to_clients(&socket, (**clients.load()).clone(), chunk);
                        }
                    }
                    else {
                        Self::send_to_clients(&socket, (**clients.load()).clone(), timestamped_data);
                    }


                };
            

                thread::sleep(Duration::from_millis(10));
            }
        });
        handler
    }

    pub fn send_to_clients(socket: &UdpSocket, clients: Vec<SocketAddr>, data: Vec<u8>) {
       for client in clients {
            let _ = socket.send_to(&data, client);
        }
    }
}