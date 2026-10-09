#!/usr/bin/env python3
"""Minimal MIPS (R5900 o32) disassembler for ACTION.ELF, per HANDOFF.md §3.

Usage:
  python3 mipsdis.py <function-addr-hex> [count]   (repo default /home/z/my-project/NightfirePort)
  python3 mipsdis.py 1cb4f8 257        # disassemble 257 instructions

Reads work/elf/ACTION.ELF (single LOAD: vaddr 0x107100 -> file 0x1000) and
work/action-symtab.tsv for symbol names. Decodes the integer/FPU subset the
game's C compiles to; EE vector ops (COP2, MMI) are tagged. Not complete:
hand-check unknowns.
"""
import struct
import sys
import bisect

ELF = None
SYMADDR = {}
SYMS = []
LOAD_OFF = 0x106100  # vaddr - LOAD_OFF = file offset (0x107100 - 0x1000)
LOAD_LO, LOAD_HI = 0x107100, 0x107100 + 0x206570

R = ["zero","at","v0","v1","a0","a1","a2","a3","t0","t1","t2","t3","t4","t5","t6","t7",
     "s0","s1","s2","s3","s4","s5","s6","s7","t8","t9","k0","k1","gp","sp","fp","ra"]
F = ["f%d" % i for i in range(32)]

def load(repo):
    global ELF
    ELF = open(repo + "/work/elf/ACTION.ELF", "rb").read()
    with open(repo + "/work/action-symtab.tsv") as fh:
        for line in fh:
            p = line.rstrip("\n").split("\t")
            if len(p) >= 6 and p[5]:
                try:
                    a = int(p[0], 16)
                except ValueError:
                    continue
                SYMADDR[a] = p[5]
                SYMS.append((a, p[5]))
    SYMS.sort()
    return len(SYMS)

def sym(addr):
    if addr in SYMADDR:
        return SYMADDR[addr]
    i = bisect.bisect_right(SYMS, (addr, "\xff")) - 1
    if i >= 0 and addr - SYMS[i][0] < 0x800:
        return "%s+%x" % (SYMS[i][1], addr - SYMS[i][0])
    return None

def fetch(addr):
    if not (LOAD_LO <= addr < LOAD_HI):
        raise ValueError("addr %08x outside LOAD" % addr)
    return struct.unpack_from("<I", ELF, addr - LOAD_OFF)[0]

def dis(addr, n):
    out = []
    for i in range(n):
        a = addr + i * 4
        w = fetch(a)
        out.append("%08x  %08x  %s" % (a, w, decode(a, w)))
    return "\n".join(out)

