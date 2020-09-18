use mmac::*;
use std::io::BufWriter;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

const REF_FILE: &'static str = "test_data/largeref.m3vcf";

fn main() {
    let ref_panel_path = Path::new(REF_FILE);

    eprintln!("RefPanelFeed: loading from reference panel ({})", REF_FILE);
    let mut ref_panel = RefPanelWriter::new(&ref_panel_path);

    eprintln!("RefPanelFeed: n_blocks = {}", ref_panel.n_blocks());
    eprintln!("RefPanelFeed: n_haps = {}", ref_panel.n_haps());
    eprintln!("RefPanelFeed: n_markers = {}", ref_panel.n_markers());

    let stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7777").unwrap());
    eprintln!("RefPanelFeed: connected to Main");

    ref_panel.write(BufWriter::new(stream)).unwrap();
    eprintln!("RefPanelFeed: done")
}
