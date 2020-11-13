#!/bin/bash

RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"
export RUSTFLAGS="$RUSTFLAGS"

if [[ $SGX -eq 1 ]]
then
    SP_FLAGS="--target x86_64-fortanix-unknown-sgx"
else
    RA=0
fi

if [[ $LITE -eq 1 ]]
then
    if [[ $RA -eq 1 ]]
    then
        BIN_FLAGS="--features smac-lite,remote-attestation --no-default-features"
    else
        BIN_FLAGS="--features smac-lite --no-default-features"
    fi
else
    if [[ $RA -eq 0 ]]
    then
        BIN_FLAGS="--features smac --no-default-features"
    fi
fi

if [[ ! -z "$SMAC_CLANG_DIR" ]]
then
    export PATH=$SMAC_CLANG_DIR/bin/:$PATH
    export LLVM_CONFIG_PATH=$SMAC_CLANG_DIR/bin/llvm-config
    export LD_LIBRARY_PATH=$SMAC_CLANG_DIR/lib
fi

# For building & running enclave
TARGET_NAME=smac-service-provider
TARGET_DIR=service-provider/target/x86_64-fortanix-unknown-sgx/release
TARGET=$TARGET_DIR/$TARGET_NAME
TARGET_SGX=$TARGET_DIR/$TARGET_NAME.sgxs
TARGET_SIG=$TARGET_DIR/$TARGET_NAME.sig
