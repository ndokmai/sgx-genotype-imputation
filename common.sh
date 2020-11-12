#!/bin/bash

RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"
export RUSTFLAGS="$RUSTFLAGS"

if [[ $LITE -eq 1 ]]
then
    BIN_FLAGS="--features smac-lite --no-default-features"
fi

if [[ $SGX -eq 1 ]]
then
    SP_FLAGS="--target x86_64-fortanix-unknown-sgx"
fi
