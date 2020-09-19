import sys
input_m3vcf = sys.argv[1]

skip_dup = False
flag = False
firsttime = True
prev = ""
for line in open(input_m3vcf):
    if line[0] == "#":
        continue
    if "BLOCK:" in line:
        if firsttime:
            firsttime = False
        else:
            flag = True
        continue
    if flag:
        flag = False
        continue
    tok = line.split("\t")
    cur = tok[0] + ":" + tok[1]
    if skip_dup and cur == prev:
        continue
    prev = cur
    print(line.rstrip())
