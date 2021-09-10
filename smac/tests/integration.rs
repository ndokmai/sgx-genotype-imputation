use smac::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const REF_PANEL_FILE: &'static str = "test_data/smallref.m3vcf.gz";
const BITMASK_FILE: &'static str = "test_data/small_input_bitmask.txt";
const SYMBOLS_FILE: &'static str = "test_data/small_input_symbols.txt";

#[cfg(not(feature = "leak-resistant"))]
const REF_OUTPUT_FILE: &'static str = "test_data/small_output_ref.txt";

#[cfg(feature = "leak-resistant")]
const REF_OUTPUT_FILE: &'static str = "test_data/small_output_log_ref.txt";

#[cfg(not(feature = "leak-resistant"))]
const EPSILON: f32 = 1e-5;

#[cfg(feature = "leak-resistant")]
const EPSILON: f32 = 1e-2;

fn load_ref_output() -> Vec<f32> {
    let file = BufReader::new(File::open(REF_OUTPUT_FILE).unwrap());
    file.lines()
        .map(|line| line.unwrap().parse::<f32>().unwrap())
        .collect()
}

#[test]
fn integration_test() {
    let ref_panel_path = Path::new(REF_PANEL_FILE);
    let bitmask_path = Path::new(BITMASK_FILE);
    let symbols_path = Path::new(SYMBOLS_FILE);
    let (ref_panel_meta, ref_panel_block_iter) = m3vcf::load_ref_panel(ref_panel_path);
    let ref_panel_blocks = ref_panel_block_iter
        .map(|b| b.into())
        .collect::<Vec<smac::RealBlock>>();

    let bitmask = load_bitmask_iter(bitmask_path).collect::<Vec<_>>();
    let symbols = load_symbols_iter(symbols_path).collect::<Vec<_>>();
    let imputed = smac(&ref_panel_meta, &ref_panel_blocks, &bitmask, symbols);
    let ref_imputed = load_ref_output();
    assert!(imputed
        .into_iter()
        .zip(ref_imputed.into_iter())
        .all(|(a, b)| {
            let a: f32 = a.into();
            println!("a = {}", a);
            println!("b = {}", b);
            (a - b).abs() < EPSILON || (a.is_nan() && b.is_nan())
        }));
}
