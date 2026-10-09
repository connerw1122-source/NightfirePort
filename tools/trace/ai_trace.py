"""Record a guard's AI state every game frame in a running PCSX2 (our own code) — ground truth for the nf-game port.

Optionally teleports Bond first (writes the player object's position/rotation in emulated RAM). Samples once per
in-game frame (GameState+0x34) the nearest drone's (or --drone N's) fields listed in FIELDS, plus Bond's position.

Usage: python tools/trace/ai_trace.py [--teleport x y z yaw] [--drone N] [--frames 600] [--out work/ai_trace.tsv]
                                      [--field name:offset:type ...]   (extra Drone fields, type u32/u16/u8/f32/i16)
"""
import os
import struct
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
from pine import Pine  # noqa: E402

GAMESTATE = 0x2A3768
DRONE_LIST = 0x298E14
PLAYERS = 0x2D88E0
# Drone-relative unless prefixed obj./anim.; offsets measured in WORKLOG 0016–0018 or from the decomp.
FIELDS = [
    ("sm_state", 0x10C, "u16"), ("danim", 0x56C, "u16"), ("script", 0x57C, "u32"), ("state_req", 0x558, "u16"), ("anim_state", 0x56A, "u16"),
    ("A0", 0xA0, "f32"), ("alert50c", 0x50C, "f32"), ("fire_flag", 0x40, "u8"), ("last_fire", 0xBC8, "u32"),
    ("clip", 0xBBC, "i16"), ("mode", 0xC5, "u8"), ("flags4f8", 0x4F8, "u32"),
    ("obj.x", 0x30, "f32"), ("obj.y", 0x34, "f32"), ("obj.z", 0x38, "f32"), ("obj.yaw", 0x54, "f32"),
]


def read(p, addr, typ):
    if typ == "f32":
        return struct.unpack("<f", struct.pack("<I", p.read32(addr)))[0]
    if typ == "u32":
        return p.read32(addr)
    if typ == "u16":
        return p.read16(addr)
    if typ == "i16":
        v = p.read16(addr)
        return v - 0x10000 if v & 0x8000 else v
    if typ == "u8":
        return p.read_many([(addr, 1)])[0]
    raise ValueError(typ)


def drones(p):
    out, d = [], p.read32(DRONE_LIST)
    while d and len(out) < 128:
        out.append(d)
        d = p.read32(d + 4)
    return out


def main():
    a = sys.argv[1:]
    p = Pine()
    fields = list(FIELDS)
    for i, x in enumerate(a):
        if x == "--field":
            name, off, typ = a[i + 1].split(":")
            fields.append((name, int(off, 16), typ))
    player = p.read32(PLAYERS)
    if "--teleport" in a:
        i = a.index("--teleport")
        x, y, z, yaw = (float(v) for v in a[i + 1:i + 5])
        for k, v in enumerate((x, y, z)):
            p.write(player + 0x30 + 4 * k, struct.unpack("<I", struct.pack("<f", v))[0], 4)
        p.write(player + 0x54, struct.unpack("<I", struct.pack("<f", yaw))[0], 4)
    frames = int(a[a.index("--frames") + 1]) if "--frames" in a else 600
    out = a[a.index("--out") + 1] if "--out" in a else "work/ai_trace.tsv"
    ds = drones(p)
    ppos = [read(p, player + 0x30 + 4 * k, "f32") for k in range(3)]
    if "--drone" in a:
        d = ds[int(a[a.index("--drone") + 1])]
    else:
        def dist(d):
            o = p.read32(d + 0xC)
            return sum((read(p, o + 0x30 + 4 * k, "f32") - ppos[k]) ** 2 for k in range(3))
        d = min(ds, key=dist)
    print(f"level {p.read32(GAMESTATE + 12):08x}, Bond at {ppos}, tracing drone {d:08x} ({len(ds)} drones)")
    with open(out, "w") as f:
        f.write("frame\t" + "\t".join(n for n, _, _ in fields) + "\tbond.x\tbond.y\tbond.z\tbond.health\n")
        last, n, t0 = None, 0, time.time()
        while n < frames and time.time() - t0 < frames / 30 + 10:
            fr = p.read32(GAMESTATE + 0x34)
            if fr == last:
                continue
            last = fr
            obj = p.read32(d + 0xC)
            row = [fr]
            for name, off, typ in fields:
                base = obj if name.startswith("obj.") else d
                row.append(read(p, base + off, typ))
            row += [read(p, player + 0x30 + 4 * k, "f32") for k in range(3)]
            pd = p.read32(player + 0xE0)
            row += [read(p, pd + 0x894, "f32") if pd else 0.0]
            f.write("\t".join(f"{v:.4f}" if isinstance(v, float) else (f"{v:x}" if isinstance(v, int) and v > 0xFFFF else str(v)) for v in row) + "\n")
            n += 1
    print(f"{n} frames -> {out}")


if __name__ == "__main__":
    main()
