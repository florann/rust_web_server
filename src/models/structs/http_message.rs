pub struct HttpMessage {
    pub start_line: String,
    pub header_field: Vec<String>,
    pub body: String
}