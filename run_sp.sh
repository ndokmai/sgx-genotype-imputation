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
if [[ $SGX -eq 1 ]]
then
    ftxsgx-runner --signature coresident $TARGET_SGX $N_THREADS &
else
    service-provider/target/release/smac-service-provider $N_THREADS &
fi
