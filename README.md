# SMac: Secure Genotype Imputation in Intel SGX
## Installation Requirements
- Ubuntu 16.04/18.04/20.04
- [Rust](https://www.rust-lang.org/tools/install)
- [Fortanix EDP](https://edp.fortanix.com/docs/installation/guide/) (for running in SGX mode)
- [Clang >= 3.8.0](https://releases.llvm.org/download.html) (for remote attestation)
    - To automatically set up clang 3.8.0 locally, run `source setup_clang.sh`

## Configuration & Build
Edit the following parameters in [config.sh](config.sh) to configure SMac:
- `LITE`: SMac or SMac-lite
    - `0`: SMac, with timing protection
    - `1`: SMac-lite, without timing protection
- `SGX`: SGX or simulation mode
    - `0`: simulation mode; no SGX hardware required
    - `1`: SGX mode; Fortanix EDP must be installed and configured properly
- `RA`: remote attestation
    - `0`: disable remote attestation 
    - `1`: enable remote attestation 
- `ENCLAVE_HEAP_SIZE`: enclave heap size in bytes; if the enclave fails while starting, try increasing this number.

Rebuild for every change in configuration. 

Next, build by running
```bash
./build.sh

```

## Quick Test
To locally test whether the code can run successfully according to the configuration, run
```bash
./test.sh
```
The script will test the code with sample data located at [smac/test_data/](smac/test_data/). The output of SMac will be saved at `output.txt`.
<!--- To test on chr20 chunk1, first follow the instruction on https://github.com/statgen/Minimac4
to install minimac4. Replace the "minimac" executable in minimac/test_chr20_mmac.sh
with the correct path. Then run the script (test_chr20_mmac.sh) which saves the output to
out/mmac/. To test leak-resilient Rust implementation of minimac, run minimac/test_chr20_rust.sh
which saves the output to out/rust/. --->

## Workflow
```
                                     +---------------------------------+
                                     |              +--------+         |
                                     |              |        |         |
                                     |              |  Host  |         |
                                     |              |        |         |
                                     |              +----|---+         |
                                     |                   |             |
                                     |   reference panel |             |
                                     |                   |             |
                         network     |   +---------------|---------+   |
+------------------+                 |   |               |         |   |
|                  |                 |   |  +------------v------+  |   |
|   +----------+   |      input      |   |  |                   |  |   |
|   |          ----------------------------->  Service Provider |  |   |
|   |  Client  |   |                 |   |  |                   |  |   |
|   |          <-----------------------------   [running SMac]  |  |   |
|   +----------+   |      output     |   |  |                   |  |   |
|                  |                 |   |  +-------------------+  |   |
+--Client Machine--+                 |   |                         |   |
                                     |   +-------SGX Enclave-------+   |
                                     |                                 |
                                     +-----------Host Machine----------+
```
## Client

### Input data format 
SMac client takes two files as user input: index file and data file. These two files together
encode a haplotype sequence that the user wishes to impute. Index file includes a binary
vector (0 or 1 in each line) indicating whether the corresponding genetic variant in the
reference panel (M3VCF file) is included in the user's data. Note that the length of this
vector should match the number of genetic variants in the reference panel. Data file includes
a vector of -1 (missing), 0 (reference allele), or 1 (alternative allele), one for each line.
These correspond to the user's data values at the nonzero indices in the index file. 
Example input files (`input_ind.txt` and `input_dat.txt`) as well as sample scripts for
generating random input can be found in [scripts/](script/).


### Running Client 

To start Client on Client Machine, run
```bash
./run_client.sh <service provider ip addr> <input index txt file> <input data txt file> <output txt file>
```
where `<input index txt file>` and `<input data txt file>` are formatted according to [input data format](#input-data-format). `<output txt file>` is the name of the output text file to be created.  For example, 

```bash
./run_client.sh 127.0.0.1 smac/test_data/large_input_ind.txt smac/test_data/large_input_dat.txt output.txt
```
## Service Provider

SMac service provider takes the reference panel in the M3VCF format as input. To start Service Provider and Host on Host Machine,

```bash
./run_sp.sh <reference panel m3vcf.gz>
```

where `<reference panel m3vcf.gz>` is a reference panel gzipped M3VCF file. For example,
```bash
./run_sp.sh smac/test_data/largeref.m3vcf.gz
```
## Output format
The output of SMac is a text file including the imputed alternative allele dosages at every
genetic position covered by the reference panel M3VCF file (one number per line).

## Contact for questions
Ko Dokmai, ndokmai@iu.edu

Hoon Cho, hhcho@broadinstitute.org
