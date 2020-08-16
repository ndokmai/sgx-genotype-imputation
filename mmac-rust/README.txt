# How to run

Generate random input data based on largeref.m3vcf:

    python3 gen_template.py largeref.m3vcf > template.txt
    python3 gen_input.py template.txt input  

This produces input.txt (for rust) and input.vcf (for minimac). Then run:

    time cargo run --release

This writes imputed results to output.txt. Expected results from
minimac are included in output_minimac.txt.
