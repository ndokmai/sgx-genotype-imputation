#!/bin/bash

source config.sh
source common.sh

(cd host && cargo +nightly build --release $BIN_FLAGS) &&
    (cd service-provider && cargo +nightly build --release $SP_FLAGS $BIN_FLAGS -Zfeatures=itarget)
