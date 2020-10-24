#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )

source $SCRIPT_PATH/cmd.sh
source $SCRIPT_PATH/../settings.sh

$SCRIPT_PATH/build.sh 0 || exit 1

$SCRIPT_PATH/setup.sh &

sleep 0.1

$SCRIPT_PATH/../scripts/mem_measure.sh "$CMD_NOSGX" $MEM_OUTPUT
