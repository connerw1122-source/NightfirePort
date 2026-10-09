"""Drones in an EE RAM snapshot (snapshot.py): position, skin, danim, script, height state. Our own code.
Usage: python tools/trace/drones_snapshot.py <ee.bin>"""
import struct
import sys

ram = open(sys.argv[1], "rb").read()
u = lambda a: struct.unpack_from("<I", ram, a & 0x1FFFFFF)[0]
f = lambda a: struct.unpack_from("<f", ram, a & 0x1FFFFFF)[0]
print("level %08x" % u(0x2A3768 + 12))
d, n = u(0x298E14), 0
while d and n < 128:
    obj = u(d + 0xC)
    print("drone %08x skin %08x danim %4x script %08x A0 %.4f pos (%.3f, %.3f, %.3f) obj flags %08x"
          % (d, u(d + 0xC34), u(d + 0x56C) & 0xFFFF, u(d + 0x57C), f(d + 0xA0), f(obj + 0x30), f(obj + 0x34), f(obj + 0x38), u(obj + 0xF0)))
    d, n = u(d + 4), n + 1
print(n, "drones")
