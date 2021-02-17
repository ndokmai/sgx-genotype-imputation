#!/bin/bash

source config.sh
source common.sh

echo "=========== Building Host... ==========="

(cd host && cargo +nightly build --release $BIN_FLAGS)

echo "========================================"

echo "===== Building Service Provider... ====="

(cd service-provider && cargo +nightly build --release $SP_FLAGS $BIN_FLAGS) &&
    if [[ $SGX -eq 1 ]]
    then
        SGX_THREADS=$(($N_THREADS+2))
        ftxsgx-elf2sgxs $TARGET --heap-size $ENCLAVE_HEAP_SIZE -d --stack-size $ENCLAVE_STACK_SIZE --threads $SGX_THREADS --output $TARGET_SGX &&
        sgxs-sign --key $SP_SIGNING_KEY $TARGET_SGX $TARGET_SIG -d --xfrm 7/0 --isvprodid 0 --isvsvn 0
    fi

echo "========================================"

echo "=========== Building Client... ========="

(cd client && cargo +nightly build --release $BIN_FLAGS)

echo "========================================"

if [[ $RA -eq 1 ]]
then
    (
    cd client/keys
    wget -nc -c https://certificates.trustedservices.intel.com/Intel_SGX_Attestation_RootCA.pem
    )
fi
