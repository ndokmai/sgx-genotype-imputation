#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
source $SCRIPT_PATH/../settings.sh

export RUSTFLAGS="$RUSTFLAGS"

cd $RUST_MINIMAC_DIR && cargo build --release --features leak-resistant

cd $RUST_SERVER_DIR && cargo build --release --features leak-resistant --no-default-features
