if [[ $LITE -eq 1 ]]
then
    SMAC_FLAGS="--no-default-features"
    SP_FLAGS="--features smac-lite --no-default-features"
fi

if [[ $NO_SGX -ne 1 ]]
then
    SP_FLAGS="$SP_FLAGS --target x86_64-fortanix-unknown-sgx"
fi

export RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

(cd smac && cargo +nightly build --release $SMAC_FLAGS) &&
    (cd service-provider && cargo +nightly build --release $SP_FLAGS) &&

(
cd smac
cargo +nightly run -q --release $SMAC_FLAGS --bin host -- test_data/largeref.m3vcf.gz &
cd ..

cd service-provider
cargo +nightly run -q --release $SP_FLAGS &
cd ..

cd smac
cargo +nightly run -q --release $SMAC_FLAGS --bin client -- 127.0.0.1 test_data/large_input_ind.txt test_data/large_input_dat.txt output.txt
)
