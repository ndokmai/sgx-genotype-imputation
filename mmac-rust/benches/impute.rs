use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mmac::*;
use std::path::Path;

const REF_PANEL_FILE: &'static str = "test_data/largeref.m3vcf";
const INPUT_FILE: &'static str = "test_data/large_input.txt";

pub fn impute_bench(c: &mut Criterion) {
    let chunk_id = 0;
    let ref_panel_path = Path::new(REF_PANEL_FILE);
    let input_path = Path::new(INPUT_FILE);
    let ref_panel = RefPanel::load(chunk_id, &ref_panel_path);
    let thap = load_chunk_from_input(chunk_id, &input_path);
    c.bench_function("impute test data", |b| {
        b.iter(|| {
            let cache = LocalCache;
            let ref_panel = ref_panel.clone().into_reader();
            impute_chunk(
                black_box(chunk_id),
                black_box(thap.view()),
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
