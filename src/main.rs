mod models;
use std::{io::Read, net::{TcpListener}};
use crate::models::structs::http_message::HttpMessage;

fn main() {

    let listener = TcpListener::bind("127.0.0.1:1235").unwrap();

    loop {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    
                    let http_message = HttpMessage::new(stream);
                    match http_message {
                        Ok(http_message) => {
                            println!("{:?}", &http_message.start_line);
                            for header_field in &http_message.header_field {
                                println!("{:?}", header_field);
                            }
                            let string = http_message.body;
                            println!("{:?}", string);
                        },
                        Err(error) => {
                            println!("{}", error)
                        }
                    }
                },
                Err(error) => {

                }
            }
        }   
    } 
}
