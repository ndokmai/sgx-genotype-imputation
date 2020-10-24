#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
source $SCRIPT_PATH/../settings.sh

CMD=$RUST_SERVER_DIR/target/release/server
