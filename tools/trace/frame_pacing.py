"""Measure the on-foot engine's update rate live (UNKNOWNS U-013; docs/systems/input.md section 6).

Polls VBlankCount, GameCount, PS2FramesToSkip, FRAME_RATE_MUL (bits), PS2SkipCount and GameState+12 (level hashcode)
much faster than 60 Hz. Each change of GameCount is one game update (`main`: GameCount += PS2FramesToSkip), so
counting GameCount changes and the skip value at each one gives the exact share of 1- vs 2-vblank updates.
Addresses: ACTION.ELF .symtab (SHA-1 e7ffdcb3...).

Usage: python -I tools/trace/frame_pacing.py [seconds]      (prints a summary; no game memory is written)
"""
import collections
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pine import Pine, sample  # noqa: E402

VBLANK_COUNT, GAME_COUNT, FRAMES_TO_SKIP, FRAME_RATE_MUL, SKIP_COUNT = 0x30ca4c, 0x30c710, 0x30c70c, 0x30d0d8, 0x30c708
GAME_STATE_LEVEL = 0x2a3768 + 12


def main(seconds=10.0):
    p = Pine()
    level = p.read32(GAME_STATE_LEVEL)
    rows = sample(p, [(VBLANK_COUNT, 4), (GAME_COUNT, 4), (FRAMES_TO_SKIP, 4), (FRAME_RATE_MUL, 4), (SKIP_COUNT, 4)], seconds)
    p.close()
    updates = collections.Counter()
    muls = collections.Counter()
    prev_gc = rows[0][2]
    for _t, _vb, gc, skip, mul, _sc in rows[1:]:
        if gc != prev_gc:
            updates[gc - prev_gc] += 1
            muls[round(struct.unpack("<f", struct.pack("<I", mul))[0], 3)] += 1
            prev_gc = gc
    dt = rows[-1][0] - rows[0][0]
    vb = rows[-1][1] - rows[0][1]
    n = sum(updates.values())
    print(f"level hashcode {level:#010x}; {len(rows)} samples in {dt:.2f} s ({len(rows) / dt:.0f} Hz)")
    print(f"vblanks {vb} ({vb / dt:.2f}/s); updates seen {n} ({n / dt:.2f}/s)")
    print(f"GameCount step per update (= vblanks per update): {dict(sorted(updates.items()))}")
    print(f"FRAME_RATE_MUL at those updates: {dict(sorted(muls.items()))}")
    if vb and n:
        print(f"mean vblanks per update {vb / n:.3f}  ->  effective {60 * n / vb:.1f} updates per 60 vblanks")


if __name__ == "__main__":
    main(float(sys.argv[1]) if len(sys.argv) > 1 else 10.0)
