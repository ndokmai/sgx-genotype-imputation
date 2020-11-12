# SMac: Genotype Imputation in Intel SGX
## Installation Requirements
- Ubuntu 16.04/18.04/20.04
- [Rust](https://www.rust-lang.org/tools/install)
- [Fortanix EDP](https://edp.fortanix.com/docs/installation/guide/) (for running in SGX mode)

## Configuration
Edit the following parameters in [config.sh](config.sh) to configure SMac:
- `LITE`: SMac or SMac-lite
    - `0`: SMac, with timing protection
    - `1`: SMac-lite, without timing protection
- `SGX`: SGX or simulation mode
    - `0`: simulation mode; not SGX hardware required
    - `1`: SGX mode; Fortanix EDP must be installed and configured properly

## Quick Test
```bash
./test.sh
```
<!--- To test on chr20 chunk1, first follow the instruction on https://github.com/statgen/Minimac4
to install minimac4. Replace the "minimac" executable in minimac/test_chr20_mmac.sh
with the correct path. Then run the script (test_chr20_mmac.sh) which saves the output to
out/mmac/. To test leak-resilient Rust implementation of minimac, run minimac/test_chr20_rust.sh
which saves the output to out/rust/. --->

## Input data processing
<!--- Add instructions --->

## Client
Build Client by running
```bash
./build_client.sh
```

To start Client,
```bash
./run_client.sh <service provider ip addr> <input index file> <input data file> <output file>
```
For example, 

```bash
./run_client.sh 127.0.0.1 smac/test_data/large_input_ind.txt smac/test_data/large_input_dat.txt output.txt
```
## Service Provider
Build Service Provider by running
```bash
./build_sp.sh
```

To start Service Provider,

```bash
./run_sp.sh <reference panel m3vcf.gz>
```

For example,
```bash
./run_sp.sh smac/test_data/largeref.m3vcf.gz
```
## Output format
<!--- Add explanation --->


