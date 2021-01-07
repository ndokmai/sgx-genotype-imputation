use bufstream::BufStream;
#[cfg(not(feature = "smac-lite"))]
use smac::*;
#[cfg(feature = "smac-lite")]
use smac_lite::*;

#[cfg(feature = "remote-attestation")]
mod ra {
    use super::*;
    use ra_sp::{AttestationResult, SpConfig, SpRaContext};
    pub use sgx_crypto::random::{entropy_new, Rng};
    pub use sgx_crypto::tls_psk::client;

    const HOST_PORT: u16 = 7779;
    const CONFIG_FILE_PATH: &str = "client/settings.json";

    fn parse_config_file(path: &str) -> SpConfig {
        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap()
    }

    pub fn remote_attestation(host_ip_addr: &str) -> AttestationResult {
        let mut host_stream = tcp_keep_connecting(SocketAddr::from((
            IpAddr::from_str(host_ip_addr).unwrap(),
            HOST_PORT,
        )));
        eprintln!("Client: connected to Host");
        eprintln!("Client: begin remote attestation...");
        let config = parse_config_file(CONFIG_FILE_PATH);
        let mut entropy = entropy_new();
        let context = SpRaContext::init(config, &mut entropy).unwrap();
        let result = context.do_attestation(&mut host_stream).unwrap();
        eprintln!("Client: remote attestation successful!");
        result
    }
}
#[cfg(feature = "remote-attestation")]
use ra::*;

use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::writeln;

const SP_PORT: u16 = 7778;

fn exit_print(name: &str) {
    eprintln!(
        "Usage: {} <reference panel file> <bitmask file> <symbols batch dir> <results dir>",
        name
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (sp_ip_addr, bitmask_file, symbols_batch_dir, results_dir) = {
        if args.len() != 5 {
            return exit_print(&args[0]);
        } else {
            eprintln!("Client: Using command line parameters: ");
            (
                args[1].as_str(),
                args[2].as_str(),
                args[3].as_str(),
                args[4].as_str(),
            )
        }
    };

    eprintln!("\tService Provider IP address:\t{}", sp_ip_addr);
    eprintln!("\tService Provider port:\t\t{}", SP_PORT);
    eprintln!("\tBitmask file:\t\t\t{}", bitmask_file);
    eprintln!("\tSymbols batch directory:\t{}", symbols_batch_dir);
    eprintln!("\tResults directory:\t\t{}", results_dir);

    #[cfg(feature = "remote-attestation")]
    let ra_result = remote_attestation(sp_ip_addr);

    #[allow(unused_mut)]
    let mut sp_stream = tcp_keep_connecting(SocketAddr::from((
        IpAddr::from_str(sp_ip_addr).unwrap(),
        SP_PORT,
    )));

    eprintln!("Client: connected to SP");
    #[cfg(feature = "remote-attestation")]
    let mut entropy = entropy_new();
    #[cfg(feature = "remote-attestation")]
    let mut rng = Rng::new(&mut entropy).unwrap();
    #[cfg(feature = "remote-attestation")]
    let config = client::config(&mut rng, &ra_result.master_key).unwrap();
    #[cfg(feature = "remote-attestation")]
    let mut ctx = client::context(&config).unwrap();
    #[cfg(feature = "remote-attestation")]
    let sp_stream = ctx.establish(&mut sp_stream, None).unwrap();

    let mut sp_stream = BufStream::new(sp_stream);

    eprintln!("Client: start sending inputs");

    let bitmask = load_bitmask(Path::new(bitmask_file));
    let symbols_batch_iter = load_symbols_batch(Path::new(symbols_batch_dir));

    let mut paths = Vec::new();

    eprintln!("Client: sending bitmask file...");
    bincode::serialize_into(&mut sp_stream, &bitmask).unwrap();

    let batch_size = get_symbols_batch_size(Path::new(symbols_batch_dir));

    bincode::serialize_into(&mut sp_stream, &batch_size).unwrap();

    for (path, symbols) in symbols_batch_iter {
        eprintln!("Client: \tsending {}", path.to_str().unwrap());
        paths.push(path);
        bincode::serialize_into(&mut sp_stream, &symbols).unwrap();
    }
    sp_stream.flush().unwrap();

    eprintln!("Client: done sending inputs");

    create_dir_all(results_dir).expect("Cannot create directory");

    eprintln!("Client: waiting for results...");

    for path in paths.into_iter() {
        let path = format!(
            "{}{}",
            path.file_stem().unwrap().to_str().unwrap(),
            ".result.txt"
        );
        let path = Path::new(results_dir).join(&path);
        let result: Vec<f32> = bincode::deserialize_from(&mut sp_stream).unwrap();
        eprintln!("Client: \t writing results to {}", path.to_str().unwrap());
        let mut file = File::create(path).unwrap();
        writeln!(
            file,
            "{}",
            result
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        )
        .unwrap();
    }
    eprint!("Client: done!");
}
