## How to run

Generate random input data based on largeref.m3vcf:
```bash
python3 gen_template.py test_data/largeref.m3vcf > template.txt
python3 gen_input.py template.txt input
```

This produces input.txt (for rust) and input.vcf (for minimac). Then run:
```bash
time cargo run --release --bin test_run
```

This writes imputed results to output.txt. Expected results from
minimac are included in output_minimac.txt.

## Unit test and benchmark
To test small inputs with `smallref.m3vcf`,
```bash
cargo test
```
To benchmark large inputs with `largeref.m3vcf`,
```bash
cargo bench
```

## Leak-resistant `feature`
To use `leak-resistant` feature, run any `cargo` commands in the following way:
```bash
FTFP_INTBITS={num_bits} cargo {...} --features leak-resistant
```

where `{num_bits}` is the number of bits `libfixtimefixpoint` uses to represent the integer portion. (Refer to Figure 11 in [this paper](https://people.eecs.berkeley.edu/~dkohlbre/papers/subnormal.pdf) for more details.) For example,
```bash
time FTFP_INTBITS=30 cargo run --release --bin test_run --features leak-resistant
```
