#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
source $SCRIPT_PATH/../settings.sh

$HOST $REF_PANEL_FILE &

sleep 0.1

$CLIENT $RUST_INPUT_INDEX_FILE $RUST_INPUT_DATA_FILE $RUST_OUTPUT_FILE
