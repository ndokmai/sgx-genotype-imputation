REF=$1
INPUT=$2
TARGET=app/target/release/app

(cd app && cargo build --release ) && \
    (cd runner && cargo build --release --features simulation) && \
    (cd client && cargo build --release) && \
(cd runner && cargo run --release --features simulation -- ../${TARGET} ../${REF}) &
(cd client && cargo run --release -- ../${INPUT})

#(cd app && cargo build --release ) && \
    #(cd runner && cargo build --release --features simulation) && \
    #(cd client && cargo build --release)


