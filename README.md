# SMac: Genotype Imputation in Intel SGX
## Requirements
- Ubuntu 16.04/18.04
- [Rust](https://www.rust-lang.org/tools/install)
- [Fortanix EDP](https://edp.fortanix.com/docs/installation/guide/) (for running in SGX mode)

## Quick Test
```bash
./test.sh
```

To test without SGX (simulation mode), set the environment variable `NO_SGX=1`.

To test `smac-lite` (no timing protection), set the environment variable `LITE=1`.

<!--- To test on chr20 chunk1, first follow the instruction on https://github.com/statgen/Minimac4
to install minimac4. Replace the "minimac" executable in minimac/test_chr20_mmac.sh
with the correct path. Then run the script (test_chr20_mmac.sh) which saves the output to
out/mmac/. To test leak-resilient Rust implementation of minimac, run minimac/test_chr20_rust.sh
which saves the output to out/rust/. --->

## Input data processing
<!--- Add instructions --->

## Client

```bash
./run_client.sh <service provider ip addr> <input index file> <input data file> <output file>
```
For example, 

```bash
./run_client.sh 127.0.0.1 smac/test_data/large_input_ind.txt smac/test_data/large_input_dat.txt output.txt
```

To run `smac-lite` (no timing protection), set the environment variable `LITE=1`.

## Service Provider

```bash
./run_sp.sh <reference panel m3vcf.gz>
```

For example,
```bash
./run_sp.sh smac/test_data/largeref.m3vcf.gz
```

To run without SGX (simulation mode), set the environment variable `NO_SGX=1`.

To run `smac-lite` (no timing protection), set the environment variable `LITE=1`.

## Output format
<!--- Add explanation --->
