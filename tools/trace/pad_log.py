"""Log pad state and the resulting game actions live (UNKNOWNS U-010, U-011; docs/systems/input.md sections 2-3).

Every time something changes, writes one line: time, slot-0 buttons held (tSlot+0x122), slot-0 analog bytes after the
game's dead zone (tSlot+0x128..0x12b), player-0 controller style (PlayerSetting+0xe) and the non-zero action floats
(PlayerSetting+0x14, 40 floats). Read-only. Addresses: ACTION.ELF .symtab (SHA-1 e7ffdcb3...).

Usage: python -I tools/trace/pad_log.py <seconds> <out.tsv>
"""
import os
import struct
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pine import Pine  # noqa: E402

T_SLOT, PLAYER_SETTING = 0x245680, 0x2a38c8


def main(seconds, out):
    p = Pine()
    t0, last = time.perf_counter(), None
    with open(out, "w", newline="\n") as f:
        f.write("t\tbuttons\trx\try\tlx\tly\tstyle\tactions\n")
        while (t := time.perf_counter() - t0) < seconds:
            slot = p.read_bytes(T_SLOT + 0x120, 12)            # 0x120 new, 0x122 held, 0x124 pad buf, 0x128 sticks
            ps = p.read_bytes(PLAYER_SETTING, 0x14 + 40 * 4)
            buttons = struct.unpack_from("<H", slot, 2)[0]
            rx, ry, lx, ly = slot[8:12]
            style = struct.unpack_from("<H", ps, 0xe)[0]
            acts = struct.unpack_from("<40f", ps, 0x14)
            nz = " ".join(f"{i}:{v:+.3f}" for i, v in enumerate(acts) if v != 0.0)
            state = (buttons, rx, ry, lx, ly, style, nz)
            if state != last:
                f.write(f"{t:.3f}\t{buttons:#06x}\t{rx}\t{ry}\t{lx}\t{ly}\t{style}\t{nz}\n")
                f.flush()
                last = state
    p.close()


if __name__ == "__main__":
    main(float(sys.argv[1]), sys.argv[2])
