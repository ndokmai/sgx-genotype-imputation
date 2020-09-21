use bufstream::BufStream;
#[cfg(not(feature = "leak-resistant"))]
use minimac::*;
#[cfg(feature = "leak-resistant")]
use minimac_resistant::*;
use std::io::BufReader;
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(5)
        .build_global()
        .unwrap();

    let (host_stream, host_socket) = TcpListener::bind("localhost:7777")
        .unwrap()
        .accept()
        .unwrap();

    eprintln!("Server: accepted connection from Host at {:?}", host_socket);

    let ref_panel_reader =
        RefPanelReader::new(10, Arc::new(Mutex::new(BufReader::new(host_stream)))).unwrap();

    let (client_stream, client_socket) = TcpListener::bind("localhost:7778")
        .unwrap()
        .accept()
        .unwrap();

    eprintln!(
        "Server: accepted connection from Client at {:?}",
        client_socket
    );

    //let client_stream = Arc::new(Mutex::new(BufStream::new(client_stream)));
    let client_stream = Arc::new(Mutex::new(BufStream::with_capacities(
        1 << 18,
        1 << 18,
        client_stream,
    )));

    let (thap_ind, thap_dat) = InputReader::new(50, client_stream.clone()).into_pair_iter();
    //let (thap_ind, thap_dat) = OwnedInput::from_remote(&mut *client_stream.lock().unwrap())
    //.unwrap()
    //.into_pair_iter();

    let cache = OffloadCache::new(
        100,
        EncryptedCacheBackend::new(TcpCacheBackend::with_capacities(
            1 << 18,
            1 << 18,
            SocketAddr::from_str("127.0.0.1:8888").unwrap(),
        )),
    );

    //let cache = OffloadCache::new(10, EncryptedCacheBackend::new(NonEnclaveLocalCacheBackend));

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
