#[cfg(not(feature = "smac-lite"))]
use smac::*;
#[cfg(feature = "smac-lite")]
use smac_lite::*;

#[cfg(feature = "remote-attestation")]
use ra_client::ClientRaContext;
#[cfg(feature = "remote-attestation")]
const CLIENT_PORT: u16 = 7779;
#[cfg(feature = "remote-attestation")]
use std::net::TcpListener;

use bufstream::BufStream;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;

const SP_PORT: u16 = 7777;

fn exit_print(name: &str) {
    eprintln!("Usage: {} <reference panel file>", name);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let ref_panel_file = {
        if args.len() != 2 {
            return exit_print(&args[0]);
        } else {
            eprintln!("Host: Using command line parameters: ");
            args[1].as_str()
        }
    };
    eprintln!("\tReference panel file:\t{}", ref_panel_file);

    let mut ref_panel = RefPanelWriter::new(&Path::new(&ref_panel_file));

    eprintln!("Host: n_blocks = {}", ref_panel.n_blocks());
    eprintln!("Host: n_haps = {}", ref_panel.n_haps());
    eprintln!("Host: n_markers = {}", ref_panel.n_markers());

    let mut sp_stream = BufStream::new(tcp_keep_connecting(SocketAddr::from((
        IpAddr::from_str("127.0.0.1").unwrap(),
        SP_PORT,
    ))));

    eprintln!("Host: connected to SP");

    #[cfg(feature = "remote-attestation")]
    {
        let (mut client_stream, client_socket) = TcpListener::bind(SocketAddr::from((
            IpAddr::from_str("127.0.0.1").unwrap(),
            CLIENT_PORT,
        )))
        .unwrap()
        .accept()
        .unwrap();
        eprintln!(
            "Host: accepted connection from Client at {:?}",
            client_socket
        );

        eprintln!("Host: begin remote-attestation...");
        let context = ClientRaContext::init().unwrap();
        context
            .do_attestation(&mut sp_stream, &mut client_stream)
            .unwrap();
        eprintln!("Host: remote attestation successful!");
    }

    ref_panel.write(&mut sp_stream).unwrap();

    eprintln!("Host: done");
}
