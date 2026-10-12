"""Every instruction that touches a memory range in the real game: PCSX2 (the GDB-server build, docs/fields/README.md)
over the GDB Remote Serial Protocol (our own client; the server is the snowyegret23/pcsx2 fork's DebugTools/GDBServer).

The fork's custom commands travel as `qPcsx2:<hex of "verb args">`. A watchpoint in "break" mode pauses the emulated
EE at each access; this client records the PC (the accessing instruction), names it with ACTION.ELF's symbol table,
and continues at once. ("log" mode only prints in PCSX2 developer builds, so break mode is the reliable one.)

Usage: python -I tools/trace/gdbwatch.py <start> <end> <read|write|readwrite> [--hits N] [--seconds S]
  Prints a table: PC, function (+offset), hit count. Example (the HUD switch timer BL+0x95E, 1 byte):
    python -I tools/trace/gdbwatch.py 0x016cfa4e 0x016cfa4f write --seconds 20
The game must be running in the GDB-server PCSX2 (C:\\Users\\Yoda\\Downloads\\PS2\\PCSX2-gdb), EE server on 10501.
"""
import bisect
import os
import socket
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SYMTAB = os.path.join(HERE, "..", "..", "work", "action-symtab.tsv")


class Gdb:
    def __init__(self, port=10501, host="127.0.0.1", timeout=10.0):
        self.s = socket.create_connection((host, port), timeout=timeout)
        self.buf = b""

    def _read_packet(self):
        while True:
            i = self.buf.find(b"$")
            j = self.buf.find(b"#", i + 1) if i >= 0 else -1
            if i >= 0 and j >= 0 and len(self.buf) >= j + 3:
                body = self.buf[i + 1:j]
                self.buf = self.buf[j + 3:]
                self.s.sendall(b"+")
                return body.decode("latin-1")
            chunk = self.s.recv(4096)
            if not chunk:
                raise ConnectionError("GDB server closed")
            self.buf += chunk.replace(b"+", b"") if not self.buf else chunk

    def send(self, payload):
        data = payload.encode("latin-1")
        csum = sum(data) & 0xFF
        self.s.sendall(b"$" + data + b"#" + f"{csum:02x}".encode())

    def cmd(self, payload):
        self.send(payload)
        return self._read_packet()

    def pcsx2(self, text):
        r = self.cmd("qPcsx2:" + text.encode().hex())
        try:
            return bytes.fromhex(r).decode("utf-8", "replace")
        except ValueError:
            return r

    def close(self):
        self.s.close()


def load_symbols():
    syms = []
    try:
        for line in open(SYMTAB, encoding="utf-8"):
            f = line.rstrip("\n").split("\t")
            if len(f) >= 6 and f[2] == "2":  # functions
                syms.append((int(f[0], 16), int(f[1]), f[5]))
    except OSError:
        pass
    syms.sort()
    return syms


def name(syms, pc):
    i = bisect.bisect_right([s[0] for s in syms], pc) - 1
    if i >= 0:
        a, size, n = syms[i]
        if pc < a + max(size, 1):
            return f"{n.split('__')[0]}+{pc - a:#x}"
    return "?"


def main():
    a = sys.argv[1:]
    if len(a) < 3:
        sys.exit(__doc__)
    start, end, kind = int(a[0], 0), int(a[1], 0), a[2]
    max_hits = int(a[a.index("--hits") + 1]) if "--hits" in a else 2000
    seconds = float(a[a.index("--seconds") + 1]) if "--seconds" in a else 10.0
    syms = load_symbols()
    g = Gdb()
    print(g.pcsx2("status"))
    print(g.pcsx2(f"debug_set_watchpoint {start:#x} {end:#x} {kind} break"))
    hits = {}
    t0 = time.time()
    n = 0
    try:
        g.send("c")
        while n < max_hits and time.time() - t0 < seconds:
            g.s.settimeout(max(0.5, seconds - (time.time() - t0)))
            try:
                stop = g._read_packet()
            except socket.timeout:
                break
            if not stop or stop[0] not in "TS":
                continue
            # The PC is GDB register 37 (0x25), little-endian hex. (`debug_registers` overflows the server's
            # packet buffer: every 128-bit register of every category.)
            pc = int.from_bytes(bytes.fromhex(g.cmd("p25")[:8]), "little")
            hits[pc] = hits.get(pc, 0) + 1
            n += 1
            g.send("c")
    finally:
        g.s.settimeout(5.0)
        # Stop the run cleanly: interrupt if running, remove the watchpoint, resume.
        try:
            g.s.sendall(b"\x03")
        except Exception:
            pass
        # Drain every pending packet (the interrupt's stop reply, and a hit's stop reply that arrived after the
        # last read) so the next command's reply isn't shifted by one.
        g.s.settimeout(1.0)
        while True:
            try:
                g._read_packet()
            except (socket.timeout, OSError, ConnectionError):
                break
        g.buf = b""
        g.s.settimeout(5.0)
        print(g.pcsx2(f"debug_remove_watchpoint {start:#x} {end:#x}"))
        # Leave the game running: the server's `D` (detach) PAUSES the VM, so resume explicitly instead.
        print("resume:", g.pcsx2("resume"))
        g.close()
    print(f"{n} hits in {time.time() - t0:.1f}s, {len(hits)} distinct instructions")
    for pc, c in sorted(hits.items(), key=lambda kv: -kv[1]):
        print(f"  {pc:08x}  {name(syms, pc):48s} {c}")


if __name__ == "__main__":
    main()
