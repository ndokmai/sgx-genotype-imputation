#!/bin/bash

source config.sh
source common.sh

cd client && cargo +nightly build --release $BIN_FLAGS
