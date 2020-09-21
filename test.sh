if [[ $LEAK_RESISTANT -eq 1 ]]
then
    MINIMAC_FLAGS="--features leak-resistant"
    SERVER_FLAGS="--features leak-resistant --no-default-features"
fi

if [[ $SIM -ne 1 ]]
then
    SERVER_FLAGS="$SERVER_FLAGS --target x86_64-fortanix-unknown-sgx"
fi

export RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

(cd minimac && cargo build --release $MINIMAC_FLAGS) &&
    (cd server && cargo build --release $SERVER_FLAGS) &&

(
cd minimac
cargo run -q --release $MINIMAC_FLAGS --bin cache_server &
cargo run -q --release $MINIMAC_FLAGS --bin host &
cd ..

cd server
cargo run -q --release $SERVER_FLAGS &
cd ..

cd minimac
cargo run -q --release $MINIMAC_FLAGS --bin client

{ kill $(pidof cache_server) &&
    wait $(pidof cache_server); } 2>/dev/null

)
