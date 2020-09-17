use mmac::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::Path;

const REF_PANEL_FILE: &'static str = "test_data/smallref.m3vcf";
const INPUT_IND_FILE: &'static str = "test_data/small_input_ind.txt";
const INPUT_DAT_FILE: &'static str = "test_data/small_input_dat.txt";

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
    let port: u16 = 9999;
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    std::thread::spawn(move || {
        TcpCacheBackend::remote_proc(port, OffloadCache::new(50, FileCacheBackend));
    });

    let chunk_id = 0;
    let ref_panel_path = Path::new(REF_PANEL_FILE);
    let input_ind_path = Path::new(INPUT_IND_FILE);
    let input_dat_path = Path::new(INPUT_DAT_FILE);
    let ref_panel = OwnedRefPanelWriter::load(chunk_id, &ref_panel_path);
    let thap_ind = load_chunk_from_input_ind(chunk_id, &input_ind_path);
    let thap_dat = load_chunk_from_input_dat(chunk_id, &input_dat_path);
    let cache = OffloadCache::new(50, EncryptedCacheBackend::new(TcpCacheBackend::new(addr)));
    let imputed = impute_chunk(
        chunk_id,
        thap_ind.view(),
        thap_dat.view(),
        ref_panel.into_reader(),
        cache,
    );
    let ref_imputed = load_ref_output();
    assert!(imputed
        .into_iter()
        .zip(ref_imputed.into_iter())
        .all(|(&a, b)| {
            let a: f32 = a.into();
            println!("a = {}", a);
            println!("b = {}", b);
            (a - b).abs() < EPSILON || (a.is_nan() && b.is_nan())
        }));
}
