# sgx-genotype-imputation
## Requirements
- Ubuntu 16.04/18.04
- [Rust](https://www.rust-lang.org/tools/install)
- [Fortanix EDP](https://edp.fortanix.com/docs/installation/guide/)

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

