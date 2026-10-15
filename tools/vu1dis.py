#!/usr/bin/env python3
"""VU1 microcode disassembler for the NightfirePort VU1 microcode task (G-014).

Decodes 64-bit VU instruction pairs (upper | lower) as documented in the public
Sony VU manual and cross-checked against PCSX2's VU disassembler tables
(pcsx2/pcsx2/DebugTools/DisVUmicro.h, DisVUops.h, GPL-3.0; fetched from
https://github.com/PCSX2/pcsx2 — the tables below follow those exactly).

Usage:
  python3 tools/vu1dis.py <file.bin> [--base N] [--qwords N] [--format auto]
    --base      first qword index to print (default 0)
    --qwords    number of qwords (default: whole file)
    --format    raw (plain qwords) | ovly (name = DVP overlay section, see below)

DVP overlay sections from ACTION.ELF (see docs/formats/vu1-microcode.md):
  python3 tools/vu1dis.py <ACTION.ELF> --elf   dumps .vutext + every overlay.
"""
import struct, sys

# ---------------------------------------------------------------- tables ----
# Upper op = bits[5:0]; special group 0x3c..0x3f indexed by fd = bits[10:6].
# (DisVUmicro.h: disVU_MicroU, disVU_UPPER_FD_00/01/10/11_TABLE.)
UPPER = {
    0x00: "ADDx", 0x01: "ADDy", 0x02: "ADDz", 0x03: "ADDw",
    0x04: "SUBx", 0x05: "SUBy", 0x06: "SUBz", 0x07: "SUBw",
    0x08: "MADDx", 0x09: "MADDy", 0x0A: "MADDz", 0x0B: "MADDw",
    0x0C: "MSUBx", 0x0D: "MSUBy", 0x0E: "MSUBz", 0x0F: "MSUBw",
    0x10: "MAXx", 0x11: "MAXy", 0x12: "MAXz", 0x13: "MAXw",
    0x14: "MINIx", 0x15: "MINIy", 0x16: "MINIz", 0x17: "MINIw",
    0x18: "MULx", 0x19: "MULy", 0x1A: "MULz", 0x1B: "MULw",
    0x1C: "MULq", 0x1D: "MAXi", 0x1E: "MULi", 0x1F: "MINIi",
    0x20: "ADDq", 0x21: "MADDq", 0x22: "ADDi", 0x23: "MADDi",
    0x24: "SUBq", 0x25: "MSUBq", 0x26: "SUBi", 0x27: "MSUBi",
    0x28: "ADD", 0x29: "MADD", 0x2A: "MUL", 0x2B: "MAX",
    0x2C: "SUB", 0x2D: "MSUB", 0x2E: "OPMSUB", 0x2F: "MINI",
}
# (op-0x3c selects the table, fd selects the entry.)
UPPER_FD = [
    ["ADDAx", "SUBx", "MADDAx", "MSUBAx", "ITOF0", "FTOI0", "MULAx", "MULAq",
     "ADDAq", "SUBAq", "ADDA", "SUBA"],
    ["ADDAy", "SUBy", "MADDAy", "MSUBAy", "ITOF4", "FTOI4", "MULAy", "ABS",
     "MADDAq", "MSUBAq", "MADDA", "MSUBA"],
    ["ADDAz", "SUBz", "MADDAz", "MSUBAz", "ITOF12", "FTOI12", "MULAz", "MULAi",
     "ADDAi", "SUBAi", "MULA", "OPMULA"],
    ["ADDAw", "SUBw", "MADDAw", "MSUBAw", "ITOF15", "FTOI15", "MULAw", "CLIP",
     "MADDAi", "MSUBAi", None, "NOP"],
]

