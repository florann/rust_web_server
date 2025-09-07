use std::{fmt::Error, io::Read, net::TcpStream};
use crate::models::structs::http_message::HttpMessage;

impl HttpMessage  {
    pub fn new(mut tcp_stream: TcpStream) -> Result<Self, String> {
        let mut buf = [0u8;100 * 1024];
        let nb_bytes_read = tcp_stream.read(&mut buf);
        
        //Checking encoding US-ASCII
        let is_valid = buf
        .iter()
        .all(|byte| 
            Self::is_usascii_byte(byte)
        );
        if !is_valid {
            return Err("Invalid encoding".to_string());
        } 

        let mut count: usize = 0;

        //Parse the first line IS Request-line or Status-line
        //Until CRLF 
        let crlf = b"\r\n";
        let mut request_line: [u8; 1024] = [0; 1024]; 
        for i in 0..buf.len()-1 {
            if buf[i] == crlf[0] && buf[i+1] == crlf[1] {
                count = i + 2;
                break;
            }

            request_line[i] = buf[i];
        }

        //Parse X header 
        //Format : Something CRLF
        
        let mut vec_header_field: Vec<[u8; 1024]> = Vec::new();
        let mut header_field_buf: [u8; 1024] = [0; 1024];
        let mut buf_counter: usize = 0;

        for mut i in count..buf.len()-1 {

            if buf[i] == crlf[0] && buf[i + 1] != crlf[1] {
                break; // Invalide message
            }
            else if buf[i] == crlf[0] && buf[i + 1] == crlf[1] {
                vec_header_field.push(header_field_buf);
                header_field_buf.fill(0);
                buf_counter = 0;
                if buf[i + 2] == crlf[0] && buf[i + 3] == crlf[1]{
                    break;
                }
                i += 2;
            }

            header_field_buf[buf_counter] = buf[i];
            buf_counter += 1;
        }   


        //According to parse info 
        //Parse body for BODY_LENGTH given in header information 

        Ok(HttpMessage {
            start_line : request_line,
            header_field : vec_header_field,
            body: [0; 4096]
        })
    }

    fn is_usascii_byte(byte: &u8) -> bool {
        if *byte < 127 {
            true
        }
        else {
            false
        }

    }
}