#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )

source $SCRIPT_PATH/cmd.sh
source $SCRIPT_PATH/../settings.sh

$SCRIPT_PATH/build.sh 1 || exit 1

$SCRIPT_PATH/../scripts/time.sh $N_TIMES "$CMD" $SCRIPT_PATH/setup.sh $SCRIPT_PATH/cleanup.sh $TIME_OUTPUT
