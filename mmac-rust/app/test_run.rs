use mmac::*;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::process::Command;
use std::str::FromStr;
use std::time::Instant;
use std::writeln;

const OUTPUT_FILE: &'static str = "output.txt";

fn main() {
    #[allow(unused_mut)]
    let mut args: Vec<&str> = vec![];

    #[cfg(feature = "leak-resistant")]
    args.push("--features");
    #[cfg(feature = "leak-resistant")]
    args.push("leak-resistant");

    #[cfg(not(debug_assertions))]
    args.push("--release");

    eprintln!("Main: spawn ref_panel_feed");
    let mut ref_panel_feed = Command::new("cargo")
        .arg("+nightly")
        .arg("-q")
        .arg("run")
        .args(&["--bin", "ref_panel_feed"])
        .args(&args[..])
        .spawn()
        .unwrap();


    eprintln!("Main: spawn cache_server");
    let mut cache_server = Command::new("cargo")
        .arg("+nightly")
        .arg("-q")
        .arg("run")
        .args(&["--bin", "cache_server"])
        .args(&args[..])
        .spawn()
        .unwrap();


    eprintln!("Main: spawn input_feed");

    let mut input_feed = Command::new("cargo")
        .arg("+nightly")
        .arg("-q")
        .arg("run")
        .args(&["--bin", "input_feed"])
        .args(&args[..])
        .spawn()
        .unwrap();

    let ref_panel_stream = TcpListener::bind("localhost:7777")
        .unwrap()
        .accept()
        .unwrap()
        .0;

    eprintln!("Main: connected to ref_panel_feed");

    let ref_panel_reader = RefPanelReader::new(100, BufReader::new(ref_panel_stream)).unwrap();

    let input_stream = TcpListener::bind("localhost:7778")
        .unwrap()
        .accept()
        .unwrap()
        .0;

    eprintln!("Main: connected to input_feed");

    let (thap_ind, thap_dat) = InputReader::new(1000, input_stream).into_pair_iter();

    let cache = OffloadCache::new(
        50,
        EncryptedCacheBackend::new(TcpCacheBackend::new(
            SocketAddr::from_str("127.0.0.1:8888").unwrap(),
        )),
    );

    eprintln!("Main: connected to cache_server");

    eprintln!("Main: begin imputation");

    let now = std::time::Instant::now();

    let imputed = impute_all(thap_ind, thap_dat, ref_panel_reader, cache);

    eprintln!(
        "Main: imputation time: {} ms",
        (Instant::now() - now).as_millis()
    );

    let mut file = File::create(OUTPUT_FILE).unwrap();
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

    eprintln!("Main: imputation result written to {}", OUTPUT_FILE);

    let ecode = input_feed.wait().unwrap();
    assert!(ecode.success());

    let ecode = ref_panel_feed.wait().unwrap();
    assert!(ecode.success());

    cache_server.kill().unwrap();
}
