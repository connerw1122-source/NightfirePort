"""Minimal PCSX2 PINE client (our own code) for fast read-only sampling of EE memory.

Protocol (PINE spec, https://projects.govanify.com/govanify/pine, MIT; *sourced* via
docs/research/native-port-methodology.md): TCP 127.0.0.1:<slot> on Windows. Request = u32 LE total length (incl.
these 4 bytes) + u8 opcode + args; reply = u32 LE total length + u8 result (0 = OK, 0xFF = FAIL) + payload.
Opcodes used: 0 Read8, 1 Read16, 2 Read32, 3 Read64 (arg: u32 address). Several requests may be batched in one
send; replies come back in order. Keep batches small: PCSX2 drops replies beyond ~7 in flight (*suggested*,
mcp-pine docs), so this client sends at most 6 per round trip.

Reads are the default. Writes (opcodes 4–7) change the emulated EE memory only — used by goto_level.py to drive the
game's own level loader; nothing outside PCSX2's RAM is touched.
"""
import socket
import struct
import time

READ = {1: 0, 2: 1, 4: 2, 8: 3}
# Measured (WORKLOG 0006): PCSX2 2.8.2 answers the first of several separately-framed requests sent in one write and
# then stalls, so requests go one at a time.
MAX_IN_FLIGHT = 1


class Pine:
    def __init__(self, slot=28011, host="127.0.0.1", timeout=5.0):
        self.s = socket.create_connection((host, slot), timeout=timeout)
        self.s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

    def close(self):
        self.s.close()

    def _recv_exact(self, n):
        buf = b""
        while len(buf) < n:
            chunk = self.s.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("PINE connection closed")
            buf += chunk
        return buf

    def read_many(self, reqs):
        """reqs: list of (address, width in bytes). Returns list of unsigned ints, in order."""
        out = []
        for i in range(0, len(reqs), MAX_IN_FLIGHT):
            batch = reqs[i:i + MAX_IN_FLIGHT]
            msg = b"".join(struct.pack("<IBI", 9, READ[w], a) for a, w in batch)
            self.s.sendall(msg)
            for a, w in batch:
                (length,) = struct.unpack("<I", self._recv_exact(4))
                body = self._recv_exact(length - 4)
                if body[0] != 0:
                    raise IOError(f"PINE FAIL reading {w} bytes at {a:#x}")
                out.append(int.from_bytes(body[1:1 + w], "little"))
        return out

    def read32(self, a):
        return self.read_many([(a, 4)])[0]

    def read16(self, a):
        return self.read_many([(a, 2)])[0]

    def write(self, a, value, width):
        """Writes `width` (1, 2, 4, 8) bytes of `value` at `a` in EE memory (PINE Write8/16/32/64)."""
        op = {1: 4, 2: 5, 4: 6, 8: 7}[width]
        msg = struct.pack("<IBI", 9 + width, op, a) + int(value).to_bytes(width, "little")
        self.s.sendall(msg)
        (length,) = struct.unpack("<I", self._recv_exact(4))
        body = self._recv_exact(length - 4)
        if body[0] != 0:
            raise IOError(f"PINE FAIL writing {width} bytes at {a:#x}")

    def read_bytes(self, a, n):
        """n bytes from a (4-byte reads; a and n need not be aligned to anything but are read via aligned words)."""
        base = a & ~3
        words = self.read_many([(base + 4 * k, 4) for k in range((a + n - base + 3) // 4)])
        raw = b"".join(struct.pack("<I", w) for w in words)
        return raw[a - base:a - base + n]


def sample(pine, reqs, seconds, period=0.0):
    """Polls `reqs` for `seconds`; returns [(t, values...)]."""
    rows, t0 = [], time.perf_counter()
    while (t := time.perf_counter() - t0) < seconds:
        rows.append((t, *pine.read_many(reqs)))
        if period:
            time.sleep(period)
    return rows
