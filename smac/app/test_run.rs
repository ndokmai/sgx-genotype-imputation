use smac::*;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use std::writeln;

const BITMASK_FILE: &'static str = "test_data/large_input_bitmask.txt";
const SYMBOLS_BATCH_DIR: &'static str = "test_data/batch";
const REF_FILE: &'static str = "test_data/largeref.m3vcf.gz";
const RESULTS_DIR: &'static str = "results";
const N_THREADS: usize = 8;

fn exit_print(name: &str) {
    eprintln!(
        "Usage: {} <reference panel file> <bitmask file> <symbols batch dir> <results dir>",
        name
    );
}

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(N_THREADS)
        .build_global()
        .unwrap();

    let args: Vec<String> = env::args().collect();
    let mut ref_panel_file = REF_FILE;
    let mut bitmask_file = BITMASK_FILE;
    let mut symbols_batch_dir = SYMBOLS_BATCH_DIR;
    let mut results_dir = RESULTS_DIR;
    if args.len() == 1 {
        eprintln!("Using default parameters: ");
    } else if args.len() == 2 && args[1].as_str() == "-h" {
        return exit_print(&args[0]);
    } else if args.len() != 5 {
        return exit_print(&args[0]);
    } else {
        eprintln!("Using command line parameters: ");
        ref_panel_file = args[1].as_str();
        bitmask_file = args[2].as_str();
        symbols_batch_dir = args[3].as_str();
        results_dir = args[4].as_str();
    }

    eprintln!("\tReference panel file:\t\t{}", ref_panel_file);
    eprintln!("\tBitmask file:\t\t\t{}", bitmask_file);
    eprintln!("\tSymbols batch directory:\t{}", symbols_batch_dir);
    eprintln!("\tResults directory:\t\t{}", results_dir);

    let (ref_panel_meta, ref_panel_block_iter) = load_ref_panel(Path::new(ref_panel_file));
    let ref_panel_blocks = ref_panel_block_iter.collect::<Vec<_>>();

    eprintln!("n_blocks = {}", ref_panel_meta.n_blocks);
    eprintln!("n_haps = {}", ref_panel_meta.n_haps);
    eprintln!("n_markers = {}", ref_panel_meta.n_markers);

    let bitmask = load_bitmask_iter(Path::new(bitmask_file)).collect::<Vec<_>>();
    let mut symbols_batch_iter = load_symbols_batch(Path::new(symbols_batch_dir));

    create_dir_all(results_dir).expect("Cannot create directory");

    let mut done = false;
    loop {
        let mut symbols_batch = Vec::with_capacity(N_THREADS);
        let mut paths = Vec::with_capacity(N_THREADS);

        for _ in 0..N_THREADS {
            if let Some((path, symbols)) = symbols_batch_iter.next() {
                symbols_batch.push(symbols.into_iter().collect::<Vec<_>>());
                paths.push(path);
            } else {
                done = true;
            }
        }
        if symbols_batch.is_empty() {
            break;
        }
        eprintln!("Processing a new batch...");
        for path in &paths {
            eprintln!("\t{}", path.to_str().unwrap());
        }

        let now = std::time::Instant::now();
        let imputed = smac_batch(&ref_panel_meta, &ref_panel_blocks, &bitmask, symbols_batch);
        eprintln!(
            "Imputation time = {} ms",
            (Instant::now() - now).as_millis()
        );
        eprintln!("Wring imputation results...");
        for (path, result) in paths.into_iter().zip(imputed.into_iter()) {
            let path = format!(
                "{}{}",
                path.file_stem().unwrap().to_str().unwrap(),
                ".result.txt"
            );
            let path = Path::new(results_dir).join(&path);
            eprintln!("\t{}", path.to_str().unwrap());
            let mut file = File::create(path).unwrap();
            writeln!(
                file,
                "{}",
                result
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            )
            .unwrap();
        }
        if done {
            break;
        }
    }
}
