#!/bin/bash

REF_PANEL=$1

function help_msg {
    echo "Usage: $0 <reference panel m3vcf.gz>";
}

[[ -z "$REF_PANEL" ]] && { help_msg ; exit 1; }

source config.sh
source common.sh

# start host
host/target/release/smac-host $REF_PANEL &

# start service provider
cd service-provider
cargo +nightly run -q --release $SP_FLAGS $BIN_FLAGS -Zfeatures=itarget
