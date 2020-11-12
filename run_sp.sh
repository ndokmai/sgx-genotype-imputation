#!/bin/bash

REF_PANEL=$1

function help_msg {
    echo "Usage: $0 <reference panel m3vcf.gz>";
}

[[ -z "$REF_PANEL" ]] && { help_msg ; exit 1; }

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
    (cd service-provider && cargo +nightly build --release $SP_FLAGS $BIN_FLAGS) &&
    (
    # start host
    host/target/release/smac-host $REF_PANEL &
    cd service-provider
    # start service provider
    cargo +nightly run -q --release $SP_FLAGS $BIN_FLAGS
)
