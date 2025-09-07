pub struct HttpMessage {
    pub start_line: [u8; 1024],
    pub header_field: Vec<[u8;1024]>,
    pub body: Vec<u8>
}