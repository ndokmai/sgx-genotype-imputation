#!/bin/bash

CLANG=clang+llvm-3.8.0-x86_64-linux-gnu-ubuntu-16.04
mkdir -p dependencies
(
cd dependencies
if [ ! -f  "$CLANG.tar.xz" ]
then
    wget -nc https://releases.llvm.org/3.8.0/$CLANG.tar.xz
fi

if [ ! -d  "$CLANG" ]
then
    tar -x -f $CLANG.tar.xz --skip-old-files -v
fi
)
export SMAC_CLANG_DIR="$(realpath dependencies/$CLANG)"
