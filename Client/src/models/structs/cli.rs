use clap::Parser;
use std::net::IpAddr;

#[derive(Parser)]
#[command(name = "client")]
#[command(about = "CLI to run client", long_about = None)]
pub struct Cli {
    // Ip address of the emitting server
    #[arg(short, long, default_value = "127.0.0.1")]
    pub server_ip: IpAddr,
    // Port of the emitting server related to the server address
    #[arg(short, long, default_value = "8080", value_parser = clap::value_parser!(u16).range(1024..))]
    pub port: u16,
}

impl Cli {
    pub fn ensure_argument_integrity(&self) {
        // Add future validators if needed
    }
}