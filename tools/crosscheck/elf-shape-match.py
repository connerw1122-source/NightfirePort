#!/usr/bin/env python3
"""Shape-match decompiled functions between two PS2 ELFs (e.g. ACTION.ELF vs DRIVING.ELF).

Two passes, in order:
  1. exact: same size AND byte-identical body — the strongest evidence a function is the
     same compiled artifact in both builds.
  2. shape: same size and same "shape signature" — each instruction word masked so that
     build-dependent bits are dropped: j/jal keep only the opcode (targets differ), lui keeps
     opcode+regs (the address hi differs), I-type ALU/memory keeps opcode+rs+rt and drops the
     immediate; SPECIAL/REGIMM/branches/COP1 keep the whole word (branch offsets are relative
     and survive a relink).

Matches where both sides carry names (ACTION.ELF has a symtab; DRIVING.ELF is stripped) name
the DRIVING side too. Matches under 32 bytes are stub-collisions (e.g. `return 0` thunks) and
must not be trusted as evidence.

Usage: python3 tools/crosscheck/elf-shape-match.py [<elfA> <decompA> <elfB> <decompB>]
Defaults: ACTION.ELF vs DRIVING.ELF in work/. Prints counts and the list sorted by size,
and writes /tmp/shapematch.json.
"""
import json
import os
import re
import struct
import sys
from collections import defaultdict

A_ELF, A_DEC = 'work/elf/ACTION.ELF', 'work/decomp/ACTION.ELF'
B_ELF, B_DEC = 'work/elf/DRIVING.ELF', 'work/decomp/DRIVING.ELF'


def load_mapping(elf_path):
    """(filesz-bearing LOAD) → file offset = vaddr - delta."""
    data = open(elf_path, 'rb').read()
    phoff = struct.unpack_from('<I', data, 28)[0]
    phentsize, phnum = struct.unpack_from('<HH', data, 42)
    for i in range(phnum):
        o = phoff + i * phentsize
        p_type, p_offset, p_vaddr, _, p_filesz, _, _, _ = struct.unpack_from('<8I', data, o)
        if p_type == 1:
            return data, p_vaddr - p_offset, p_vaddr + p_filesz
    raise SystemExit('no LOAD segment in ' + elf_path)


def parse_decomp(d):
    out = []
    for fn in os.listdir(d):
        m = re.match(r'([0-9a-f]{8})_(.+)\.c', fn)
        if not m:
            continue
        head = open(os.path.join(d, fn), 'rb').read(300).decode('latin1')
        sm = re.search(r'size=(\d+)', head)
        if sm:
            out.append((int(m.group(1), 16), int(sm.group(1)), m.group(2)))
    return out


def sig(b):
    n = len(b) // 4
    words = struct.unpack_from('<%dI' % n, b)
    out = []
    for w in words:
        op = w >> 26
        if op in (2, 3):
            out.append(w & 0xFC000000)
        elif op == 0x0F:
            out.append(w & 0xFFFF0000)
        elif op in (0, 1, 4, 5, 6, 7, 0x10, 0x11, 0x12, 0x13, 0x1C, 0x1D, 0x1E, 0x1F):
            out.append(w)
        else:
            out.append(w & 0xFFFFFC00)
    return struct.pack('<%dI' % n, *out)


def main():
    a_elf, a_off, a_end = load_mapping(A_ELF)
    b_elf, b_off, b_end = load_mapping(B_ELF)
    A, B = parse_decomp(A_DEC), parse_decomp(B_DEC)
    print(f'A: {len(A)} funcs   B: {len(B)} funcs')

    names = {}
    tsv = 'work/action-symtab.tsv'
    if os.path.exists(tsv):
        for line in open(tsv):
            p = line.rstrip('\n').split('\t')
            if len(p) >= 6 and p[5]:
                try:
                    names[int(p[0], 16)] = p[5]
                except ValueError:
                    pass

    def body(elf, off, end, a, s):
        fo = a - off
        if fo < 0 or fo + s > min(end, len(elf)):
            return None
        return elf[fo:fo + s]

    b_by_size = defaultdict(list)
    for a, s, n in B:
        b_by_size[s].append(a)
    exact, used = [], set()
    for a, s, n in sorted(A, key=lambda t: -t[1]):
        ba = body(a_elf, a_off, a_end, a, s)
        if ba is None:
            continue
        for da in b_by_size.get(s, ()):
            if ba == body(b_elf, b_off, b_end, da, s):
                exact.append((a, da, s, n))
                used.add(a)
                break
    print('exact:', len(exact))

    b_map = defaultdict(list)
    for a, s, n in B:
        if s < 16:
            continue
        bb = body(b_elf, b_off, b_end, a, s)
        if bb is not None:
            b_map[(s, sig(bb))].append(a)
    shape = []
    for a, s, n in sorted(A, key=lambda t: -t[1]):
        if a in used or s < 16:
            continue
        ba = body(a_elf, a_off, a_end, a, s)
        if ba is None:
            continue
        cands = b_map.get((s, sig(ba)))
        if cands:
            shape.append((a, cands[0], s, n))
    print('shape:', len(shape))

    allm = [(a, d, s, nm, 'exact') for a, d, s, nm in exact]
    allm += [(a, d, s, nm, 'shape') for a, d, s, nm in shape]
    allm.sort(key=lambda t: -t[2])
    for a, d, s, nm, how in allm:
        if s >= 32:
            print(f'  A {a:08x}  B {d:08x}  size={s:5d}  {how:5s}  {nm}')
    json.dump({'exact': exact, 'shape': shape}, open('/tmp/shapematch.json', 'w'))


if __name__ == '__main__':
    main()
