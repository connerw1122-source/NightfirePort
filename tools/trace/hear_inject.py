"""Make a sound for the drones' hearing in a running PCSX2 and trace a guard's reaction (our own code; U-033).

The hearing list (`DLISTINFO` 0x349b00, in-use head 0x349b0c) holds every playing sound: +4 next, +0x10 position,
+0x34 id, +0x40 inner, +0x44 outer, +0x48 weight, +0x50 state byte (2, 3 = skipped). `Drone_InitComms` runs
`DroneFunc_HandleSoundAlerts` for every drone, then `Sound_ZeroAlertness` zeroes the weight of every non-looped sound,
so each sound alerts once. This borrows one in-use entry (a looped ambient sound, never heard): gives it a non-looped
id, a weight and a position, so the next InitComms hears it once; then restores the entry.

Usage: python tools/trace/hear_inject.py --list
       python tools/trace/hear_inject.py --near x y z [--at dx dy dz] [--id 1] [--weight 75 --inner 2 --outer 30]
                                         [--frames 240] [--out work/hear_trace.tsv]
"""
import os
import struct
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
from pine import Pine  # noqa: E402
from ai_trace import GAMESTATE, PLAYERS, drones, read  # noqa: E402

HEAD = 0x349B0C
SFX = 0x2C94A0


def f2u(v):
    return struct.unpack("<I", struct.pack("<f", v))[0]


def entries(p):
    out, e = [], p.read32(HEAD)
    while e and len(out) < 256:
        sid = p.read32(e + 0x34)
        looped = p.read_many([(SFX + (sid & 0xFFFFF) * 0x18 + 0x14, 1)])[0] if (sid & 0xFFFFF) < 0x60D else 0
        out.append(dict(addr=e, id=sid, state=p.read_many([(e + 0x50, 1)])[0], looped=looped,
                        pos=[read(p, e + 0x10 + 4 * k, "f32") for k in range(3)],
                        inner=read(p, e + 0x40, "f32"), outer=read(p, e + 0x44, "f32"), weight=read(p, e + 0x48, "f32")))
        e = p.read32(e + 4)
    return out


def main():
    a = sys.argv[1:]
    p = Pine()
    ents = entries(p)
    if "--list" in a or "--near" not in a:
        for e in ents:
            print(f"{e['addr']:08x} id {e['id']:#x} state {e['state']} looped {e['looped']} pos {[round(v, 2) for v in e['pos']]}"
                  f" inner {e['inner']} outer {e['outer']} weight {e['weight']}")
        return

    def opt(name, n, default):
        if name not in a:
            return default
        i = a.index(name)
        vals = [float(v) for v in a[i + 1:i + 1 + n]]
        return vals if n > 1 else vals[0]
    q = opt("--near", 3, None)
    at = opt("--at", 3, [1.0, 0.0, 0.0])
    sid = int(opt("--id", 1, 1))
    weight, inner, outer = opt("--weight", 1, 75.0), opt("--inner", 1, 2.0), opt("--outer", 1, 30.0)
    frames = int(opt("--frames", 1, 240))
    out = a[a.index("--out") + 1] if "--out" in a else "work/hear_trace.tsv"
    host = next((e for e in ents if e["looped"] and e["state"] not in (2, 3)), None)
    if host is None:
        sys.exit("no looped sound in the hearing list to borrow")

    def dist_q(d):
        o = p.read32(d + 0xC)
        return sum((read(p, o + 0x30 + 4 * k, "f32") - q[k]) ** 2 for k in range(3))
    d = min(drones(p), key=dist_q)
    obj = p.read32(d + 0xC)
    gpos = [read(p, obj + 0x30 + 4 * k, "f32") for k in range(3)]
    player = p.read32(PLAYERS)
    bond = [read(p, player + 0x30 + 4 * k, "f32") for k in range(3)]
    spos = [gpos[k] + at[k] for k in range(3)]
    saved = {o: p.read32(host["addr"] + o) for o in (0x10, 0x14, 0x18, 0x34, 0x40, 0x44, 0x48)}
    print(f"level {p.read32(GAMESTATE + 12):08x}; guard {d:08x} at {gpos}; Bond at {bond}; borrowing {host['addr']:08x}"
          f" (id {host['id']:#x}) as id {sid:#x} weight {weight} inner {inner} outer {outer} at {spos}")
    fields = [("sm_state", 0x10C, "u16"), ("anim_state", 0x56A, "u16"), ("danim", 0x56C, "u16"), ("alert50c", 0x50C, "f32"),
              ("loud510", 0x510, "f32"), ("share514", 0x514, "f32"), ("noise43", 0x43, "u8"), ("heard_b38", 0xB38, "u32")]
    rows = []
    start = p.read32(GAMESTATE + 0x34)
    for k, v in enumerate(spos):
        p.write(host["addr"] + 0x10 + 4 * k, f2u(v), 4)
    p.write(host["addr"] + 0x40, f2u(inner), 4)
    p.write(host["addr"] + 0x44, f2u(outer), 4)
    p.write(host["addr"] + 0x34, sid, 4)
    p.write(host["addr"] + 0x48, f2u(weight), 4)
    try:
        last, t0 = None, time.time()
        while len(rows) < frames and time.time() - t0 < frames / 30 + 10:
            fr = p.read32(GAMESTATE + 0x34)
            if fr == last:
                continue
            last = fr
            rows.append([fr - start] + [read(p, d + off, typ) for _, off, typ in fields] + [read(p, host["addr"] + 0x48, "f32")])
    finally:
        for o, v in saved.items():
            p.write(host["addr"] + o, v, 4)
    with open(out, "w") as f:
        f.write("frame\t" + "\t".join(n for n, _, _ in fields) + "\tweight\n")
        for r in rows:
            f.write("\t".join(f"{v:.4f}" if isinstance(v, float) else str(v) for v in r) + "\n")
    prev = None
    for r in rows:
        if r[1] != prev:
            print(f"frame +{r[0]}: state {r[1]:#x} anim {r[2]:#x} danim {r[3]} alert {r[4]:.3f} loud {r[5]:.3f} share {r[6]:.3f} noise {r[7]} weight {r[9]}")
            prev = r[1]
    print(f"{len(rows)} frames -> {out}")


if __name__ == "__main__":
    main()
