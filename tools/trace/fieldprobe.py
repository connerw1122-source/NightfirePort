"""Live field checks on the real game in PCSX2 over PINE (our own code). The proof step of the field map
(docs/fields/README.md): a field's meaning is [L-verified] only once poking or watching it in the running game shows
the effect the static reading predicts.

PINE serves one client at a time: stop the MCP bridge, cheats.py or any recorder first. Addresses are EE addresses.

Commands (python -I tools/trace/fieldprobe.py <cmd> ...):
  dump  <addr> <len>                      hex dump, 16 bytes a row
  find  <hex bytes | "text">  [lo hi]     every address of a byte pattern or ASCII text in [lo, hi) (default all RAM)
  ptrs  <addr> [lo hi]                    every 4-aligned word in RAM that holds <addr> (who points here)
  watch <addr> <width> <frames>           print the value each in-game frame it changes
  poke  <addr> <width> <value> [--hold N] [--shot out.png] [--keep]
                                          write the value (rewritten every frame for N frames when --hold), screenshot
                                          the game window with the value pinned, then restore the original unless --keep.
                                          A field the game recomputes every frame can't be poked directly: poke its input
                                          (the HUD's +0x38 is rewritten by HUD_UpdateAmmoPane; its input BL+0x95E isn't)
  diff  <addr> <len> <frames>             snapshot, wait N in-game frames, snapshot, print the changed offsets
  shot  <out.png>                         screenshot the game window
Values and addresses take 0x hex. Frame waits follow GameState+0x34 (the in-game frame counter).
"""
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pine import Pine  # noqa: E402
from pcsx2_shot import GRAB, WINDOW  # noqa: E402

GAMESTATE = 0x2A3768
RAM_LO, RAM_HI = 0x100000, 0x2000000


def num(s):
    return int(s, 0)


def frame(p):
    return p.read32(GAMESTATE + 0x34)


def wait_frames(p, n, each=None):
    f0 = frame(p)
    last = f0
    while True:
        f = frame(p)
        if f != last:
            last = f
            if each:
                each(f)
        if f - f0 >= n:
            return
        time.sleep(0.002)


def read(p, a, n):
    out = bytearray()
    while n > 0:
        k = min(n, 0x4000)
        out += p.read_bytes(a, k)
        a += k
        n -= k
    return bytes(out)


def read_val(p, a, w):
    return {1: lambda: p.read_bytes(a, 1)[0], 2: lambda: p.read16(a), 4: lambda: p.read32(a)}[w]()


def shot(out):
    out = os.path.abspath(out)
    ps = GRAB.replace("TITLE", WINDOW).replace("OUT", out.replace("'", "''"))
    r = subprocess.run(["powershell", "-NoProfile", "-Command", ps], capture_output=True, text=True)
    print(r.stdout.strip() or r.stderr.strip())


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    cmd = a[0]
    if cmd == "shot":
        shot(a[1])
        return
    p = Pine()
    if cmd == "dump":
        base, n = num(a[1]), num(a[2])
        data = read(p, base, n)
        for o in range(0, n, 16):
            row = data[o:o + 16]
            print(f"{base + o:08x}  {row.hex(' ')}  {''.join(chr(c) if 32 <= c < 127 else '.' for c in row)}")
    elif cmd in ("find", "ptrs"):
        if cmd == "find":
            pat = a[1].encode() if not all(c in "0123456789abcdefABCDEF " for c in a[1]) else bytes.fromhex(a[1])
        else:
            pat = num(a[1]).to_bytes(4, "little")
        lo, hi = (num(a[2]), num(a[3])) if len(a) > 3 else (RAM_LO, RAM_HI)
        data = read(p, lo, hi - lo)
        i = data.find(pat)
        n = 0
        while i >= 0 and n < 200:
            if cmd == "find" or (lo + i) % 4 == 0:
                print(f"{lo + i:08x}")
                n += 1
            i = data.find(pat, i + 1)
    elif cmd == "watch":
        addr, w, frames = num(a[1]), num(a[2]), num(a[3])
        state = {"v": read_val(p, addr, w)}
        print(f"f{frame(p)}: {state['v']:#x}")

        def each(f):
            v = read_val(p, addr, w)
            if v != state["v"]:
                print(f"f{f}: {state['v']:#x} -> {v:#x}")
                state["v"] = v
        wait_frames(p, frames, each)
    elif cmd == "poke":
        addr, w, val = num(a[1]), num(a[2]), num(a[3])
        hold = num(a[a.index("--hold") + 1]) if "--hold" in a else 0
        out = a[a.index("--shot") + 1] if "--shot" in a else None
        orig = read_val(p, addr, w)
        p.write(addr, val, w)
        if hold:
            wait_frames(p, hold, lambda f: p.write(addr, val, w))
        else:
            wait_frames(p, 2)
        if out:
            # Keep the value pinned while the window comes to the front and is captured (~0.5-1 s): the game may
            # count it down or rewrite it in between (a timer did, the first live check).
            ps = GRAB.replace("TITLE", WINDOW).replace("OUT", os.path.abspath(out).replace("'", "''"))
            proc = subprocess.Popen(["powershell", "-NoProfile", "-Command", ps], stdout=subprocess.PIPE, text=True)
            while proc.poll() is None:
                p.write(addr, val, w)
                time.sleep(0.01)
            print(proc.stdout.read().strip())
        print(f"{addr:08x} w{w}: {orig:#x} -> {val:#x} (held {hold} frames)")
        if "--keep" not in a:
            p.write(addr, orig, w)
            print(f"restored {orig:#x}")
    elif cmd == "diff":
        base, n, frames = num(a[1]), num(a[2]), num(a[3])
        before = read(p, base, n)
        wait_frames(p, frames)
        after = read(p, base, n)
        for o in range(n):
            if before[o] != after[o]:
                print(f"+{o:#06x} ({base + o:08x}): {before[o]:02x} -> {after[o]:02x}")
    else:
        sys.exit(__doc__)
    p.close()


if __name__ == "__main__":
    main()
