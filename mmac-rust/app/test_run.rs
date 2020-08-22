use mmac::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use std::writeln;

const REF_FILE: &'static str = "largeref.m3vcf";
const INPUT_FILE: &'static str = "input.txt";
const OUTPUT_FILE: &'static str = "output.txt";

fn main() {
    let chunk_id = 0;
    let ref_panel_path = Path::new(REF_FILE);
    let input_path = Path::new(INPUT_FILE);

    eprintln!(
        "Loading chunk {} from reference panel ({})",
        chunk_id, REF_FILE
    );

    let now = std::time::Instant::now();
    let ref_panel = RefPanel::load(chunk_id, &ref_panel_path);
    eprintln!(
        "Reference panel load time: {} ms",
        (Instant::now() - now).as_millis()
    );

    eprintln!("n_blocks = {}", ref_panel.blocks.len());
    eprintln!("n_haps = {}", ref_panel.n_haps);
    eprintln!("n_markers = {}", ref_panel.n_markers);

    eprintln!("Loading chunk {} from input ({})", chunk_id, INPUT_FILE);
    let now = std::time::Instant::now();
    let thap = load_chunk_from_input(chunk_id, &input_path);
    eprintln!("Input load time: {} ms", (Instant::now() - now).as_millis());

    let now = std::time::Instant::now();
    let imputed = impute_chunk(chunk_id, thap.view(), &ref_panel);
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
}
