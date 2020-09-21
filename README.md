# sgx-genotype-imputation
## Test
```bash
./test.sh
```

To test in simulation mode, set the environment variable `SIM=1`.

To enable the `leak-resisant` feature, set the environment variable `LEAK_RESISTANT=1`.

To test on chr20 chunk1, first follow the instruction on https://github.com/statgen/Minimac4
to install minimac4. Replace the "minimac" executable in minimac/test_chr20_mmac.sh
with the correct path. Then run the script (test_chr20_mmac.sh) which saves the output to
out/mmac/. To test leak-resilient Rust implementation of minimac, run minimac/test_chr20_rust.sh
which saves the output to out/rust/.
