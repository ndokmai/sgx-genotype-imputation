use crate::Input;
use ndarray::Array1;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// chunk_id represents predefined mapping from
/// integers to large genomic windows which are
/// imputed independently
/// TODO: chunk_id is currently ignored
/// and the entire toy data is loaded
pub fn load_chunk_from_input_ind(_chunk_id: usize, input_path: &Path) -> Array1<i8> {
    let x = load_vector(input_path);
    Array1::from(x)
}

pub fn load_chunk_from_input_dat(_chunk_id: usize, input_path: &Path) -> Array1<Input> {
    let x = load_vector(input_path);

    #[cfg(feature = "leak-resistant")]
    let x = x.iter().map(|v| Input::protect(*v)).collect::<Vec<_>>();

    Array1::from(x)
}

fn load_vector(input_path: &Path) -> Vec<i8> {
    let f = File::open(input_path).expect("Unable to open input file");
    let f = BufReader::new(f);
    let x = f.lines().map(|line| {
        line.expect("Unable to read line from input file")
            .parse::<i8>()
            .expect("Parsing error in input file")
    });
    x.collect::<Vec<_>>()
}
