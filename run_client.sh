#!/bin/bash

SP_IP=$1
INPUT_IDX=$2
INPUT_DATA=$3
OUTPUT=$4

function help_msg {
    echo "Usage: $0 <service provider ip addr> <input index file> <input data file> <output file>";
}

([[ -z "$SP_IP" ]] || [[ -z "$INPUT_IDX" ]] || [[ -z "$INPUT_DATA" ]] || [[ -z "$OUTPUT" ]]) && { help_msg ; exit 1; }

if [[ $LITE -eq 1 ]]
then
    SMAC_FLAGS="--no-default-features"
fi

export RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

(cd smac && cargo +nightly build --release $SMAC_FLAGS) &&
    # start client
    smac/target/release/client $SP_IP $INPUT_IDX $INPUT_DATA $OUTPUT
