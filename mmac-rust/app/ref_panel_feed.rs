use mmac::{RefPanel, RefPanelWrite, RefPanelWriter};
use std::io::BufWriter;
use std::net::TcpListener;
use std::path::Path;
use std::time::Instant;

const REF_FILE: &'static str = "test_data/largeref.m3vcf";

fn main() {
    let chunk_id = 0;
    let ref_panel_path = Path::new(REF_FILE);

    eprintln!(
        "Loading chunk {} from reference panel ({})",
        chunk_id, REF_FILE
    );

    let now = std::time::Instant::now();
    let mut ref_panel = RefPanelWriter::new(chunk_id, &ref_panel_path);

    eprintln!(
        "Reference panel load time: {} ms",
        (Instant::now() - now).as_millis()
    );

    eprintln!("n_blocks = {}", ref_panel.n_blocks());
    eprintln!("n_haps = {}", ref_panel.n_haps());
    eprintln!("n_markers = {}", ref_panel.n_markers());

    let stream = TcpListener::bind("localhost:7777")
        .unwrap()
        .accept()
        .unwrap()
        .0;

    ref_panel.write(BufWriter::new(stream)).unwrap();
}
