use bufstream::BufStream;
#[cfg(feature = "leak-resistant")]
use minimac_resistant::*;
#[cfg(not(feature = "leak-resistant"))]
use minimac::*;
use std::io::BufReader;
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    let (host_stream, host_socket) = TcpListener::bind("localhost:7777")
        .unwrap()
        .accept()
        .unwrap();

    eprintln!("Server: accepted connection from Host at {:?}", host_socket);

    let ref_panel_reader = RefPanelReader::new(100, BufReader::new(host_stream)).unwrap();

    let (client_stream, client_socket) = TcpListener::bind("localhost:7778")
        .unwrap()
        .accept()
        .unwrap();

    eprintln!(
        "Server: accepted connection from Client at {:?}",
        client_socket
    );

    let client_stream = Arc::new(Mutex::new(BufStream::new(client_stream)));

    let (thap_ind, thap_dat) = InputReader::new(client_stream.clone()).into_pair_iter();

    //use std::path::Path;
    //const INPUT_IND_FILE: &'static str = "input_ind.txt";
    //const INPUT_DAT_FILE: &'static str = "input_dat.txt";
    //let input_ind_path = Path::new(INPUT_IND_FILE);
    //let input_dat_path = Path::new(INPUT_DAT_FILE);
    //let (thap_ind, thap_dat) = OwnedInput::load(&input_ind_path, &input_ind_path).into_pair_iter();

    let cache = OffloadCache::new(
        100,
        EncryptedCacheBackend::new(TcpCacheBackend::new(
            SocketAddr::from_str("127.0.0.1:8888").unwrap(),
            100,
        )),
    );

    //let cache = LocalCache;

    eprintln!("Server: connected to CacheServer");

    let mut output_writer =
        LazyStreamOutputWriter::new(ref_panel_reader.n_markers(), client_stream);

    //let mut output_writer = OwnedOutputWriter::new();

    eprintln!("Server: begin imputation");

    let now = std::time::Instant::now();

    impute_all(
        thap_ind,
        thap_dat,
        ref_panel_reader,
        cache,
        &mut output_writer,
    );

    eprintln!(
        "Server: imputation time = {} ms",
        (Instant::now() - now).as_millis()
    );

    eprintln!("Server: done");
}
