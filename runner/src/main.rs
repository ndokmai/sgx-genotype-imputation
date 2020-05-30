use std::fs::File;
use std::process::Command;
use std::time::Duration;
use std::io::copy;
use std::net::{TcpStream, ToSocketAddrs, SocketAddr, Shutdown};

const HOST: &str = "localhost:7777";

fn usage(name: &str) {
    println!("Usage:\n{} <path_to_sgxs_file> <path_to_ref_panels_file>", name);
}

fn parse_args() -> Result<(String, String), ()> {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        3 => {
            Ok((args[1].to_owned(), 
             args[2].to_owned()))
        },
        _ => {
            usage(&args[0]);
            Err(())
        }
    }
}

fn keep_connecting(socket_addr: &SocketAddr) -> TcpStream {
    loop {
        match TcpStream::connect(socket_addr) {
            Ok(stream) => return stream,
            _ => { std::thread::sleep(Duration::from_millis(10)); },
        }
    }
}

fn main() {
    let (sgx_file_path, refs_file_path) = parse_args().unwrap();
    let mut refs_file = File::open(&refs_file_path)
        .expect("Cannot open reference panels file.");

    let mut child =  
        if cfg!(feature = "simulation") {
            Command::new(&sgx_file_path)
                .spawn()
                .unwrap()
        } else {
            Command::new("ftxsgx-runner")
                .arg(&sgx_file_path)
                .spawn()
                .unwrap()
        };
        

    let socket_addr = HOST.to_socket_addrs().unwrap().next().unwrap();
    let mut app_stream = keep_connecting(&socket_addr);
    eprintln!("RUNNER: connected");

    // no compression for now
    copy(&mut refs_file, &mut app_stream).unwrap();
    app_stream.shutdown(Shutdown::Both).unwrap();
    eprintln!("RUNNER: finished transfering reference panels");
    
    let ecode = child.wait()
        .expect("failed to wait on child");
    assert!(ecode.success());
}
