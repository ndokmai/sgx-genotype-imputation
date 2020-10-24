#!/bin/bash

FOR_TIME=$1

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
. $SCRIPT_PATH/../settings.sh

export RUSTFLAGS="$RUSTFLAGS"

cd $RUST_MINIMAC_DIR && cargo build --release --features leak-resistant

if [[ $FOR_TIME -eq 1 ]]
then
    cd $RUST_SERVER_DIR &&
        cargo build --release --target x86_64-fortanix-unknown-sgx --features leak-resistant --no-default-features &&
        $($ELF2SGXS)
else
    cd $RUST_SERVER_DIR &&
        cargo build --release --features "sim-mem-measure leak-resistant" --no-default-features
fi
