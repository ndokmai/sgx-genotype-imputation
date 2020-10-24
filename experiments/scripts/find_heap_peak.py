#!/usr/bin/env python3
import sys
import math
import msparser

filename = sys.argv[1]
data = msparser.parse_file(filename)
peak_index = data['peak_snapshot_index']
peak_snapshot = data['snapshots'][peak_index]
peak_heap = peak_snapshot['mem_heap']
extra = peak_snapshot['mem_heap_extra']
total = peak_heap + extra
total_mb = math.ceil(total / (1 << 20))
print(total_mb)
