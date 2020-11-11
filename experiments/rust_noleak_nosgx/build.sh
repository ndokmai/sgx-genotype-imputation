#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
source $SCRIPT_PATH/../settings.sh

export RUSTFLAGS="$RUSTFLAGS"

cd $RUST_SMAC_DIR && cargo build --release

cd $RUST_SP_DIR && cargo build --release
