#!/bin/bash

FOR_TIME=$1

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
. $SCRIPT_PATH/../settings.sh

export RUSTFLAGS="$RUSTFLAGS"

cd $RUST_SMAC_DIR && cargo build --release

if [[ $FOR_TIME -eq 1 ]]
then
    cd $RUST_SP_DIR &&
        cargo build --release --target x86_64-fortanix-unknown-sgx &&
        $($ELF2SGXS)
else
    cd $RUST_SP_DIR &&
        cargo build --release --features sim-mem-measure
fi
