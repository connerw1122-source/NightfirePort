"""Record a reference trace from the real game in PCSX2 while the user plays (our own code; plan v2 §3).

Every in-game frame (GameState+0x34) it writes one line of numbers: the frame, the level, the pad (held buttons and
the four analog bytes, tSlot+0x122/+0x128), Bond's position and yaw (player obj+0x30/+0x54), health and armour
(BLData+0x894/+0x8B0), and every guard's state and position (DRONE_LIST walk: +0x10C, obj+0x30). Switch channels
(0x26fc90, 256 bytes) are logged only when one changes, as "ch <frame> <index> <old> <new>" lines. Numbers only:
no game bytes are stored.

--god keeps Bond alive by writing full health (100.0) and armour (50.0) into emulated RAM every frame, and refills
every owned weapon's clip to the fullest seen every 30 frames, so the user can play through without dying. That changes the run (damage reactions still happen, death never does), so the
trace header records it.

Usage: python -I tools/trace/trace_record.py <out.trace> [--god] [--seconds N]
Stop with Ctrl+C at any time; the file is complete up to the last frame.
Needs PCSX2 running with PINE enabled (slot 28011) and a level loaded.
"""
import os
import struct
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pine import Pine  # noqa: E402

GAMESTATE = 0x2A3768      # +0xC level hashcode, +0x34 in-game frame counter
DRONE_LIST = 0x298E14     # first drone; +4 next
PLAYERS = 0x2D88E0        # player obj_tag*; obj+0xE0 = BLData*
T_SLOT = 0x245680         # pad slot 0: +0x122 held buttons (u16), +0x128..0x12B analog bytes
SWITCH_CHANNELS = 0x26FC90

F100 = struct.unpack("<I", struct.pack("<f", 100.0))[0]
F50 = struct.unpack("<I", struct.pack("<f", 50.0))[0]


def f32(v):
    return struct.unpack("<f", struct.pack("<I", v))[0]


def drones(p):
    out, d = [], p.read32(DRONE_LIST)
    while d and len(out) < 128:
        out.append(d)
        d = p.read32(d + 4)
    return out


def main():
    a = sys.argv[1:]
    if not a or a[0].startswith("--"):
        print(__doc__)
        return
    out, god = a[0], "--god" in a
    seconds = float(a[a.index("--seconds") + 1]) if "--seconds" in a else None
    p = Pine()
    # Wait for the game: PINE answers before a disc is booted, but EE reads fail until then.
    while True:
        try:
            level = p.read32(GAMESTATE + 0xC)
            if level >> 24 == 0x07:
                break
        except OSError:
            pass
        print("waiting for the game to be in a level (boot the disc and start a mission)...")
        time.sleep(2)
    print(f"level {level:08x}; recording to {out}{' (god mode)' if god else ''}. Play; Ctrl+C to stop.")
    channels = p.read_bytes(SWITCH_CHANNELS, 256)
    t0, last, n = time.time(), None, 0
    max_clip = {}
    with open(out, "w") as f:
        f.write(f"# nightfire trace v1 level={level:08x} god={int(god)} start={time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write("# frame level pad ax ay bx by bond.x bond.y bond.z bond.yaw health armour | drone: state x y z ...\n")
        f.write("ch0 " + channels.hex() + "\n")
        try:
            while seconds is None or time.time() - t0 < seconds:
                fr = p.read32(GAMESTATE + 0x34)
                if fr == last:
                    continue
                last = fr
                lv = p.read32(GAMESTATE + 0xC)
                obj = p.read32(PLAYERS)
                bl = p.read32(obj + 0xE0) if obj else 0
                if god and bl:
                    p.write(bl + 0x894, F100, 4)
                    p.write(bl + 0x8B0, F50, 4)
                # Infinite ammo (with --god): every 30 frames, top each owned weapon's clip (BLData+0x1B8+w*0xC, s16;
                # owned byte at +0x1BA) back up to the fullest clip seen for it this run.
                if god and bl and fr % 30 == 0:
                    for w in range(1, 0x72):
                        e = bl + 0x1B8 + w * 0xC
                        if p.read_bytes(e + 2, 1)[0] != 1:
                            continue
                        clip = p.read16(e)
                        clip = clip - 0x10000 if clip & 0x8000 else clip
                        best = max_clip.get(w, 0)
                        if clip > best:
                            max_clip[w] = clip
                        elif clip < best:
                            p.write(e, best, 2)
                slot = p.read_bytes(T_SLOT + 0x122, 10)
                held = slot[0] | slot[1] << 8
                sticks = slot[6:10]
                pos = [f32(p.read32(obj + 0x30 + 4 * k)) for k in range(3)] if obj else [0.0] * 3
                yaw = f32(p.read32(obj + 0x54)) if obj else 0.0
                hp = f32(p.read32(bl + 0x894)) if bl else 0.0
                ar = f32(p.read32(bl + 0x8B0)) if bl else 0.0
                row = [str(fr), f"{lv:08x}", f"{held:04x}", *[str(b) for b in sticks],
                       *[f"{v:.4f}" for v in pos], f"{yaw:.4f}", f"{hp:.1f}", f"{ar:.1f}", "|"]
                for d in drones(p):
                    o = p.read32(d + 0xC)
                    st = p.read16(d + 0x10C)
                    dp = [f32(p.read32(o + 0x30 + 4 * k)) for k in range(3)] if o else [0.0] * 3
                    row += [str(st), *[f"{v:.3f}" for v in dp]]
                f.write(" ".join(row) + "\n")
                now = p.read_bytes(SWITCH_CHANNELS, 256)
                for i, (old, new) in enumerate(zip(channels, now)):
                    if old != new:
                        f.write(f"ch {fr} {i} {old} {new}\n")
                channels = now
                n += 1
                if n % 600 == 0:
                    f.flush()
                    print(f"  {n} frames, level {lv:08x}, Bond at ({pos[0]:.1f}, {pos[1]:.1f}, {pos[2]:.1f})")
        except KeyboardInterrupt:
            pass
    print(f"{n} frames -> {out}")


if __name__ == "__main__":
    main()
