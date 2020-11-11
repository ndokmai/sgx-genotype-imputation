use bufstream::BufStream;
#[cfg(not(feature = "smac-lite"))]
use smac::*;
#[cfg(feature = "smac-lite")]
use smac_lite::*;
use std::io::BufReader;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(6)
        .build_global()
        .unwrap();

    let (host_stream, host_socket) = TcpListener::bind("localhost:7777")
        .unwrap()
        .accept()
        .unwrap();

    eprintln!("SP: accepted connection from Host at {:?}", host_socket);

    let ref_panel_reader =
        RefPanelReader::new(10, Arc::new(Mutex::new(BufReader::new(host_stream)))).unwrap();

    let (client_stream, client_socket) = TcpListener::bind("localhost:7778")
        .unwrap()
        .accept()
        .unwrap();

    eprintln!(
        "SP: accepted connection from Client at {:?}",
        client_socket
    );

    let client_stream = Arc::new(Mutex::new(BufStream::with_capacities(
        1 << 18,
        1 << 18,
        client_stream,
    )));

    let (thap_ind, thap_dat) = InputReader::new(50, client_stream.clone()).into_pair_iter();

    #[cfg(any(
        all(target_env = "sgx", target_vendor = "fortanix"),
        feature = "sim-mem-measure"
    ))]
    let cache = LocalCache;

    #[cfg(all(
        not(all(target_env = "sgx", target_vendor = "fortanix")),
        not(feature = "sim-mem-measure")
    ))]
    let cache = OffloadCache::new(100, FileCacheBackend);

    let output_writer = LazyStreamOutputWriter::new(ref_panel_reader.n_markers(), client_stream);

    eprintln!("SP: begin imputation");

    let now = std::time::Instant::now();

    smac(thap_ind, thap_dat, ref_panel_reader, cache, output_writer);

    eprintln!(
        "SP: imputation time = {} ms",
        (Instant::now() - now).as_millis()
    );

    eprintln!("SP: done");
}
