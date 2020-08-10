#!/bin/sh
minimac4 --refHaps smallref.m3vcf --haps input.vcf --format DS --noPhoneHome --nobgzip --prefix test_mmac --probThreshold 0 --diffThreshold 0 --topThreshold 0
python3 parse_output.py test_mmac.dose.vcf > test_mmac.dose.txt
