#!/usr/bin/env python3
"""param_offsets.py — for each decompile .c file whose signature mentions a struct
type, list every offset accessed on that struct's parameter.

Decompile files are named `ADDR_FuncName__SIG.c` where SIG is the GCC-mangled
parameter list: `P9level_tag` = level_tag*, `P7obj_tag` = obj_tag*, etc.
T0/T1/T3 = repeated earlier types. `N20` = array of 20.

Usage: python3 param_offsets.py <StructName>
Output: per function, the param index holding the struct and the offsets touched.
"""
import os
import re
import sys
from collections import defaultdict

DECOMP = "/home/z/NightfirePort/work/decomp/ACTION.ELF"


def parse_one(sig, i):
    if i >= len(sig) or not sig[i].isdigit():
        return None, i
    num = ""
    while i < len(sig) and sig[i].isdigit():
        num += sig[i]
        i += 1
    n = int(num)
    name = sig[i : i + n]
    return name, i + n


def parse_sig(sig):
    """Return list of type names per parameter. Handles P<len><name>, T<n> repeats,
    N<len> arrays."""
    params = []
    i = 0
    names = []
    while i < len(sig):
        if sig[i] == "T":
            j = i + 1
            num = ""
            while j < len(sig) and sig[j].isdigit():
                num += sig[j]
                j += 1
            idx = int(num)  # T<n> = parameters[n], 0-based over the full list
            if 0 <= idx < len(names):
                names.append(names[idx])
            else:
                names.append(f"T{idx}?")
            i = j
            continue
        if sig[i] == "P" or sig[i] == "R":
            i += 1
        if i < len(sig) and sig[i] == "N":
            j = i + 1
            cnt = ""
            while j < len(sig) and sig[j].isdigit():
                cnt += sig[j]
                j += 1
            name, j2 = parse_one(sig, j)
            names.append(f"{name}[{cnt}]")
            i = j2
            continue
        name, i2 = parse_one(sig, i)
        if name is None:
            break
        names.append(name)
        i = i2
    return names


def scan(struct, addr_filter=None):
    rows = []
    for fn in sorted(os.listdir(DECOMP)):
        if not fn.endswith(".c"):
            continue
        m = re.match(r"^([0-9a-f]+)_(.+)$", fn)
        if not m:
            continue
        addr, sym = m.group(1), m.group(2)
        if addr_filter and not addr.startswith(addr_filter):
            continue
        sigpos = sym.find("__")
        if sigpos < 0:
            continue
        sig = sym[sigpos + 2 :]
        if sig.startswith("F"):
            sig = sig[1:]
        types = parse_sig(sig)
        idxs = [i + 1 for i, t in enumerate(types) if t == struct or t.startswith(struct + "[")]
        if not idxs:
            continue
        with open(os.path.join(DECOMP, fn), "r", errors="replace") as fh:
            lines = fh.readlines()
        hits = defaultdict(list)  # param idx -> list of (line, offset, dir)
        for ln, line in enumerate(lines, 1):
            for mm in re.finditer(r"param_(\d+)\s*\+\s*(0x[0-9a-fA-F]+|\d+)", line):
                p = int(mm.group(1))
                if p in idxs:
                    off = int(mm.group(2), 0)
                    hits[p].append((ln, off))
            for mm in re.finditer(r"param_(\d+)\[", line):
                p = int(mm.group(1))
                if p in idxs:
                    hits[p].append((ln, 0))
        if not hits:
            continue
        base = sym.split("__")[0]
        rows.append((addr, base, {p: v for p, v in hits.items()}))
    return rows


if __name__ == "__main__":
    struct = sys.argv[1]
    filt = sys.argv[3] if len(sys.argv) > 3 and sys.argv[2] == "--addr-filter" else None
    for addr, fn, hits in scan(struct, filt):
        for p, v in sorted(hits.items()):
            offs = sorted(set(o for _, o in v))
            print(f"{addr} {fn} param{p}: {', '.join(hex(o) for o in offs)} ({len(v)} refs)")
