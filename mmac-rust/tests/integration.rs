use mmac::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::Path;
use std::thread::spawn;

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
    spawn(move || {
        TcpCacheBackend::remote_proc(port, OffloadCache::new(50, FileCacheBackend));
    });

    let ref_panel_path = Path::new(REF_PANEL_FILE);
    let input_ind_path = Path::new(INPUT_IND_FILE);
    let input_dat_path = Path::new(INPUT_DAT_FILE);

    // ref panel
    let (ref_panel_stream1, mut ref_panel_stream2) = pipe::pipe();
    spawn(move || {
        let mut ref_panel_writer = RefPanelWriter::new(&ref_panel_path);
        ref_panel_writer.write(&mut ref_panel_stream2).unwrap();
    });
    let ref_panel_reader = RefPanelReader::new(50, ref_panel_stream1).unwrap();

    // input
    let (input_stream1, mut input_stream2) = pipe::pipe();
    spawn(move || {
        let mut input_writer = InputWriter::new(&input_ind_path, &input_dat_path);
        input_writer.write(&mut input_stream2).unwrap();
    });

    let (thap_ind, thap_dat) = InputReader::new(100, input_stream1).into_pair_iter();
    let cache = OffloadCache::new(50, EncryptedCacheBackend::new(TcpCacheBackend::new(addr)));
    let mut output_writer = OwnedOutputWriter::new();
    impute_all(
        thap_ind,
        thap_dat,
        ref_panel_reader,
        cache,
        &mut output_writer,
    );
    let imputed = output_writer.into_reader().collect::<Vec<_>>();
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
