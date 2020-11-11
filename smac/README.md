## How to run

Generate random input data based on largeref.m3vcf:
```bash
python3 gen_template.py test_data/largeref.m3vcf > template.txt
python3 gen_input.py template.txt input
```

This produces input.txt (for rust) and input.vcf (for minimac). Then run:
```bash
cargo +nightly run --release --bin test_run
```

This writes imputed results to output.txt. Expected results from
minimac are included in output_minimac.txt.

## Unit test and benchmark
To test small inputs with `smallref.m3vcf`,
```bash
cargo +nightly test
```
To benchmark large inputs with `largeref.m3vcf`,
```bash
cargo +nightly bench
```

## Leak-resistant `feature`
To use `leak-resistant` feature, run any `cargo` commands in the following way:
```bash
cargo +nightly {...} --features leak-resistant
```
