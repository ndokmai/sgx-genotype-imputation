use mmac::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const REF_PANEL_FILE: &'static str = "test_data/smallref.m3vcf";
const INPUT_FILE: &'static str = "test_data/small_input.txt";
const REF_OUTPUT_FILE: &'static str = "test_data/small_output_ref.txt";

#[cfg(not(feature = "leak-resistant"))]
const EPSILON: f64 = f64::EPSILON;

#[cfg(feature = "leak-resistant")]
const EPSILON: f64 = 1e-4;

fn load_ref_output() -> Vec<f64> {
    let file = BufReader::new(File::open(REF_OUTPUT_FILE).unwrap());
    file.lines()
        .map(|line| line.unwrap().parse::<f64>().unwrap())
        .collect()
}

#[test]
fn integration_test() {
    let chunk_id = 0;
    let ref_panel_path = Path::new(REF_PANEL_FILE);
    let input_path = Path::new(INPUT_FILE);
    let ref_panel = RefPanel::load(chunk_id, &ref_panel_path);
    let thap = load_chunk_from_input(chunk_id, &input_path);
    let imputed = impute_chunk(chunk_id, thap.view(), &ref_panel);
    let ref_imputed = load_ref_output();
    assert!(imputed
        .into_iter()
        .zip(ref_imputed.into_iter())
        .all(|(&a, b)| {
            let a: f64 = a.into();
            println!("a = {}", a);
            println!("b = {}", b);
            (a - b).abs() < EPSILON || (a.is_nan() && b.is_nan())
        }));
}