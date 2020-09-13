use mmac::{impute_chunk, load_chunk_from_input, RefPanelReader};
use mmac::cache::{OffloadCache, FileCacheBackend};
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use std::writeln;

const INPUT_FILE: &'static str = "input.txt";
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
        .arg("run")
        .args(&["--bin", "ref_panel_feed"])
        .args(&args[..])
        .spawn()
        .unwrap();

    let chunk_id = 0;
    let input_path = Path::new(INPUT_FILE);

    eprintln!("Loading chunk {} from input ({})", chunk_id, INPUT_FILE);
    let now = std::time::Instant::now();
    let thap = load_chunk_from_input(chunk_id, &input_path);
    eprintln!("Input load time: {} ms", (Instant::now() - now).as_millis());

    let stream = {
        let stream;
        loop {
            match TcpStream::connect("localhost:7777") {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(_) => {}
            };
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        stream.unwrap()
    };

    let bound = 50;
    let ref_panel_reader = RefPanelReader::new(bound, BufReader::new(stream)).unwrap();

    let now = std::time::Instant::now();
    let cache = OffloadCache::new(bound, FileCacheBackend);
    let imputed = impute_chunk(chunk_id, thap.view(), ref_panel_reader, cache);
    eprintln!("Imputation time: {} ms", (Instant::now() - now).as_millis());

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

    eprintln!("Imputation result written to {}", OUTPUT_FILE);

    let ecode = feed.wait().unwrap();
    assert!(ecode.success());
}
