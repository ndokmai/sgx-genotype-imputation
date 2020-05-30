use std::net::{TcpStream, ToSocketAddrs, SocketAddr};
use std::io::copy;
use std::fs::File;
use std::time::Duration;
use std::env;

const HOST: &str = "localhost:1234";

fn keep_connecting(socket_addr: &SocketAddr) -> TcpStream {
    loop {
        match TcpStream::connect(socket_addr) {
            Ok(stream) => return stream,
            _ => { std::thread::sleep(Duration::from_millis(10)); },
        }
    }
}

/// Read sample file and pipe it to tcp stream
fn main() {
    let socket_addr = HOST.to_socket_addrs().unwrap().next().unwrap();
    let mut app_stream = keep_connecting(&socket_addr);
    eprintln!("CLIENT: connected");

    let sample_path = env::args().nth(1).unwrap();
    let mut sample_file = File::open(&sample_path).expect("Cannot open imputation samples file");

    // no secure channel for now
    copy(&mut sample_file, &mut app_stream).unwrap();
    eprintln!("CLIENT: finished transfering inputs");
}
