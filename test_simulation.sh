TARGET=app/target/release/app

(cd app && cargo build --release ) && \
    (cd runner && cargo build --release --features simulation) && \
    (cd client && cargo build --release) && \
(cd runner && cargo run --release --features simulation -- ../${TARGET} ../data/100_haps_10K_markers.txt) &
(cd client && cargo run --release -- ../data/10000_marker_sample.txt)


