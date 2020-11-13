use bufstream::BufStream;
#[cfg(not(feature = "smac-lite"))]
use smac::*;
#[cfg(feature = "smac-lite")]
use smac_lite::*;

#[cfg(feature = "remote-attestation")]
mod ra {
    pub use ra_enclave::EnclaveRaContext;
    pub use sgx_crypto::random::Rng;
    pub use sgx_crypto::tls_psk::server;
}
#[cfg(feature = "remote-attestation")]
use ra::*;

use std::io::BufReader;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const HOST_PORT: u16 = 7777;
const CLIENT_PORT: u16 = 7778;

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(6)
        .build_global()
        .unwrap();

    #[allow(unused_mut)]
    let (mut host_stream, host_socket) = TcpListener::bind(SocketAddr::from((
        IpAddr::from_str("127.0.0.1").unwrap(),
        HOST_PORT,
    )))
    .unwrap()
    .accept()
    .unwrap();

    eprintln!("SP: accepted connection from Host at {:?}", host_socket);

    #[cfg(feature = "remote-attestation")]
    let mut psk_callback = {
        eprintln!("SP: begin remote-attestation...");
        let client_verification_key = include_str!("../keys/client_public_verification_key.pem");
        let client_verification_key = format!("{}\0", client_verification_key);
        let context = EnclaveRaContext::init(&client_verification_key).unwrap();
        let (_signing_key, master_key) = context.do_attestation(&mut host_stream).unwrap();
        eprintln!("SP: remote-attestation successful!");
        server::callback(&master_key)
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
    let mut rng = Rng::new();
    #[cfg(feature = "remote-attestation")]
    let config = server::config(&mut rng, &mut psk_callback);
    #[cfg(feature = "remote-attestation")]
    let mut ctx = server::context(&config).unwrap();
    #[cfg(feature = "remote-attestation")]
    let client_stream = ctx.establish(&mut client_stream, None).unwrap();

    let ref_panel_reader =
        RefPanelReader::new(10, Arc::new(Mutex::new(BufReader::new(host_stream)))).unwrap();

    let mut client_stream = BufStream::with_capacities(
        1 << 18,
        1 << 18,
        client_stream,
    );

    let (thap_ind, thap_dat) = OwnedInput::from_remote(&mut client_stream).unwrap().into_pair_iter();

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

    let output_writer = OwnedOutputWriter::new();

    let n_markers = ref_panel_reader.n_markers();
    eprintln!("SP: begin imputation");

    let now = std::time::Instant::now();

    let output = smac(thap_ind, thap_dat, ref_panel_reader, cache, output_writer);

    eprintln!(
        "SP: imputation time = {} ms",
        (Instant::now() - now).as_millis()
    );

    eprintln!("SP: writing outputs to Client ...");
    let mut output_writer = StreamOutputWriter::new(n_markers, &mut client_stream);
    for o in output.into_reader() {
        output_writer.push(o);
    }

    eprintln!("SP: done");
}
