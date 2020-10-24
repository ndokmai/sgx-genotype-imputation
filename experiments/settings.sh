#!/bin/bash

# I/O
DATA_DIR=/home/ndokmai/workspace/temp/sgx-genotype-imputation/data # TODO change this
REF_PANEL=chr20_train_recompressed
SAMPLE=chr20_HG00128_hap1
CHUNK_ID=1
CHUNK=${SAMPLE}_chunk${CHUNK_ID}
REF_PANEL_FILE=$REF_PANEL.chunk.$CHUNK_ID.GWAS.m3vcf.gz
OUTPUT=${CHUNK}_output
TIME_OUTPUT=${CHUNK}_time_sec.txt
MEM_OUTPUT=${CHUNK}_mem_mb.txt

# experiment parameters
N_TIMES=10 # average timing results over n runs

# rust
PROJECT_DIR=/home/ndokmai/workspace/temp/sgx-genotype-imputation # TODO change this
RUST_INPUT_INDEX_FILE=${CHUNK}_ind.txt
RUST_INPUT_DATA_FILE=${CHUNK}_dat.txt
RUST_OUTPUT_FILE=$OUTPUT.txt
RUST_MINIMAC_DIR=$PROJECT_DIR/minimac
RUST_SERVER_DIR=$PROJECT_DIR/server
SERVER=$RUST_SERVER_DIR/target/release/server
HOST=$RUST_MINIMAC_DIR/target/release/host
CLIENT=$RUST_MINIMAC_DIR/target/release/client
RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

# minimac
MINIMAC=/home/ndokmai/workspace/Minimac4/release-build/minimac4
MINIMAC_INTERMEDIATE_FILE=$REF_PANEL
MINIMAC_INPUT_FILE=$SAMPLE.vcf.gz

# SGX
HEAP_SIZE=0x80000000
STACK_SIZE=0x200000
N_THREADS=8
SGX_SERVER=$RUST_SERVER_DIR/target/x86_64-fortanix-unknown-sgx/release/server
ELF2SGXS="ftxsgx-elf2sgxs --heap-size $HEAP_SIZE --stack-size $STACK_SIZE --threads $N_THREADS $SGX_SERVER"
SGX_RUN_SERVER="ftxsgx-runner $SGX_SERVER.sgxs"

# don't change this par
REF_PANEL_FILE=$DATA_DIR/$REF_PANEL_FILE
RUST_INPUT_INDEX_FILE=$DATA_DIR/$RUST_INPUT_INDEX_FILE
RUST_INPUT_DATA_FILE=$DATA_DIR/$RUST_INPUT_DATA_FILE
MINIMAC_INTERMEDIATE_FILE=$DATA_DIR/$MINIMAC_INTERMEDIATE_FILE
MINIMAC_INPUT_FILE=$DATA_DIR/$MINIMAC_INPUT_FILE
