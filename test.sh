export RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

(cd minimac && cargo build --release --features leak-resistant)
(cd server && cargo build --release --target x86_64-fortanix-unknown-sgx)

cd minimac
cargo run -q --release --features leak-resistant --bin cache_server &
cargo run -q --release --features leak-resistant --bin host &
cd ..

cd server
cargo run -q --release --target x86_64-fortanix-unknown-sgx &
cd ..

cd minimac
cargo run -q --release --features leak-resistant --bin client

killall cache_server
