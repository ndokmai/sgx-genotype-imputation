use minimac::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use std::writeln;
use std::env;

//const INPUT_IND_FILE: &'static str = "test_data/large_input_ind.txt";
//const INPUT_DAT_FILE: &'static str = "test_data/large_input_dat.txt";
//const REF_FILE: &'static str = "test_data/largeref.m3vcf";
//const OUTPUT_FILE: &'static str = "output.txt";

fn main() {
    let args: Vec<String> = env::args().collect();
    let ref_panel_file = &args[1];
    let ind_file = &args[2];
    let dat_file = &args[3];
    let output_file = &args[4];

    let ref_panel = OwnedRefPanelWriter::load(&Path::new(&ref_panel_file)).into_reader();
    let (thap_ind, thap_data) =
        OwnedInput::load(&Path::new(ind_file), &Path::new(dat_file)).into_pair_iter();
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
