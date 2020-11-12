#[cfg(not(feature = "smac-lite"))]
use smac::*;
#[cfg(feature = "smac-lite")]
use smac_lite::*;
use std::env;
use std::io::BufWriter;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

const REF_FILE: &'static str = "../smac/test_data/largeref.m3vcf.gz";

fn exit_print(name: &str) {
    eprintln!("Usage: {} <reference panel file>", name);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut ref_panel_file = REF_FILE;
    if args.len() == 1 {
        eprintln!("Using default parameters: ");
    } else if args.len() != 2 {
        return exit_print(&args[0]);
    } else {
        eprintln!("Using command line parameters: ");
        ref_panel_file = args[1].as_str();
    }
    eprintln!("\tReference panel file:\t{}", ref_panel_file);

    let mut ref_panel = RefPanelWriter::new(&Path::new(&ref_panel_file));

    eprintln!("Host: n_blocks = {}", ref_panel.n_blocks());
    eprintln!("Host: n_haps = {}", ref_panel.n_haps());
    eprintln!("Host: n_markers = {}", ref_panel.n_markers());

    let server_stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7777").unwrap());

    eprintln!("Host: connected to Server");

    ref_panel.write(BufWriter::new(server_stream)).unwrap();

    eprintln!("Host: done");
}
