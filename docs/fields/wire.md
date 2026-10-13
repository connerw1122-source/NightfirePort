# Wire (the grapple/balance wire, 8 bytes at obj+0x110; class 0x2C)

Proof levels: see [README.md](README.md). Instance size 8 from `Wire_Create` 0x1981d8:13
(`control_create_object(8,…)`); class 0x2C at 0x1981d8:26. Placement 0x3E
(0x1d04f0:252). This doc formalises docs/atlas/wire-grapple.md (which already carried the
disasm-level proof) into the field-map format. Level: ee_start_05 (07000005 at start).

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u16 | **wire kind** (param 1): 0 = hand-over-hand balance wire, 2 = zipline; 1 is refused by `Player_CollWire` (returns before mounting) | [M* writer 0x1981d8:21 (`*puVar1 = *(undefined2 *)(param_3 + 0x30)`); readers `Player_CollWire` 0x197130:100 (`uVar1 = *puVar2`, branches 0/1/2), 0x197130:130/152/160/169-203] |
| +0x02 | u16 | param 0 — not referenced by any wire code found (snapshot value 2) | [M store 0x1981d8:20; no reader — grep census in wire-grapple.md] |
| +0x04 | u16 | param 2 — not referenced by any wire code found (snapshot 0) | [M store 0x1981d8:22; no reader] |
| +0x06 | u16 | **the ride channel** (param 3): held 1 every tick Bond is on the wire, 0 on dismount/fall (`Player_Wire` 0x197658:106-111 and :160-163; also `Player_Creep` 0x196a44, `Player_Zipline`) | [M* store 0x1981d8:23; readers/writers `Player_Wire` 0x197658:108-111,160-163] |

The geometry is NOT in the instance: the wire's line comes from the object header at grab/ride
time — sphere centre obj+0x80..0x8B, radius obj+0x8C (`Vec_Spherical_2_Cartesian(radius − 2.0,
obj+0x54 + π/2, obj+0x58)` gives the endpoints ± around the centre; `Player_Wire` re-derives
with radius − 0.5 every frame) [M `Player_CollWire` 0x197130:111-127; `Player_Wire`
0x197658:114-131]. `Wire_Create` also NEGATES rot z (`param_2 + 8`) before creating
[0x1981d8:12] and runs `Controls_StraddleTest` (0x134778) once — that marks obj+0xF0 bit 0x8000
if the line straddles ≥ 2 cels and links the object into ForcedList [M 0x134778:38-57].

## Riders (every reader of the instance, complete)

- `Player_CollWire` 0x197130 (jump/hit mount): +0x00 (kind → substate 6 or 0xF + anim), then the
  header-derived line + the player's closest point.
- `Player_Wire` 0x197658 (the ride loop): +0x06 (hold/clear the channel); the ride math uses the
  header line, not the instance.
- `Player_Creep` 0x196a44, `Player_Zipline` (kind-2 ride): +0x06 (same channel contract).
No other function touches class-0x2C instances [M grep `\+ 0xe0` over the Wire/Creep/Zipline/
Grapple files + `param_offsets.py` census].

## Port check

`crates/nf-game/src/wire.rs` models kind (p1), param0 (p0), param2 (p2), channel (p3) and the
header-derived line — its own test pins the live 07000005 wire (`00 00 02 00 00 00 00 00`)
[M wire.rs:246-248]. **No mismatch found**; the port doc comment at wire.rs:10 carries the same
reading.

## [L-read] spot checks (work/ee_start_05.bin)

One class-0x2C object (the courtyard wire): instance u16s `00 00 | 02 00 | 00 00 | 00 00` at
0x016dc510 — kind 0, p0 = 2, p2 = 0, channel 0 [L; wire-grapple.md's table row]. The ride channel
is 0 at level start ✓ (nobody riding).

## Live checks

1. Find the wire live: `DynamicObjList 0x2705a0 → head+0x14 → obj+0xFF == 0x2C → obj+0xE0`.
2. Jump onto the wire and `watch` instance+0x06 — it must read 1 for every frame of the ride and
   drop to 0 on dismount/fall (the channel drives whatever listens on param 3, e.g. a trigger).
3. `poke` a kind-2 wire's +0x00 from 0 to 2 before jumping — the mount should become a zipline
   slide (the endpoint-half test + the 0x6000095 anim) instead of the balance walk.
