use minimac::*;
use std::io::BufWriter;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

const REF_FILE: &'static str = "test_data/largeref.m3vcf";

fn main() {
    eprintln!("Host: loading from reference panel ({})", REF_FILE);
    let ref_panel_path = Path::new(REF_FILE);
    let mut ref_panel = RefPanelWriter::new(&ref_panel_path);

    eprintln!("Host: n_blocks = {}", ref_panel.n_blocks());
    eprintln!("Host: n_haps = {}", ref_panel.n_haps());
    eprintln!("Host: n_markers = {}", ref_panel.n_markers());

    let server_stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7777").unwrap());

    eprintln!("Host: connected to Server");

    ref_panel.write(BufWriter::new(server_stream)).unwrap();

    eprintln!("Host: done");
}