# Lower: major = bits[31:25] (DisVUmicro.h disVU_MicroL); major 0x40 = the
# special group with minor = bits[5:0], sub-index fd = bits[10:6].
LOWER_MAJOR = {
    0x00: "LQ", 0x01: "SQ", 0x04: "ILW", 0x05: "ISW",
    0x08: "IADDIU", 0x09: "ISUBIU",
    0x10: "FCEQ", 0x11: "FCSET", 0x12: "FCAND", 0x13: "FCOR",
    0x14: "FSEQ", 0x15: "FSSET", 0x16: "FSAND", 0x17: "FSOR",
    0x18: "FMEQ", 0x1A: "FMAND", 0x1B: "FMOR", 0x1C: "FCGET",
    0x20: "B", 0x21: "BAL", 0x24: "JR", 0x25: "JALR",
    0x28: "IBEQ", 0x29: "IBNE", 0x2C: "IBLTZ", 0x2D: "IBGTZ",
    0x2E: "IBLEZ", 0x2F: "IBGEZ",
    0x40: "LOWER_SPECIAL",
}
LOWER_SPECIAL = {
    0x30: "IADD", 0x31: "ISUB", 0x32: "IADDI", 0x34: "IAND", 0x35: "IOR",
    0x3C: "T3_00", 0x3D: "T3_01", 0x3E: "T3_10", 0x3F: "T3_11",
}
# (minor 0x3c..0x3f selects the table, fd selects the entry.)
LOWER_T3 = [
    {0x0C: "MOVE", 0x0D: "LQI", 0x0E: "DIV", 0x0F: "MTIR", 0x10: "RNEXT",
     0x19: "MFP", 0x1A: "XTOP", 0x1B: "XGKICK", 0x1C: "ESADD",
     0x1D: "EATANxy", 0x1E: "ESQRT", 0x1F: "ESIN"},
    {0x0C: "MR32", 0x0D: "SQI", 0x0E: "SQRT", 0x0F: "MFIR", 0x10: "RGET",
     0x1A: "XITOP", 0x1C: "ERSADD", 0x1D: "EATANxz", 0x1E: "ERSQRT",
     0x1F: "EATAN"},
    {0x0D: "LQD", 0x0E: "RSQRT", 0x0F: "ILWR", 0x10: "RINIT", 0x1C: "ELENG",
     0x1D: "ESUM", 0x1E: "ERCPR", 0x1F: "EEXP"},
    {0x0D: "SQD", 0x0E: "WAITQ", 0x0F: "ISWR", 0x10: "RXOR", 0x1C: "ERLENG",
     0x1E: "WAITP"},
]

UPPER_NOP = 0x000002FF   # op 0x3f (FD_11), fd 0x0b = NOP
LOWER_NOP = 0x8000033C   # major 0x40, minor 0x3c (T3_00), fd 0x0c = MOVE vf0,vf0

def dec_dest(code):
    m = (code >> 21) & 0xF
    return "".join(c for i, c in enumerate("xyzw") if m & (8 >> i))

def s11(v):  # sign-extend 11-bit immediate
    return v - 0x800 if v & 0x400 else v

def s5(v):
    return v - 0x10 if v & 0x10 else v

def dis_upper(code):
    """Decode the 32-bit upper half; returns text."""
    if code == UPPER_NOP:
        return "NOP"
    op = code & 0x3F
    ft = (code >> 16) & 0x1F
    fs = (code >> 11) & 0x1F
    fd = (code >> 6) & 0x1F
    flags = ""
    if code & (1 << 31): flags += "I"
    if code & (1 << 30): flags += "E"
    if code & (1 << 29): flags += "M"
    if code & (1 << 28): flags += "D"
    if code & (1 << 27): flags += "T"
    if flags: flags = "{" + flags + "}"
    dest = dec_dest(code)

    def fvf(r): return "vf%02d" % r
    def fvi(r): return "vi%02d" % r

    if op >= 0x3C:  # special group
        name = UPPER_FD[op - 0x3C][fd] if fd < 12 else None
        if name is None:
            return "BAD_UPPER(%08x)" % code
        if name == "NOP":
            return "NOP" + flags
        if name == "CLIP":  # CLIP fs, ft (no dest; the I bit is ignored by hardware)
            return "CLIP %s, %s" % (fvf(fs), fvf(ft))
        if name.startswith(("ADDA", "SUBA", "MULA", "MADDA", "MSUBA")):
            if name[-1] in "xyzw":  # broadcast forms: ADDAx ACC, fs, ft.x
                src = "%s.%s" % (fvf(ft), name[-1])
            elif name.endswith("i"):
                src = "I"
            elif name.endswith("q"):
                src = "Q"
            else:
                src = fvf(ft)
            return "%s.%s ACC, %s, %s%s" % (name, dest, fvf(fs), src, flags)
        if name in ("ITOF0", "FTOI0", "ITOF4", "FTOI4", "ITOF12", "FTOI12",
                    "ITOF15", "FTOI15", "ABS"):
            return "%s.%s %s, %s%s" % (name, dest, fvf(ft), fvf(fs), flags)
        return "BAD_UPPER(%08x)" % code

    name = UPPER.get(op)
    if name is None:
        return "BAD_UPPER(%08x)" % code
    bc = name[-1] if name[-1] in "xyzw" and name not in ("ADD", "MADD", "MUL",
                                                        "MAX", "SUB", "MSUB",
                                                        "MINI", "OPMSUB") else None
    if name in ("ADD", "MADD", "MUL", "MAX", "SUB", "MSUB", "MINI", "OPMSUB"):
        return "%s.%s %s, %s, %s%s" % (name, dest, fvf(fd), fvf(fs), fvf(ft), flags)
    if bc:  # broadcast form: ADDx fd, fs, ft.x
        return "%s.%s %s, %s, %s.%s%s" % (name, dest, fvf(fd), fvf(fs), fvf(ft), bc, flags)
    if name in ("MULq", "ADDq", "MADDq", "SUBq", "MSUBq", "MULi", "ADDi",
                "MADDi", "SUBi", "MSUBi", "MAXi", "MINIi"):
        src = "Q" if name[-1] == "q" else "I"
        return "%s.%s %s, %s, %s%s" % (name, dest, fvf(fd), fvf(fs), src, flags)
    return "BAD_UPPER(%08x)" % code

