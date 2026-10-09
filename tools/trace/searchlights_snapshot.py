#!/usr/bin/env python3
"""Searchlight objects in the EE start snapshot (level 07000005).

Walks DynamicObjList (0x2705a0, head +0x14, next obj+0x14), finds class-0x22
objects (Searchlight_Create sets obj+0xff = 0x22) and dumps the instance data
(obj+0xe0) as derived from Searchlight_Create/Update (0x1cb4f8 / 0x1cb900):

  +0x00 u16 part type (0 mount, 1 head, 2 spot/light, 3 beam, 4 corona)
  +0x04 f32 phase (head) / seconds-not-seen (spot)
  +0x10 f32[4] placement rotation angles (rad); +0x14 = base yaw
  +0x20..+0x30 part pointers by type (mount/head/spot/beam/corona)
  +0x34 u8 on-channel, +0x35 u8 off-channel, +0x36 u8 alarm-channel
  +0x38 f32 half-range, +0x3c f32 mid (rad)

Object: +0x14 next, +0x30 pos, +0x50 pitch, +0x54 yaw, +0xd0 child, +0xd8 glist,
+0xe0 instance, +0xe8 scale, +0xec smoke phase, +0xf0 flags, +0xf4 state (u16),
+0xfa view flags, +0xfc drawn, +0xfe delete, +0xff class, +0x106 alpha.
"""
import struct
import sys

ram = open(sys.argv[1] if len(sys.argv) > 1 else "/home/z/my-project/NightfirePort/work/ee_start_05.bin", "rb").read()
M = 0x1FFFFFF
u = lambda a: struct.unpack_from("<I", ram, a & M)[0]
us = lambda a: struct.unpack_from("<H", ram, a & M)[0]
ub = lambda a: ram[a & M]
f = lambda a: struct.unpack_from("<f", ram, a & M)[0]

head = u(0x2705A0 + 0x14)
objs, searchlights, n = head, [], 0
while objs and n < 2000:
    if ub(objs + 0xFF) == 0x22:
        searchlights.append(objs)
    objs, n = u(objs + 0x14), n + 1

print("%d class-0x22 objects (of %d on the list)" % (len(searchlights), n))
PARTS = {0: "mount", 1: "head", 2: "spot", 3: "beam", 4: "corona", 5: "?", 6: "??"}
for o in searchlights:
    ins = u(o + 0xE0)
    t = us(ins)
    print("\nobj %08x part %d (%s) state %d pos (%8.3f,%8.3f,%8.3f) pitch %8.4f yaw %8.4f drawn %d"
          % (o, t, PARTS.get(t, "?"), us(o + 0xF4), f(o + 0x30), f(o + 0x34), f(o + 0x38), f(o + 0x50), f(o + 0x54), ub(o + 0xFC)))
    print("   rot (%.4f, %.4f, %.4f, %.4f) phase %10.3f half %7.4f mid %7.4f  chan on %d off %d alarm %d"
          % (f(ins + 0x10), f(ins + 0x14), f(ins + 0x18), f(ins + 0x1C), f(ins + 4), f(ins + 0x38), f(ins + 0x3C),
             ub(ins + 0x34), ub(ins + 0x35), ub(ins + 0x36)))
    for k, name in ((0x20, "mount"), (0x24, "head"), (0x28, "spot"), (0x2C, "beam"), (0x30, "corona")):
        p = u(ins + k)
        if p:
            print("   -> %-6s %08x part %d pos (%8.3f,%8.3f,%8.3f) state %d drawn %d"
                  % (name, p, us(u(p + 0xE0)), f(p + 0x30), f(p + 0x34), f(p + 0x38), us(p + 0xF4), ub(p + 0xFC)))
