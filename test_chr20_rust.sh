#!/bin/sh
SAMPLE="chr20_HG00128_hap1"
mkdir -p out/rust
gunzip -k data/chr20_train_recompressed.chunk.1.GWAS.m3vcf.gz
cd minimac
cargo run --release --bin test_run --features leak-resistant ../data/chr20_train_recompressed.chunk.1.GWAS.m3vcf ../data/chr20_chunk1_ind.txt ../data/${SAMPLE}_chunk1_dat.txt ../out/rust/${SAMPLE}_chunk1_rust.txt 
cd ..
