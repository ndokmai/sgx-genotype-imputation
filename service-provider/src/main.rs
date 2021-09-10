use bufstream::BufStream;
#[cfg(not(feature = "smac-lite"))]
use smac::*;
#[cfg(feature = "smac-lite")]
use smac_lite::*;

#[cfg(feature = "remote-attestation")]
mod ra {
    pub use ra_enclave::EnclaveRaContext;
    pub use sgx_crypto::tls_psk::server;
}
#[cfg(feature = "remote-attestation")]
use ra::*;

use std::io::Write;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::time::Instant;

const HOST_PORT: u16 = 7777;
const CLIENT_PORT: u16 = 7778;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num_threads = args[1].parse::<usize>().unwrap();
    println!("args: {:?}", args);
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    let (host_stream, host_socket) = TcpListener::bind(SocketAddr::from((
        IpAddr::from_str("127.0.0.1").unwrap(),
        HOST_PORT,
    )))
    .unwrap()
    .accept()
    .unwrap();

    let mut host_stream = BufStream::new(host_stream);

    eprintln!("SP: accepted connection from Host at {:?}", host_socket);

    #[cfg(feature = "remote-attestation")]
    let mut context = {
        eprintln!("SP: begin remote-attestation...");
        let client_verification_key = include_str!("../keys/client_public_verification_key.pem");
        let client_verification_key = format!("{}\0", client_verification_key);
        let context = EnclaveRaContext::init(&client_verification_key).unwrap();
        let (_signing_key, master_key) = context.do_attestation(&mut host_stream).unwrap();
        eprintln!("SP: remote-attestation successful!");
        server::ServerTlsPskContext::new(master_key)
    };

    #[allow(unused_mut)]
    let (mut client_stream, client_socket) = TcpListener::bind(SocketAddr::from((
        IpAddr::from_str("127.0.0.1").unwrap(),
        CLIENT_PORT,
    )))
    .unwrap()
    .accept()
    .unwrap();

    eprintln!("SP: accepted connection from Client at {:?}", client_socket);

    #[cfg(feature = "remote-attestation")]
    let client_stream = context.establish(&mut client_stream, None).unwrap();

    let mut client_stream = BufStream::new(client_stream);

    eprintln!("SP: receiving reference panel from Host...");
    let ref_panel_meta: m3vcf::RefPanelMeta = bincode::deserialize_from(&mut host_stream).unwrap();
    let ref_panel_blocks: Vec<m3vcf::Block> = bincode::deserialize_from(&mut host_stream).unwrap();
    let ref_panel_blocks = ref_panel_blocks
        .into_iter()
        .map(|b| b.into())
        .collect::<Vec<RealBlock>>();

    eprintln!("SP: receiving bitmask from Client...");
    let bitmask: Bitmask = bincode::deserialize_from(&mut client_stream).unwrap();
    let bitmask = bitmask.into_iter().collect::<Vec<_>>();

    eprintln!("SP: begin imputation");
    let mut batch_size: usize = bincode::deserialize_from(&mut client_stream).unwrap();

    let (symbols_send, symbols_recv) = channel();
    let (results_send, results_recv) = channel();

    std::thread::spawn(move || loop {
        if let Ok(symbols_batch) = symbols_recv.recv() {
            let now = std::time::Instant::now();
            let results = smac_batch(&ref_panel_meta, &ref_panel_blocks, &bitmask, symbols_batch);
            eprintln!(
                "SP: \timputation time = {} ms",
                (Instant::now() - now).as_millis()
            );
            results_send.send(results).unwrap();
        } else {
            break;
        }
    });

    let n_threads = rayon::current_num_threads();

    while batch_size > 0 {
        let n = usize::min(batch_size, n_threads);
        batch_size -= n;
        let mut symbols_batch = Vec::with_capacity(n);
        eprintln!("SP: \tprocessing a new batch of size {}", n);
        for _ in 0..n {
            let symbols: SymbolVec = bincode::deserialize_from(&mut client_stream).unwrap();
            symbols_batch.push(symbols.into_iter().collect::<Vec<_>>());
        }
        symbols_send.send(symbols_batch).unwrap();
        let results = results_recv.recv().unwrap();
        eprintln!("SP: \twriting results to Client...");
        for result in results.into_iter() {
            bincode::serialize_into(&mut client_stream, &result).unwrap();
        }
        client_stream.flush().unwrap();
    }
    eprintln!("SP: done");
}
