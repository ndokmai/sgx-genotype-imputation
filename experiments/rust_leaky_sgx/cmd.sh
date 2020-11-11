#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
. $SCRIPT_PATH/../settings.sh

CMD=$SGX_RUN_SP
CMD_NOSGX=$SP
