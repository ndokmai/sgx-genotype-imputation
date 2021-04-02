#!/bin/bash

source config.sh
source common.sh

(cd smac && cargo +nightly run --bin timing_leak --release)
