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

## Input data format
<!--- Add instructions --->
SMac client takes two files as user input: index file and data file. These two files together
encode a haplotype sequence that the user wishes to impute. Index file includes a binary
vector (0 or 1 in each line) indicating whether the corresponding genetic variant in the
reference panel (M3VCF file) is included in the user's data. Note that the length of this
vector should match the number of genetic variants in the reference panel. Data file includes
a vector of -1 (missing), 0 (reference allele), or 1 (alternative allele), one for each line.
These correspond to the user's data values at the nonzero indices in the index file. 
Example input files (`input_ind.txt` and `input_dat.txt`) as well as sample scripts for
generating random input can be found in `scripts/`.

SMac service provider takes the reference panel in the M3VCF format as input. This file
is provided as a gzipped file.

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

The output of SMac is a text file including the imputed alternative allele dosages at every
genetic position covered by the reference panel M3VCF file (one number per line).

## Contact for questions
Ko Dokmai, ndokmai@iu.edu

Hoon Cho, hhcho@broadinstitute.org
