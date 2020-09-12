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
pub fn load_chunk_from_input(_chunk_id: usize, input_path: &Path) -> Array1<Input> {
    let f = File::open(input_path).expect("Unable to open input file");
    let f = BufReader::new(f);
    let x = f.lines().map(|line| {
        line.expect("Unable to read line from input file")
            .parse::<i8>()
            .expect("Parsing error in input file")
    });

    #[cfg(feature = "leak-resistant")]
    let x = x.map(|v| Input::protect(v));

    let x = x.collect::<Vec<_>>();
    Array1::from(x)
}
