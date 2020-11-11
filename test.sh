if [[ $LITE -eq 1 ]]
then
    SMAC_FLAGS="--no-default-features"
    SP_FLAGS="--features smac-lite --no-default-features"
fi

if [[ $NO_SGX -ne 1 ]]
then
    SP_FLAGS="$SP_FLAGS --target x86_64-fortanix-unknown-sgx"
fi

export RUSTFLAGS="-Ctarget-cpu=native -Ctarget-feature=+aes,+avx,+avx2,+sse2,+sse4.1,+ssse3"

(cd smac && cargo build --release $SMAC_FLAGS) &&
    (cd service-provider && cargo build --release $SP_FLAGS) &&

(
cd smac
cargo run -q --release $SMAC_FLAGS --bin host &
cd ..

cd service-provider
cargo run -q --release $SP_FLAGS &
cd ..

cd smac
cargo run -q --release $SMAC_FLAGS --bin client
)
