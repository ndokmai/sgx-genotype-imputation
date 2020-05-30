TARGET=app/target/x86_64-fortanix-unknown-sgx/release/app

(cd app && cargo build --release --target x86_64-fortanix-unknown-sgx ) && \
    ftxsgx-elf2sgxs $TARGET --heap-size 0x20000000 --stack-size 0x20000 --threads 8 --debug && \
(cd runner && cargo build --release) && \
    (cd client && cargo build --release) && \
(cd runner && cargo run --release -- ../${TARGET}.sgxs ../data/100_haps_10K_markers.txt) &
(cd client && cargo run --release -- ../data/10000_marker_sample.txt)


