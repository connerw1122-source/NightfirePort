"""God mode and infinite ammo for the real game in PCSX2, through PINE (our own code; the same writes as
trace_record.py --god --ammo, without recording).

Every in-game frame: health (BLData+0x894) = 100.0 and armour (+0x8B0) = 50.0. Every 30 frames: each owned weapon's
clip (BLData+0x1B8 + w*0xC, s16; owned byte at +2) and each ammo type's spare rounds (+0x170 + a*2, s16) are topped
back up to the most seen this run. Survives level changes (BLData is re-read every frame).

Usage: python -I tools/trace/cheats.py [--level 07000006]   (Ctrl+C stops)
  --level  first load that level with the game's own loader (goto_level.py), then keep the cheats on.
PINE serves one client at a time: stop any other PINE user (the MCP bridge, a recorder) first.
"""
import os
import struct
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pine import Pine  # noqa: E402

GAMESTATE = 0x2A3768
PLAYERS = 0x2D88E0
F100 = struct.unpack("<I", struct.pack("<f", 100.0))[0]
F50 = struct.unpack("<I", struct.pack("<f", 50.0))[0]


def s16(v):
    return v - 0x10000 if v & 0x8000 else v


def main():
    a = sys.argv[1:]
    if "--level" in a:
        lv = a[a.index("--level") + 1]
        subprocess.run([sys.executable, "-I", os.path.join(os.path.dirname(os.path.abspath(__file__)), "goto_level.py"), lv], check=True)
    p = Pine()
    best_clip, best_spare = {}, {}
    last, bl_prev = None, None
    print("god mode + infinite ammo on. Ctrl+C to stop.", flush=True)
    while True:
        try:
            fr = p.read32(GAMESTATE + 0x34)
        except OSError:
            time.sleep(1)
            continue
        if fr == last:
            time.sleep(0.002)
            continue
        last = fr
        obj = p.read32(PLAYERS)
        bl = p.read32(obj + 0xE0) if obj else 0
        if not bl:
            continue
        if bl != bl_prev:  # a new level: forget the old maxima
            best_clip, best_spare, bl_prev = {}, {}, bl
        p.write(bl + 0x894, F100, 4)
        p.write(bl + 0x8B0, F50, 4)
        if fr % 30:
            continue
        for w in range(1, 0x72):
            e = bl + 0x1B8 + w * 0xC
            if p.read_bytes(e + 2, 1)[0] != 1:
                continue
            clip = s16(p.read16(e))
            if clip > best_clip.get(w, 0):
                best_clip[w] = clip
            elif clip < best_clip.get(w, 0):
                p.write(e, best_clip[w], 2)
        for k in range(0x24):  # 36 types: +0x170..+0x1B8 (the clip table starts there)
            e = bl + 0x170 + k * 2
            n = s16(p.read16(e))
            if n > best_spare.get(k, 0):
                best_spare[k] = n
            elif n < best_spare.get(k, 0):
                p.write(e, best_spare[k], 2)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