def decode(a, w):
    op = w >> 26
    rs, rt, rd = (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31
    sa = (w >> 6) & 31
    imm, simm = w & 0xFFFF, (w & 0xFFFF) - 0x10000 if w & 0x8000 else (w & 0xFFFF)
    tgt = ((a + 4) & 0xF0000000) | ((w & 0x3FFFFFF) << 2)
    btgt = a + 4 + (simm << 2)
    def S(x): return sym(x) or ("%08x" % x)
    if op == 0:
        f = w & 63
        if w == 0: return "nop"
        if f == 0: return "sll %s,%s,%d" % (R[rd], R[rt], sa)
        if f == 2: return "srl %s,%s,%d" % (R[rd], R[rt], sa)
        if f == 3: return "sra %s,%s,%d" % (R[rd], R[rt], sa)
        if f == 4: return "sllv %s,%s,%s" % (R[rd], R[rt], R[rs])
        if f == 6: return "srlv %s,%s,%s" % (R[rd], R[rt], R[rs])
        if f == 7: return "srav %s,%s,%s" % (R[rd], R[rt], R[rs])
        if f == 8: return "jr %s" % R[rs]
        if f == 9: return "jalr %s,%s" % (R[rd], R[rs])
        if f == 10: return "movz %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 11: return "movn %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 12: return "syscall %x" % (w >> 6)
        if f == 13: return "break"
        if f == 15: return "sync"
        if f == 16: return "mfhi %s" % R[rd]
        if f == 17: return "mthi %s" % R[rs]
        if f == 18: return "mflo %s" % R[rd]
        if f == 19: return "mtlo %s" % R[rs]
        if f == 24: return "mult %s,%s" % (R[rs], R[rt])
        if f == 25: return "multu %s,%s" % (R[rs], R[rt])
        if f == 26: return "div %s,%s" % (R[rs], R[rt])
        if f == 27: return "divu %s,%s" % (R[rs], R[rt])
        if f == 32: return "add %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 33: return "addu %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 34: return "sub %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 35: return "subu %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 36: return "and %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 37: return "or %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 38: return "xor %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 39: return "nor %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 42: return "slt %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 43: return "sltu %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 44: return "dadd %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 45: return "daddu %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 46: return "dsub %s,%s,%s" % (R[rd], R[rs], R[rt])
        if f == 47: return "dsubu %s,%s,%s" % (R[rd], R[rs], R[rt])
        return "special? f=%d %08x" % (f, w)
    if op == 1:
        f = rt
        if f == 0: return "bltz %s,%08x" % (R[rs], btgt)
        if f == 1: return "bgez %s,%08x" % (R[rs], btgt)
        if f == 2: return "bltzl %s,%08x" % (R[rs], btgt)
        if f == 3: return "bgezl %s,%08x" % (R[rs], btgt)
        if f == 16: return "bltzal %s,%08x" % (R[rs], btgt)
        if f == 17: return "bgezal %s,%08x" % (R[rs], btgt)
        return "regimm? %d" % f
    if op == 2: return "j %08x  ; %s" % (tgt, S(tgt))
    if op == 3: return "jal %08x  ; %s" % (tgt, S(tgt))
    if op == 4: return "beq %s,%s,%08x" % (R[rs], R[rt], btgt)
    if op == 5: return "bne %s,%s,%08x" % (R[rs], R[rt], btgt)
    if op == 6: return "blez %s,%08x" % (R[rs], btgt)
    if op == 7: return "bgtz %s,%08x" % (R[rs], btgt)
    if op == 8: return "addi %s,%s,%d" % (R[rt], R[rs], simm)
    if op == 9: return "addiu %s,%s,%d" % (R[rt], R[rs], simm)
    if op == 10: return "slti %s,%s,%d" % (R[rt], R[rs], simm)
    if op == 11: return "sltiu %s,%s,%d" % (R[rt], R[rs], simm)
    if op == 12: return "andi %s,%s,%x" % (R[rt], R[rs], imm)
    if op == 13: return "ori %s,%s,%x" % (R[rt], R[rs], imm)
    if op == 14: return "xori %s,%s,%x" % (R[rt], R[rs], imm)
    if op == 15: return "lui %s,%x" % (R[rt], imm)
    if op == 16: return "cop0 %08x" % w
    if op == 17:
        fmt, func = rs, w & 63
        fd, fs, ft = (w >> 6) & 31, (w >> 11) & 31, (w >> 16) & 31
        if fmt == 0: return "mfc1 %s,%s" % (R[rt], F[rd])
        if fmt == 2: return "cfc1 %s,f%d" % (R[rt], rd)
        if fmt == 4: return "mtc1 %s,%s" % (R[rt], F[rd])
        if fmt == 6: return "ctc1 %s,f%d" % (R[rt], rd)
        if fmt == 8: return ("bc1t %08x" if (w >> 16) & 1 else "bc1f %08x") % btgt
        if fmt in (16, 17):
            n = ".s" if fmt == 16 else ".d"
            if func == 0: return "add%s %s,%s,%s" % (n, F[fd], F[fs], F[ft])
            if func == 1: return "sub%s %s,%s,%s" % (n, F[fd], F[fs], F[ft])
            if func == 2: return "mul%s %s,%s,%s" % (n, F[fd], F[fs], F[ft])
            if func == 3: return "div%s %s,%s,%s" % (n, F[fd], F[fs], F[ft])
            if func == 4: return "sqrt%s %s,%s" % (n, F[fd], F[fs])
            if func == 5: return "abs%s %s,%s" % (n, F[fd], F[fs])
            if func == 6: return "mov%s %s,%s" % (n, F[fd], F[fs])
            if func == 7: return "neg%s %s,%s" % (n, F[fd], F[fs])
        if fmt == 20 and func == 32: return "cvt.s.w %s,%s" % (F[fd], F[fs])
        if fmt == 16 and func == 32: return "cvt.w.s %s,%s" % (F[fd], F[fs])
        if fmt in (16, 17) and func >= 48:
            return "c.%d%s %s,%s" % (func & 15, ".s" if fmt == 16 else ".d", F[fs], F[ft])
        return "cop1? fmt=%d func=%d %08x" % (fmt, func, w)
    if op == 18: return "cop2 %08x  ; VU0 macro" % w
    if op == 20: return "beql %s,%s,%08x" % (R[rs], R[rt], btgt)
    if op == 21: return "bnel %s,%s,%08x" % (R[rs], R[rt], btgt)
    if op == 22: return "blezl %s,%08x" % (R[rs], btgt)
    if op == 23: return "bgtzl %s,%08x" % (R[rs], btgt)
    if op == 28: return "mmi(0x1c) %08x" % w
    if op == 30: return "lq %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 31: return "sq %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 0x31: return "lwc1 %s,%d(%s)" % (F[rt], simm, R[rs])
    if op == 32: return "lb %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 33: return "lh %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 34: return "lwl %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 35: return "lw %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 36: return "lbu %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 37: return "lhu %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 38: return "lwr %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 39: return "lwu %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 40: return "sb %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 41: return "sh %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 42: return "swl %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 43: return "sw %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 46: return "swr %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 47: return "cache %d,%d(%s)" % (rt, simm, R[rs])
    if op == 48: return "ll %s,%d(%s)" % (R[rt], simm, R[rs])
    if op == 50: return "lqc2 vf%d,%d(%s)" % (rt, simm, R[rs])
    if op == 54: return "sqc2 vf%d,%d(%s)" % (rt, simm, R[rs])
    if op == 0x39: return "swc1 %s,%d(%s)" % (F[rt], simm, R[rs])
    if op == 56: return "sc %s,%d(%s)" % (R[rt], simm, R[rs])
    return "?? op=%d %08x" % (op, w)

if __name__ == "__main__":
    if len(sys.argv) >= 4:
        repo, addr_s, cnt = sys.argv[1], sys.argv[2], int(sys.argv[3])
    elif len(sys.argv) == 3:
        repo, addr_s, cnt = "/home/z/my-project/NightfirePort", sys.argv[1], int(sys.argv[2])
    else:
        repo, addr_s, cnt = "/home/z/my-project/NightfirePort", sys.argv[1], 64
    load(repo)
    print(dis(int(addr_s, 16), cnt))
