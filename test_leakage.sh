#!/bin/bash

source config.sh
source common.sh

(cd tp-fixedpoint && cargo +nightly run --bin timing_leak --release)
