use std::{fmt::Error, io::Read, net::TcpStream};
use crate::models::structs::http_message::HttpMessage;

impl HttpMessage  {
    pub fn new(mut tcp_stream: TcpStream) -> Result<Self, String> {
        let mut buf = [0u8;100 * 1024];
        let nb_bytes_read = tcp_stream.read(&mut buf);

        println!("{}", String::from_utf8_lossy(&buf).to_string());
        
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
        let mut body_start: usize = 0;

        for mut i in count..buf.len()-1 {

            if buf[i] == crlf[0] && buf[i + 1] != crlf[1] {
                break; // Invalide message
            }
            else if buf[i] == crlf[0] && buf[i + 1] == crlf[1] {
                vec_header_field.push(header_field_buf);
                header_field_buf.fill(0);
                buf_counter = 0;
                if buf[i + 2] == crlf[0] && buf[i + 3] == crlf[1]{
                    body_start += 6;
                    break;
                }
                i += 2;
                body_start += 2;
            }

            header_field_buf[buf_counter] = buf[i];
            buf_counter += 1;
            body_start += 1;
        }   


        //According to parse info 
        //Parse body for BODY_LENGTH given in header information 
        // Retriving body length information
        let mut body_content: Vec<u8> = Vec::new();
        let parsing_bytes = b"C\nContent-Length";
        let mut body_length: usize = 0;
        let split_char: u8 = b':';
        if let Some(slice) = vec_header_field
                .iter()
                .find(|&slice| {
                    slice.windows(parsing_bytes.len()).any(|window| window == parsing_bytes)
                }) {

            let mut split_char_index = slice.iter().position(|char| char == &split_char).unwrap();
            split_char_index += 1;

            let splited_slice: Vec<u8> = slice
                .iter()
                .skip(split_char_index)
                .take(slice.len() - split_char_index)
                .copied()
                .collect();

            let mut str: String = splited_slice.iter()
            .filter(|&&char| char >= 0x30 && char <= 0x39)
            .map(|&char| char as char)
            .collect();
            str = str.trim().to_string();

            body_length = str.parse().unwrap();
            println!("{}", body_length)

            //let body_length: u16 = body_header_content.parse().unwrap(); 
        }
        
        println!("{:?}", body_start);
        for i in body_start..body_start+body_length {
             if i >= buf.len()-1 {
                break;
            }

            body_content.push(buf[i]);
        }
    

        Ok(HttpMessage {
            start_line : request_line,
            header_field : vec_header_field,
            body: body_content
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