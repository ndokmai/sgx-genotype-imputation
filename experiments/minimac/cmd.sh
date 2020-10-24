#!/bin/bash

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
# include global settings
source $SCRIPT_PATH/../settings.sh

CMD="$MINIMAC --refHaps $REF_PANEL_FILE --haps $MINIMAC_INPUT_FILE --format DS --noPhoneHome --nobgzip --prefix $OUTPUT --probThreshold 0 --diffThreshold 0 --topThreshold 0 --intermediate $MINIMAC_INTERMEDIATE_FILE"
