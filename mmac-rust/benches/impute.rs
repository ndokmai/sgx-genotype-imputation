use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mmac::*;
use std::path::Path;

const REF_PANEL_FILE: &'static str = "test_data/largeref.m3vcf";
const INPUT_IND_FILE: &'static str = "test_data/large_input_ind.txt";
const INPUT_DAT_FILE: &'static str = "test_data/large_input_dat.txt";

pub fn impute_bench(c: &mut Criterion) {
    let ref_panel_path = Path::new(REF_PANEL_FILE);
    let input_ind_path = Path::new(INPUT_IND_FILE);
    let input_dat_path = Path::new(INPUT_DAT_FILE);
    let ref_panel = OwnedRefPanelWriter::load(&ref_panel_path);
    let input = OwnedInput::load(&input_ind_path, &input_dat_path);
    c.bench_function("impute test data", |b| {
        b.iter(|| {
            let cache = LocalCache;
            let ref_panel = ref_panel.clone().into_reader();
            let (thap_ind, thap_dat) = input.clone().into_pair_iter();
            impute_all(
                black_box(thap_ind),
                black_box(thap_dat),
                black_box(ref_panel),
                black_box(cache),
            )
        })
    });
}
criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(10))
        .sample_size(20);
    targets = impute_bench
}
criterion_main!(benches);