def dis_lower(code, pc):
    """Decode the 32-bit lower half; pc = qword index (for branch targets)."""
    if code == LOWER_NOP:
        return "NOP"
    major = code >> 25
    dest = dec_dest(code)
    it = (code >> 16) & 0x1F
    is_ = (code >> 11) & 0x1F
    id_ = (code >> 6) & 0x1F
    imm11 = code & 0x7FF
    imm15 = ((code >> 10) & 0x7800) | (code & 0x7FF)
    def fvi(r): return "vi%02d" % r
    def fvf(r): return "vf%02d" % r
    bt = lambda: "0x%03x" % (pc + 1 + s11(imm11))  # target qword (branch+1+imm)

    name = LOWER_MAJOR.get(major)
    if name == "LOWER_SPECIAL":
        minor = code & 0x3F
        name = LOWER_SPECIAL.get(minor, "BAD_LOWER(%08x)" % code)
        if name.startswith("T3_"):
            t3sel = {"T3_00": 0, "T3_01": 1, "T3_10": 2, "T3_11": 3}[name]
            sub = LOWER_T3[t3sel].get(id_)
            if sub is None:
                return "BAD_LOWER(%08x)" % code
            if sub in ("MOVE", "MR32"):
                return "%s%s %s, %s" % (sub, dest, fvf(it), fvf(is_))
            if sub in ("LQI", "LQD"):
                return "%s%s %s, (vi%02d)" % (sub, dest, fvf(it), is_)
            if sub in ("SQI", "SQD"):
                return "%s%s (vi%02d), %s" % (sub, dest, it, fvf(is_))
            if sub in ("DIV", "RSQRT"):
                sf = "xyzw"[(code >> 21) & 3]
                tf = "xyzw"[(code >> 23) & 3]
                return "%s %s.%s, %s.%s" % (sub, fvf(is_), sf, fvf(it), tf)
            if sub in ("MTIR",):
                sf = "xyzw"[(code >> 21) & 3]
                return "MTIR %s, %s.%s" % (fvi(it), fvf(is_), sf)
            if sub in ("MFIR",):
                return "MFIR%s %s, %s" % (dest, fvf(it), fvi(is_))
            if sub in ("MFP",):
                return "MFP%s %s, P" % (dest, fvf(it))
            if sub in ("XTOP", "XITOP"):
                return "%s %s" % (sub, fvi(it))
            if sub == "XGKICK":
                return "XGKICK %s" % fvi(is_)
            if sub in ("RNEXT", "RGET"):
                return "%s%s %s, R" % (sub, dest, fvf(it))
            if sub == "RINIT":
                return "RINIT %s, %s" % (fvi(it), fvf(is_))
            if sub == "RXOR":
                return "RXOR %s, %s" % (fvi(it), fvf(is_))
            if sub in ("ESADD", "ERSADD", "ELENG", "ERLENG", "ESUM", "ERCPR",
                       "ESQRT", "ERSQRT", "EATANxy", "EATANxz", "EATAN", "ESIN",
                       "EEXP"):
                return "%s %s" % (sub, fvf(is_))
            if sub in ("WAITQ", "WAITP"):
                return sub
            return "BAD_LOWER(%08x)" % code
        if name == "IADDI":
            return "IADDI %s, %s, %d" % (fvi(it), fvi(is_), s5(id_))
        if name in ("IADD", "ISUB", "IAND", "IOR"):
            return "%s %s, %s, %s" % (name, fvi(id_), fvi(is_), fvi(it))
        return "BAD_LOWER(%08x)" % code

    if name is None:
        return "BAD_LOWER(%08x)" % code
    if name == "LQ":
        return "LQ.%s %s, %d(%s)" % (dest, fvf(it), s11(imm11), fvi(is_))
    if name == "SQ":
        return "SQ.%s %s, %d(%s)" % (dest, fvf(is_), s11(imm11), fvi(it))
    if name == "ILW":
        return "ILW.%s %s, %d(%s)" % (dest, fvi(it), s11(imm11), fvi(is_))
    if name == "ISW":
        return "ISW.%s %s, %d(%s)" % (dest, fvi(it), s11(imm11), fvi(is_))
    if name in ("IADDIU", "ISUBIU"):
        return "%s %s, %s, 0x%04x" % (name, fvi(it), fvi(is_), imm15)
    if name in ("FCEQ", "FCSET", "FCAND", "FCOR"):
        return "%s %s, 0x%06x" % (name, fvi(1), code & 0xFFFFFF)
    if name in ("FSEQ", "FSSET", "FSAND", "FSOR"):
        if name == "FSSET":
            return "FSSET 0x%03x" % (code & 0xFFF)
        return "%s %s, 0x%03x" % (name, fvi(it), code & 0xFFF)
    if name == "FMEQ":
        return "FMEQ %s, %s" % (fvi(it), fvi(is_))
    if name in ("FMAND", "FMOR"):
        return "%s %s, %s" % (name, fvi(it), fvi(is_))
    if name == "FCGET":
        return "FCGET %s" % fvi(it)
    if name in ("B", "BAL"):
        return "%s 0x%03x%s" % (name, pc + 1 + s11(imm11),
                                (", " + fvi(it)) if name == "BAL" else "")
    if name in ("IBEQ", "IBNE", "IBLTZ", "IBGTZ", "IBLEZ", "IBGEZ"):
        return "%s 0x%03x, %s, %s" % (name, pc + 1 + s11(imm11), fvi(it), fvi(is_))
    if name == "JR":
        return "JR %s" % fvi(is_)
    if name == "JALR":
        return "JALR %s, %s" % (fvi(it), fvi(is_))
    return "BAD_LOWER(%08x)" % code

