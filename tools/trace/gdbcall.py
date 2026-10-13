"""Record real calls of one game function in PCSX2: its inputs at entry and its outputs at return.

The differential-testing half of `verify-bitexact` (docs/tools/call-traces.md): the port replays each recorded call
through its own version of the function and must produce the same bits. Runs against the GDB-server PCSX2
(docs/tools/pcsx2-gdb.md, EE server 127.0.0.1:10501) with a level loaded and the game running.

How: a breakpoint at the function's entry. At each hit: read every register (`g`), evaluate the spec's regions
against the ENTRY registers, read them, and put a temporary breakpoint on $ra. Continue; when the CPU stops at $ra
with the entry $sp (so it is this call's return, not another path through that address), read $v0/$v1/$f0 and the
regions again. Remove the $ra breakpoint, re-arm the entry, continue. Ends after N calls or S seconds, then removes
every breakpoint and resumes the game.

Usage:
  python -I tools/trace/gdbcall.py <function|0xaddr> --spec "<region> ..." [--calls N] [--seconds S] [--out FILE]

A region is `in:<expr>:<len>`, `out:<expr>:<len>` or `io:<expr>:<len>` (read at entry, at return, or both).
<expr> is a sum of terms: a register (`a0`, `sp`, `s1` …), a hex/dec number, or a dereference `@(<expr>)` (the u32
at that address, read at entry). Examples:
  Quat_QuatToMat          --spec "in:a0:16 out:a1:64"
  Player_AutoAim          --spec "io:@(a0+0xe0)+0x114:0x20 in:@(a0+0xdc)+0x60:4"
  Rand_Rand               --spec "io:0x<rng state addr>:8"   (registers alone carry the argument and result)
"""
import argparse
import bisect
import os
import re
import socket
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gdbwatch import Gdb, load_symbols, name  # noqa: E402

