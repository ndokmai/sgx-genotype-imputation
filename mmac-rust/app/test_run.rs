use mmac::cache::{EncryptedCacheBackend, OffloadCache, TcpCacheBackend};
use mmac::{impute_chunk, tcp_keep_connecting, InputReader, OwnedInput, RefPanelReader};
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use std::time::Instant;
use std::writeln;

const INPUT_IND_FILE: &'static str = "input_ind.txt";
const INPUT_DAT_FILE: &'static str = "input_dat.txt";
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

    let mut feed = Command::new("cargo")
        .arg("+nightly")
        .arg("run")
        .args(&["--bin", "ref_panel_feed"])
        .args(&args[..])
        .spawn()
        .unwrap();
    eprintln!("Main: spawn ref_ref_panel_feed");

    let mut cache_server = Command::new("cargo")
        .arg("+nightly")
        .arg("run")
        .args(&["--bin", "cache_server"])
        .args(&args[..])
        .spawn()
        .unwrap();

    eprintln!("Main: spawn cache_server");

    let chunk_id = 0;
    let input_ind_path = Path::new(INPUT_IND_FILE);
    let input_dat_path = Path::new(INPUT_DAT_FILE);

    eprintln!(
        "Main: loading chunk {} from input ({} and {})",
        chunk_id, INPUT_IND_FILE, INPUT_DAT_FILE
    );
    let now = std::time::Instant::now();
    let (thap_ind, thap_dat) = OwnedInput::load(&input_ind_path, &input_dat_path).into_pair_iter();

    eprintln!(
        "Main: input load time: {} ms",
        (Instant::now() - now).as_millis()
    );

    let stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7777").unwrap());

    eprintln!("Main: connected to ref_panel_feed");

    let ref_panel_reader = RefPanelReader::new(100, BufReader::new(stream)).unwrap();

    let now = std::time::Instant::now();
    let cache = OffloadCache::new(
        50,
        EncryptedCacheBackend::new(TcpCacheBackend::new(
            SocketAddr::from_str("127.0.0.1:8888").unwrap(),
        )),
    );

    eprintln!("Main: connected to cache_server");

    eprintln!("Main: begin imputation");

    let imputed = impute_chunk(chunk_id, thap_ind, thap_dat, ref_panel_reader, cache);

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

    let ecode = feed.wait().unwrap();
    assert!(ecode.success());

    cache_server.kill().unwrap();
}
