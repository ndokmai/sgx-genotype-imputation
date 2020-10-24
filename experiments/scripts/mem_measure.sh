#!/bin/bash

CMD=$1
OUTPUT_FILE=$2

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )

LIGHTBLUE='\033[1;34m'
NOCOLOR='\033[0m'

echo -e "${LIGHTBLUE}==== Running Massif ====${NOCOLOR}"
valgrind --tool=massif --massif-out-file=.massif.out $CMD
$SCRIPT_PATH/find_heap_peak.py .massif.out | cat > $OUTPUT_FILE
echo -e "${LIGHTBLUE}==== Result written to $OUTPUT_FILE ====${NOCOLOR}"
rm -f .massif.out
