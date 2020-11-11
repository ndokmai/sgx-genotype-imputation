use std::net::{SocketAddr, TcpStream};

pub fn tcp_keep_connecting(addr: SocketAddr) -> TcpStream {
    let stream;
    loop {
        if let Ok(s) = TcpStream::connect(addr) {
            stream = Some(s);
            break;
        };
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    stream.unwrap()
}
