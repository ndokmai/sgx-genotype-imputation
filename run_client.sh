#!/bin/bash

SP_IP=$1
INPUT_IDX=$2
INPUT_DATA=$3
OUTPUT=$4

function help_msg {
    echo "Usage: $0 <service provider ip addr> <input index file> <input data file> <output file>";
}

([[ -z "$SP_IP" ]] || [[ -z "$INPUT_IDX" ]] || [[ -z "$INPUT_DATA" ]] || [[ -z "$OUTPUT" ]]) && { help_msg ; exit 1; }

source config.sh
source common.sh

# start client
client/target/release/smac-client $SP_IP $INPUT_IDX $INPUT_DATA $OUTPUT
