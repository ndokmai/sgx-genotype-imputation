import sys
fname = sys.argv[1]
for line in open(fname):
    if line[0] == "#":
        continue
    val = line.strip().split("\t")[-1]
    print(val)
