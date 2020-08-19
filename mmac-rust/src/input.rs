use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// chunk_id represents predefined mapping from
/// integers to large genomic windows which are
/// imputed independently
/// TODO: chunk_id is currently ignored
/// and the entire toy data is loaded
pub fn load_chunk_from_input(_chunk_id: usize, input_path: &Path) -> Vec<i8> {
    //let n = 97020; // TODO: hardcoded variant count
    let f = File::open(input_path).expect("Unable to open input file");
    let f = BufReader::new(f);
    let x = f
        .lines()
        .map(|line| {
            line.expect("Unable to read line from input file")
                .parse::<i8>()
                .expect("Parsing error in input file")
        })
        .collect::<Vec<_>>();
    x
}
