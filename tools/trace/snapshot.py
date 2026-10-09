"""Consistent EE memory snapshots from a running PCSX2 for offline comparison (our own code).

PINE opcode 9 (SaveState, u8 slot; PINE spec, *sourced* as in pine.py) makes PCSX2 write a save state; a .p2s is a zip
whose `eeMemory.bin` is the 32 MB EE RAM (*measured*: listed below when run). Reading memory from the state instead of
over PINE gives one instant for all values (PINE reads one word per round trip, several game frames for a matrix
palette).

Usage: python tools/trace/snapshot.py <slot> <out.bin>   (writes EE RAM to out.bin; use an unused slot, default 9)
"""
import glob
import os
import socket
import struct
import sys
import time
import zipfile

SSTATES = r"C:\Users\Yoda\Downloads\PS2\PCSX2\sstates"


def save_state(slot, port=28011):
    s = socket.create_connection(("127.0.0.1", port), timeout=10)
    s.sendall(struct.pack("<IBB", 6, 9, slot))
    hdr = s.recv(5)
    s.close()
    return hdr[4] == 0 if len(hdr) == 5 else False


def main():
    slot = int(sys.argv[1]) if len(sys.argv) > 1 else 9
    out = sys.argv[2] if len(sys.argv) > 2 else "work/ee_snapshot.bin"
    pattern = os.path.join(SSTATES, f"SLUS-20579 (*).{slot:02d}.p2s")
    before = {p: os.path.getmtime(p) for p in glob.glob(pattern)}
    t0 = time.time()
    if not save_state(slot):
        sys.exit("PINE SaveState failed")
    path = None
    while time.time() - t0 < 30:
        for p in glob.glob(pattern):
            if os.path.getmtime(p) > before.get(p, 0) and os.path.getmtime(p) >= t0 - 1:
                path = p
        if path:
            time.sleep(1.0)  # let PCSX2 finish writing
            break
        time.sleep(0.2)
    if not path:
        sys.exit("save state file did not appear")
    with zipfile.ZipFile(path) as z:
        names = z.namelist()
        mem = z.read("eeMemory.bin")
    with open(out, "wb") as f:
        f.write(mem)
    print(f"{path}: {len(names)} members, eeMemory.bin {len(mem)} bytes -> {out}")


if __name__ == "__main__":
    main()
