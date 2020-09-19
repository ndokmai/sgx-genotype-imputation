use mmac::*;
use std::io::BufWriter;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

const REF_FILE: &'static str = "test_data/largeref.m3vcf";

fn main() {
    #[allow(unused_mut)]
    let mut args: Vec<&str> = vec![];

    #[cfg(feature = "leak-resistant")]
    args.push("--features");
    #[cfg(feature = "leak-resistant")]
    args.push("leak-resistant");

    #[cfg(not(debug_assertions))]
    args.push("--release");

    // make sure all bins are built before spawning
    Command::new("cargo")
        .arg("+nightly")
        .arg("build")
        .args(&args[..])
        .output()
        .unwrap();

    let mut server = Command::new("target/release/server").spawn().unwrap();

    eprintln!("Host: spawned Server");

    let mut cache_server = Command::new("target/release/cache_server").spawn().unwrap();

    eprintln!("Host: spawned CacheServer");

    let mut client = Command::new("target/release/client").spawn().unwrap();

    eprintln!("Host: spawned Client");

    eprintln!("Host: loading from reference panel ({})", REF_FILE);
    let ref_panel_path = Path::new(REF_FILE);
    let mut ref_panel = RefPanelWriter::new(&ref_panel_path);

    eprintln!("Host: n_blocks = {}", ref_panel.n_blocks());
    eprintln!("Host: n_haps = {}", ref_panel.n_haps());
    eprintln!("Host: n_markers = {}", ref_panel.n_markers());

    let server_stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7777").unwrap());

    eprintln!("Host: connected to Server");

    ref_panel.write(BufWriter::new(server_stream)).unwrap();

    eprintln!("Host: done sending reference panel");

    let ecode = client.wait().unwrap();
    assert!(ecode.success());

    let ecode = server.wait().unwrap();
    assert!(ecode.success());

    cache_server.kill().unwrap();

    eprintln!("Host: done");
}
