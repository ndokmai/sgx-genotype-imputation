#!/bin/bash

REF_PANEL=$1

function help_msg {
    echo "Usage: $0 <reference panel m3vcf.gz>";
}

[[ -z "$REF_PANEL" ]] && { help_msg ; exit 1; }

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
    # start host
    smac/target/release/host $REF_PANEL &
    cd service-provider
    # start service provider
    cargo +nightly run -q --release $SP_FLAGS
)
