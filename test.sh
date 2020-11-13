#!/bin/bash
source config.sh
source common.sh

./run_sp.sh smac/test_data/largeref.m3vcf.gz &

sleep 0.1

./run_client.sh 127.0.0.1 smac/test_data/large_input_ind.txt smac/test_data/large_input_dat.txt output.txt
