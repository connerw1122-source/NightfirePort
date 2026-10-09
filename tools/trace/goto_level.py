"""Load a level in a running PCSX2 without the menus, using the game's own loader (our own code).

Does from outside what the pause menu's "restart mission" does (`ResetMap_LevelToLoad(level, 0, 1)`, ACTION.ELF
0x1d3800, *measured*): `IsWarmReset = 0`, `GameState+8 = level`, push game-flow state 3 (`ResetMap_Load`) on
`GameStateStack`, `GameState[0x40] = 1`. Only valid while the top game-flow state is 2 (in a level or the front end).
Then waits until `GameState+12` is the level and the in-game frame counter runs, and optionally snapshots EE RAM.

Usage: python tools/trace/goto_level.py <level hex, e.g. 07000005> [--launch] [--settle N] [--snapshot out.bin]
  --launch    start PCSX2 from the resume save state first if PINE does not answer
  --settle N  in-game frames to wait after the level runs before the snapshot (default 2)
"""
import os
import socket
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
from pine import Pine  # noqa: E402
import snapshot  # noqa: E402

GAMESTATE = 0x2A3768
STACK = 0x2C7398
STACK_INDEX = 0x30CECE
IS_WARM_RESET = 0x30D001
DRONE_LIST = 0x298E14
PCSX2 = r"C:\Users\Yoda\Downloads\PS2\PCSX2\pcsx2-qt.exe"
RESUME = r"C:\Users\Yoda\Downloads\PS2\PCSX2\sstates\SLUS-20579 (5B86BB62).resume.p2s"
ISO = r"C:\Users\Yoda\Downloads\PS2\007 - Nightfire (USA).iso"


def connect(launch):
    try:
        return Pine()
    except OSError:
        if not launch:
            sys.exit("PINE does not answer (PCSX2 not running?); use --launch")
    subprocess.Popen([PCSX2, "-statefile", RESUME, "--", ISO])
    t0 = time.time()
    while time.time() - t0 < 60:
        try:
            p = Pine()
            f0 = p.read32(GAMESTATE + 0x3C)
            time.sleep(0.5)
            if p.read32(GAMESTATE + 0x3C) != f0:
                return p
            p.close()
        except (OSError, IOError):
            time.sleep(0.5)
    sys.exit("PCSX2 did not start running the game within 60 s")


def top_state(p):
    idx = p.read16(STACK_INDEX)
    return idx, (p.read32(STACK + 4 * (idx - 1)) if idx else 0)


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    level = int(args[0], 16)
    launch = "--launch" in args
    settle = int(args[args.index("--settle") + 1]) if "--settle" in args else 2
    out = args[args.index("--snapshot") + 1] if "--snapshot" in args else None
    p = connect(launch)
    t0 = time.time()
    while True:
        idx, state = top_state(p)
        if state == 2:
            break
        if time.time() - t0 > 30:
            sys.exit(f"game-flow state is {state} (stack index {idx}), not 2")
        time.sleep(0.2)
    print(f"from level {p.read32(GAMESTATE + 12):08x}, game-flow stack index {idx}, state 2")
    p.write(IS_WARM_RESET, 0, 1)
    p.write(GAMESTATE + 8, level, 4)
    p.write(STACK + 4 * idx, 3, 4)
    p.write(STACK_INDEX, idx + 1, 2)
    p.write(GAMESTATE + 0x40, 1, 1)
    t0 = time.time()
    seen = set()
    while True:
        idx2, state = top_state(p)
        if state not in seen:
            seen.add(state)
            print(f"  {time.time() - t0:5.1f}s game-flow state {state}")
        if p.read32(GAMESTATE + 12) == level and state == 2 and p.read32(DRONE_LIST) != 0 or (
                p.read32(GAMESTATE + 12) == level and state == 2 and time.time() - t0 > 20):
            break
        if time.time() - t0 > 120:
            sys.exit(f"level did not start within 120 s (state {state})")
        time.sleep(0.1)
    start = p.read32(GAMESTATE + 0x34)
    while p.read32(GAMESTATE + 0x34) < start + settle:
        time.sleep(0.01)
    print(f"level {level:08x} running after {time.time() - t0:.1f}s; in-game frame {p.read32(GAMESTATE + 0x34)}")
    p.close()
    if out:
        snapshot.take(9, out)


if __name__ == "__main__":
    main()
