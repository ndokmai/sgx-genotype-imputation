#!/bin/bash

if [[ $LITE -eq 1 ]]
then
    BIN_FLAGS="--features smac-lite --no-default-features"
fi

if [[ $NO_SGX -ne 1 ]]
then
    SP_FLAGS="--target x86_64-fortanix-unknown-sgx"
fi

export RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

(cd host && cargo +nightly build --release $BIN_FLAGS) &&
    (cd service-provider && cargo +nightly build --release $SP_FLAGS $BIN_FLAGS -Zfeatures=itarget) &&
    (cd client && cargo +nightly build --release $BIN_FLAGS)

(
cd host
cargo +nightly run -q --release $BIN_FLAGS -- ../smac/test_data/largeref.m3vcf.gz &
cd ..

cd service-provider
cargo +nightly run -q --release $SP_FLAGS $BIN_FLAGS -Zfeatures=itarget &
cd ..

cd client
cargo +nightly run -q --release $BIN_FLAGS -- 127.0.0.1 ../smac/test_data/large_input_ind.txt ../smac/test_data/large_input_dat.txt output.txt
)