GPR = ["zero", "at", "v0", "v1", "a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7",
       "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "t8", "t9", "k0", "k1", "gp", "sp", "fp", "ra"]
PC, FPR0 = 37, 38
MAX_READ = 0x800  # bytes per `m` packet (the server's PacketSize is 0x47ff hex chars)


def regs_all(g):
    """All 72 GDB registers as u32s (GPRs 0-31 lower words, 32 sr, 33 lo, 34 hi, 35 bad, 36 cause, 37 pc,
    38-69 f0-f31, 70 fcsr)."""
    r = g.cmd("g")
    return [int.from_bytes(bytes.fromhex(r[i * 8:i * 8 + 8]), "little") for i in range(len(r) // 8)]


def read_mem(g, addr, n):
    out = b""
    while n > 0:
        k = min(n, MAX_READ)
        r = g.cmd(f"m{addr:x},{k:x}")
        if not r or r.startswith("E"):
            raise IOError(f"memory read failed at {addr:#x}: {r}")
        out += bytes.fromhex(r)
        addr, n = addr + k, n - k
    return out


def tokenize(expr):
    return re.findall(r"@\(|\)|\+|0x[0-9a-fA-F]+|\d+|[a-z][a-z0-9]*", expr)


def evaluate(expr, regs, g):
    """Sum of terms; `@(e)` = the u32 at e (read now, i.e. at entry)."""
    toks = tokenize(expr)
    pos = 0

    def term():
        nonlocal pos
        t = toks[pos]; pos += 1
        if t == "@(":
            v = total()
            assert toks[pos] == ")", f"missing ) in {expr}"
            pos += 1
            return int.from_bytes(read_mem(g, v, 4), "little")
        if t.startswith("0x") or t.isdigit():
            return int(t, 0)
        if t in GPR:
            return regs[GPR.index(t)]
        if t == "s8":
            return regs[30]
        raise ValueError(f"unknown term {t!r} in {expr}")

    def total():
        nonlocal pos
        v = term()
        while pos < len(toks) and toks[pos] == "+":
            pos += 1
            v = (v + term()) & 0xFFFFFFFF
        return v

    return total()


def parse_spec(spec):
    regions = []
    for item in spec.split():
        when, expr, n = item.split(":")
        assert when in ("in", "out", "io"), item
        regions.append((when, expr, int(n, 0)))
    return regions


def resolve(fn, syms):
    if fn.startswith("0x"):
        return int(fn, 16)
    for a, _size, n in syms:
        if n == fn or n.split("__")[0] == fn:
            return a
    sys.exit(f"no symbol {fn}")


def symbol_addr(sym, syms_all):
    for line in open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "work", "action-symtab.tsv"),
                     encoding="utf-8"):
        f = line.rstrip("\n").split("\t")
        if len(f) >= 6 and f[5] == sym:
            return int(f[0], 16)
    return None


def wait_stop(g, deadline):
    while time.time() < deadline:
        g.s.settimeout(max(0.2, deadline - time.time()))
        try:
            p = g._read_packet()
        except socket.timeout:
            return None
        if p and p[0] in "TS":
            return p
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("function")
    ap.add_argument("--spec", default="")
    ap.add_argument("--calls", type=int, default=200)
    ap.add_argument("--seconds", type=float, default=20.0)
    ap.add_argument("--out")
    a = ap.parse_args()

    syms = load_symbols()
    entry = resolve(a.function, syms)
    fname = name(syms, entry).split("+")[0]
    regions = parse_spec(a.spec)
    gamestate = symbol_addr("GameState", syms)
    out = open(a.out, "w", encoding="utf-8", newline="\n") if a.out else sys.stdout

    g = Gdb()
    g.cmd("?")  # attach (pauses the VM)
    level = None
    if gamestate:
        level = int.from_bytes(read_mem(g, gamestate + 0x0C, 4), "little")
    out.write(f"# gdbcall v1 function={fname} addr={entry:#010x} spec=\"{a.spec}\" level={level:08x}\n"
              if level is not None else f"# gdbcall v1 function={fname} addr={entry:#010x} spec=\"{a.spec}\"\n")

    n = 0
    deadline = time.time() + a.seconds
    try:
        assert g.cmd(f"Z0,{entry:x},4") == "OK", "could not set the entry breakpoint"
        g.send("c")
        while n < a.calls:
            stop = wait_stop(g, deadline)
            if stop is None:
                break
            r = regs_all(g)
            if r[PC] != entry:
                g.send("c"); continue
            frame = int.from_bytes(read_mem(g, gamestate + 0x34, 4), "little") if gamestate else 0
            ra, sp = r[31], r[29]
            reads = []
            for when, expr, ln in regions:
                addr = evaluate(expr, r, g)
                reads.append((when, expr, addr, ln, read_mem(g, addr, ln) if when in ("in", "io") else None))
            # Run to this call's return: the entry breakpoint off (no nested records), $ra on.
            g.cmd(f"z0,{entry:x},4")
            g.cmd(f"Z0,{ra:x},4")
            back = None
            while True:
                g.send("c")
                if wait_stop(g, deadline + 5) is None:
                    break
                rr = regs_all(g)
                if rr[PC] == ra and rr[29] == sp:
                    back = rr
                    break
            g.cmd(f"z0,{ra:x},4")
            g.cmd(f"Z0,{entry:x},4")
            if back is None:
                break
            n += 1
            out.write(f"call {n} frame {frame} ra {ra:08x} sp {sp:08x}\n")
            out.write("gpr " + " ".join(f"{v:08x}" for v in r[0:32]) + "\n")
            out.write("fpr " + " ".join(f"{v:08x}" for v in r[FPR0:FPR0 + 32]) + "\n")
            for when, expr, addr, ln, data in reads:
                if data is not None:
                    out.write(f"in {expr} {addr:08x} {ln:x} {data.hex()}\n")
            out.write(f"ret v0 {back[2]:08x} v1 {back[3]:08x} f0 {back[FPR0]:08x}\n")
            for when, expr, addr, ln, _ in reads:
                if when in ("out", "io"):
                    out.write(f"out {expr} {addr:08x} {ln:x} {read_mem(g, addr, ln).hex()}\n")
            out.write("\n")
            out.flush()
            g.send("c")
    finally:
        g.s.settimeout(5.0)
        try:
            g.s.sendall(b"\x03")
        except Exception:
            pass
        g.s.settimeout(1.0)
        while True:
            try:
                g._read_packet()
            except (socket.timeout, OSError, ConnectionError):
                break
        g.buf = b""
        g.s.settimeout(5.0)
        print(g.pcsx2("debug_clear_breakpoints"), file=sys.stderr)
        print("resume:", g.pcsx2("resume"), file=sys.stderr)
        g.close()
        print(f"{n} calls of {fname} recorded", file=sys.stderr)


if __name__ == "__main__":
    main()
