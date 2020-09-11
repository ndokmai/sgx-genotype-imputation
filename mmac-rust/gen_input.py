import numpy as np
import sys

template_file = sys.argv[1]
out_prefix = sys.argv[2]

nvar = len([0 for line in open(template_file)])

x = np.random.randint(0,3,nvar)
np.savetxt(out_prefix + ".txt",x-1,"%d")

fp = open(out_prefix+".vcf",'w')
fp.write("##fileformat=VCFv4.0\n")
fp.write("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tNA00001\n")

g = [int(line) for line in open(out_prefix+".txt")]
ind = -1
for line in open(template_file):
    ind = ind + 1
    if g[ind] < 0:
        #continue
        #geno = "./."
        geno = "."
    else:
        #geno = "%d|%d" % (g[ind], g[ind])
        geno = "%d" % g[ind]
    tok = line.rstrip().split("\t")
    tok[8] = "GT\t" + geno
    line = "\t".join(tok)
    fp.write(line + "\n")
fp.close()
