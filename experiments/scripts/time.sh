#!/bin/bash

n=$1
CMD=$2
SETUP=$3
CLEANUP=$4
OUTPUT_FILE=$5

SCRIPT_PATH=$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )
LIGHTBLUE='\033[1;34m'
NOCOLOR='\033[0m'
RE='^[0-9]+$'
sum=0
impute_sum=0
impute_time_check=false
set -o pipefail

function parse_imputation_time {
    awk '$NF=="ms" {print $(NF-1)}' .std.out
}

function cleanup {
    rm -f .time.out .std.out
    set +o pipefail
    $CLEANUP
}

function fait_exit {
    cleanup
    exit 1
}

for (( i=1; i<=$n; i++ ))
do
    echo -e "${LIGHTBLUE}==== Round $i ====${NOCOLOR}"
    $SETUP &
    sleep 0.1
    /usr/bin/time -o .time.out -f "%e" $CMD 2>&1 | tee .std.out
    [ "$?" -eq "0" ] || fait_exit
    sum=$(echo $sum + $(cat .time.out) | bc)
    impute_time=$(parse_imputation_time)
    if [[ $impute_time =~ $RE ]] ; then
        impute_sum=$(echo $impute_sum + $impute_time | bc)
        impute_time_check=true
    fi
done
echo -e "${LIGHTBLUE}==== Done ====${NOCOLOR}"

avg=`echo "$sum / $n" | bc -l`
printf '%0.1f\n' "$avg" > $OUTPUT_FILE

if [[ $impute_time_check == true ]] ; then
    impute_avg=`echo "$impute_sum / ($n * 1000)" | bc -l`
    printf '%0.1f' "$impute_avg" >> $OUTPUT_FILE
fi

echo -e "${LIGHTBLUE}==== Result written to $OUTPUT_FILE ====${NOCOLOR}"

cleanup
