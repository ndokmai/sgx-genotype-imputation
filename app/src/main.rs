mod impute;
mod input_feed;
mod params;
mod symbol;

use impute::impute_single;
use input_feed::InputFeed;
use params::Params;
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::thread;
use symbol::Symbol;

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
    let runner_thread = thread::spawn(|| read_refs());
    let input_feed = get_input_feed();
    let inputs = input_feed.take(1).join().unwrap().unwrap();
    // take the first one
    let input = inputs.slice(ndarray::s![0, ..]);
    let refs = runner_thread.join().unwrap();
    let params = Params::init_test_params(&refs[..], inputs.ncols());
    eprintln!("APP: finished initializing parameters");

    eprintln!("APP: start timing imputation ...");
    let now = std::time::Instant::now();
    impute_single(input, &params);
    println!("APP: imputation takes {} ms", now.elapsed().as_millis());
    eprintln!("APP: done!");
}
