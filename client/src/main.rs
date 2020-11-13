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
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::writeln;

const SP_PORT: u16 = 7778;

fn exit_print(name: &str) {
    eprintln!(
        "Usage: {} <service provider ip addr> <index input file> <data input file> <output file>",
        name
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (sp_ip_addr, ind_file, dat_file, output_file) = {
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
    eprintln!("\tInput index file:\t\t{}", ind_file);
    eprintln!("\tInput data file:\t\t{}", dat_file);
    eprintln!("\tOutput file:\t\t\t{}", output_file);

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

    let n_ind = BufReader::new(File::open(ind_file).unwrap())
        .lines()
        .count();

    let mut input_writer = InputWriter::new(n_ind, &Path::new(ind_file), &Path::new(dat_file));
    input_writer.write(&mut sp_stream).unwrap();
    sp_stream.flush().unwrap();

    eprintln!("Client: done sending inputs");
    eprintln!("Client: reading outputs from SP...");

    let imputed = StreamOutputReader::read(sp_stream).collect::<Vec<Real>>();

    let mut file = File::create(output_file).unwrap();
    writeln!(
        file,
        "{}",
        imputed
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    )
    .unwrap();

    eprintln!("Client: imputation result written to {}", output_file);

    eprintln!("Client: done");
}
