#!/bin/bash

SP_IP=$1
BITMASK_FILE=$2
SYMBOLS_BATCH_DIR=$3
RESULTS_DIR=$4

function help_msg {
    echo "Usage: $0 <service provider ip addr> <bitmask file> <symbols batch dir> <results dir>";
}

([[ -z "$SP_IP" ]] || [[ -z "$BITMASK_FILE" ]] || [[ -z "$SYMBOLS_BATCH_DIR" ]] || [[ -z "$RESULTS_DIR" ]]) && { help_msg ; exit 1; }

source config.sh
source common.sh

# start client
client/target/release/smac-client $SP_IP $BITMASK_FILE $SYMBOLS_BATCH_DIR $RESULTS_DIR
