import numpy as np

x = np.random.randint(0,3,936)
np.savetxt("input.txt",x-1,"%d")

fp = open("input.vcf",'w')
fp.write("##fileformat=VCFv4.0\n")
fp.write("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tNA00001\n")

g = [int(line) for line in open("input.txt")]
ind = -1
for line in open("input_template.vcf"):
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