def dis_pair(up, lo, pc):
    return "%-38s | %s" % (dis_upper(up), dis_lower(lo, pc))

# ------------------------------------------------------------------ main ----
def main():
    args = sys.argv[1:]
    path = args[0]
    data = open(path, "rb").read()
    if "--elf" in args:
        # .vutext at 0x13bb80 (addr 0x241c80, 0x2880 bytes), overlays from the
        # section table (offsets 0x207588, 0x207d88, 0x208588, 0x208d88,
        # 0x209588, 0x209b28; sizes 0x800 x4, 0x5a0, 0x268).
        chunks = [("vutext (EE 0x241c80)", 0x13bb80, 0x2880)]
        offs = [0x207588, 0x207d88, 0x208588, 0x208d88, 0x209588, 0x209b28]
        sizes = [0x800, 0x800, 0x800, 0x800, 0x5A0, 0x268]
        vus = [0x800, 0x1000, 0x1800, 0x2000, "?", 0x0]
        for i, (o, s) in enumerate(zip(offs, sizes)):
            chunks.append(("DVP overlay %d (section 0x%x)" % (i, o), o, s))
        for title, off, size in chunks:
            print("=" * 100)
            print("== %s, 0x%x bytes (%d qwords)" % (title, size, size // 8))
            print("=" * 100)
            dump(data[off:off + size])
        return
    base = int(args[args.index("--base") + 1]) if "--base" in args else 0
    n = int(args[args.index("--qwords") + 1]) if "--qwords" in args else len(data) // 8
    dump(data[base * 8:base * 8 + n * 8], base)

def dump(buf, base=0):
    for i in range(len(buf) // 8):
        w0, w1 = struct.unpack_from("<II", buf, i * 8)
        lo, up = w0, w1  # low word = lower op, high word = upper op
        print("%03x %08x_%08x  %s" % (base + i, up, lo, dis_pair(up, lo, base + i)))

if __name__ == "__main__":
    main()
