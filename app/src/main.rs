mod params;
mod symbol;
mod impute;
mod input_feed;

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::thread;
use ndarray::s;
use params::Params;
use symbol::Symbol;
use impute::impute;
use input_feed::InputFeed;

const RUNNER_HOST: &str = "127.0.0.1:7777";
const CLIENT_HOST: &str = "127.0.0.1:1234";


fn read_refs() -> Vec<Vec<Symbol>> {
    let socket_addr = RUNNER_HOST.to_socket_addrs().unwrap().next().unwrap();
    let listener = TcpListener::bind(&socket_addr).unwrap();
    let runner_stream = listener.accept().unwrap().0;
    eprintln!("APP: accepted RUNNER");
    let runner_stream = BufReader::new(runner_stream);

    // It's safer to parse early to make sure the inputs aren't malformed
    let refs = runner_stream 
        .lines()
        .map(|l| {
            l.unwrap()
                .chars()
                .map(|c| Symbol::parse(&c).unwrap())
                .collect::<Vec<Symbol>>()
        })
        .collect::<Vec<_>>();
    eprintln!("APP: finished reading reference panels");
    refs
}

fn get_input_feed() -> InputFeed<TcpStream> {
    let socket_addr = CLIENT_HOST.to_socket_addrs().unwrap().next().unwrap();
    let listener = TcpListener::bind(&socket_addr).unwrap();
    let client_stream = listener.accept().expect("Cannot accept client").0;
    eprintln!("APP: accepted CLIENT");
    let client_stream = BufReader::new(client_stream);
    InputFeed::new(client_stream)
}

fn main() {
    let runner_thread = thread::spawn(|| read_refs() );
    let input_feed = get_input_feed();
    // try taking the first one
    let input = input_feed.take(1).join().unwrap().unwrap();
    let refs = runner_thread.join().unwrap();
    let params = Params::init(&refs[..], input.len());
    eprintln!("APP: finished initializing parameters");
    let now = std::time::Instant::now();
    impute(input.slice(s![0, ..]).as_slice().unwrap(), &params);
    println!("APP: imputation takes {} ms", now.elapsed().as_millis());
    eprintln!("APP: done!");
}
