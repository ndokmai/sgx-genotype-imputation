use minimac::*;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use std::writeln;

const INPUT_IND_FILE: &'static str = "test_data/large_input_ind.txt";
const INPUT_DAT_FILE: &'static str = "test_data/large_input_dat.txt";
const REF_FILE: &'static str = "test_data/largeref.m3vcf";
const OUTPUT_FILE: &'static str = "output.txt";

fn exit_print(name: &str) {
    eprintln!(
        "Usage: {} <reference panel file> <index input file> <data input file> <output file>",
        name
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut ref_panel_file = REF_FILE;
    let mut ind_file = INPUT_IND_FILE;
    let mut dat_file = INPUT_DAT_FILE;
    let mut output_file = OUTPUT_FILE;
    if args.len() == 1 {
        eprintln!("Using default parameters: ");
    } else if args.len() == 2 && args[1].as_str() == "-h" {
        return exit_print(&args[0]);
    } else if args.len() != 5 {
        return exit_print(&args[0]);
    } else {
        eprintln!("Using command line parameters: ");
        ref_panel_file = args[1].as_str();
        ind_file = args[2].as_str();
        dat_file = args[3].as_str();
        output_file = args[4].as_str();
    }

    eprintln!("\tReference panel file:\t{}", ref_panel_file);
    eprintln!("\tInput index file:\t{}", ind_file);
    eprintln!("\tInput data file:\t{}", dat_file);
    eprintln!("\tOutput file:\t\t{}", output_file);

    let ref_panel = OwnedRefPanelWriter::load(&Path::new(&ref_panel_file)).into_reader();
    let (thap_ind, thap_data) =
        OwnedInput::load(&Path::new(ind_file), &Path::new(dat_file)).into_pair_iter();

    eprintln!("n_blocks = {}", ref_panel.n_blocks());
    eprintln!("n_haps = {}", ref_panel.n_haps());
    eprintln!("n_markers = {}", ref_panel.n_markers());

    let mut output_writer = OwnedOutputWriter::new();

    let cache = LocalCache;

    let now = std::time::Instant::now();
    impute_all(thap_ind, thap_data, ref_panel, cache, &mut output_writer);
    eprintln!(
        "Imputation time = {} ms",
        (Instant::now() - now).as_millis()
    );

    let imputed = output_writer.into_reader().collect::<Vec<_>>();

    let mut file = File::create(output_file).unwrap();

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

    eprintln!("Imputation result written to {}", output_file);
}
