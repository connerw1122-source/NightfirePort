"""Record a reference trace from the real game in PCSX2 while the user plays (our own code; plan v2 §3).

Every in-game frame (GameState+0x34) it writes one line of numbers: the frame, the level, the pad (held buttons and
the four analog bytes, tSlot+0x122/+0x128), Bond's position and yaw (player obj+0x30/+0x54), health and armour
(BLData+0x894/+0x8B0), and every guard's state and position (DRONE_LIST walk: +0x10C, obj+0x30). Switch channels
(0x26fc90, 256 bytes) are logged only when one changes, as "ch <frame> <index> <old> <new>" lines. Numbers only:
no game bytes are stored.

--god keeps Bond alive by writing full health (100.0) and armour (50.0) into emulated RAM every frame, and refills
every owned weapon's clip to the fullest seen every 30 frames, so the user can play through without dying. That changes the run (damage reactions still happen, death never does), so the
trace header records it.

Usage: python -I tools/trace/trace_record.py <out.trace> [--god] [--ammo] [--alert] [--seconds N]
--ammo: infinite ammo (refill clips); leave it off when the recording must show shots being spent.
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
THE_DOORS = 0x316F70       # first door ext (DOORINFO); ext+4 next, ext+8 obj

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
    out, god, ammo = a[0], "--god" in a, "--ammo" in a
    alert = "--alert" in a  # append each drone's live alertness (Drone+0x50C) as a<value>
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
    inv = None
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
                    # The damage taken since the last refill (god mode would otherwise hide every hit): health below
                    # 100 or armour below 50 is a hit this frame.
                    hp0, ar0 = f32(p.read32(bl + 0x894)), f32(p.read32(bl + 0x8B0))
                    if hp0 < 100.0 or ar0 < 50.0:
                        f.write(f"hit {fr} health {100.0 - hp0:.2f} armour {50.0 - ar0:.2f}\n")
                    p.write(bl + 0x894, F100, 4)
                    p.write(bl + 0x8B0, F50, 4)
                # Infinite ammo (with --god): every 30 frames, top each owned weapon's clip (BLData+0x1B8+w*0xC, s16;
                # owned byte at +0x1BA) back up to the fullest clip seen for it this run.
                if ammo and bl and fr % 30 == 0:
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
                    if alert:
                        # Drone+0x50C: the live alertness 0..1 (docs/fields/drone.md group 3).
                        row.append(f"a{f32(p.read32(d + 0x50C)):.3f}")
                # Doors (TheDoors walk): state (obj+0xF4: 2 closed, 4 open, 8 opening, 0x10 closing) and the
                # auto-close quiet timer (ext+0x72, counts to 360).
                row.append("|")
                e, k = p.read32(THE_DOORS), 0
                while e and k < 64:
                    o = p.read32(e + 8)
                    row += [f"{p.read16(o + 0xF4) if o else 0:x}", str(p.read16(e + 0x72))]
                    e, k = p.read32(e + 4), k + 1
                f.write(" ".join(row) + "\n")
                # Inventory every 15 frames, as change events: spare rounds per ammo type (BLData+0x170 + a*2, s16)
                # and per weapon w the clip (BLData+0x1B8 + w*0xC, s16) and owned byte (+2).
                if bl and fr % 15 == 0:
                    raw = p.read_bytes(bl + 0x170, 0x1B4 + 0x72 * 0xC - 0x170)
                    spare = [int.from_bytes(raw[2 * k:2 * k + 2], 'little', signed=True) for k in range(33)]
                    weap = []
                    for w in range(0x72):
                        o = 0x1B8 + w * 0xC - 0x170
                        weap.append((int.from_bytes(raw[o:o + 2], 'little', signed=True), raw[o + 2]))
                    if inv is None:
                        owned = " ".join(f"{w:x}:{c}" for w, (c, o) in enumerate(weap) if o == 1)
                        f.write(f"inv0 {fr} spare {' '.join(map(str, spare))} owned {owned}\n")
                    else:
                        for k in range(33):
                            if spare[k] != inv[0][k]:
                                f.write(f"spare {fr} {k} {inv[0][k]} {spare[k]}\n")
                        for w in range(0x72):
                            if weap[w] != inv[1][w]:
                                f.write(f"weap {fr} {w:x} clip {inv[1][w][0]}->{weap[w][0]} owned {inv[1][w][1]}->{weap[w][1]}\n")
                    inv = (spare, weap)
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
