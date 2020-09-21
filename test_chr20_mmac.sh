#!/bin/sh
SAMPLE="chr20_HG00128_hap1"
mkdir -p out/mmac
Minimac4/build/minimac4 --refHaps data/chr20_train_recompressed.chunk.1.GWAS.m3vcf.gz --haps data/${SAMPLE}.vcf.gz --format DS --noPhoneHome --nobgzip --prefix out/mmac/${SAMPLE}_chunk1 --probThreshold 0 --diffThreshold 0 --topThreshold 0 --intermediate data/chr20_train_recompressed
python minimac/parse_output.py out/mmac/${SAMPLE}_chunk1.dose.vcf > out/mmac/${SAMPLE}_chunk1_mmac.txt
