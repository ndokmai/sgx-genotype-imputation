#!/bin/bash
source config.sh
source common.sh

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
