# The structure atlas

Plan v2 §2 (phase 0a). A typed reference for the runtime structures the port touches: one file per
struct, so that every `+0xNNN` in a Rust comment has a measured home. The Rust code names fields
after the atlas; a `+0xNNN` that does not exist here is a bug.

## Files

| File | Struct |
|---|---|
| `object.md` | the object/control list entry (`obj_tag`, 0x110-byte header) and the list machinery |
| `drone.md` | the Drone (`NDrone2` instance, 0xD20 bytes) |
| `drone-port-check.md` | every `+0xNNN` `crates/nf-game/src/drone.rs` mentions, checked against the atlas |
| `searchlight.md` | the searchlight family (class 0x22 parts, 0x40-byte instance; params 4/5, spot linking, G-036) |
| `wire-grapple.md` | the wire (type 0x3E → class 0x2C) and grapple (0x3A → 0x51), the player substates, `Player_SSGrapple` |
| `cel.md` | the cel/room (`cel_tag` 0xb0, ambient +0x9a/9c/9e, portals block 0x21, object→cel links) |
| `gs-leaf.md` | the per-object GS/lighting state (obj+0x100..0x106, TweakRGBA, the 0x800 path) and the character-brightness root cause |
| `render-port-check.md` | the disagreements with `nf-view` scene.rs/gpu.rs and `nf-game` searchlight.rs (and wire.rs data) |

Planned next (plan v2 §2 order): `BLData`/player, `weapon_data`, trigger ext, door, pickup,
script player, switch channels.

## Format (per struct)

1. **Name, size, allocation site** — the creating function and its address; who mallocs it and
   how big.
2. **Field table** — `offset | type | name | meaning | writers (fn@addr) | readers (fn@addr) | tag`.
   Only first-hand evidence goes in the table; a field nobody has measured yet says so.
3. **Placement-param → field mapping** — for structs created from placements: the
   `*_Create` reading `level_tag+0x2C+4k` (param k) → which field it lands in.
4. **Snapshot check** — at least 3 fields read from a real instance in `work/ee_start_05.bin`
   (level 07000005 at start; file offset = EE address), with the address and the value, and why
   the value makes sense.

## Evidence tags

- **[M]** measured in code — the decompile at the given address, or its disassembly
  (ACTION.ELF SHA-1 `e7ffdcb3…`). The decompiler drops stores and arguments; where the C looks
  incomplete the disassembly is the truth. One lesson already paid for: Ghidra's global typing
  can skew displayed offsets (`DynamicObjList._20_4_` is really `head+0x14`, see object.md).
- **[D]** measured on the disc (bytes from the user's ISO).
- **[L]** read from a live EE snapshot (`work/ee_start_*.bin`), address + value given.
- **[S]** suggested / inferred — someone's reading, not yet re-measured. Keep these out of
  field tables; park them in the notes or UNKNOWNS.

## Rules for updating

- Every claim keeps its function name and address. No address, no claim.
- Snapshot values are raw hex plus the natural interpretation; a value that makes no sense is a
  finding, not an embarrassment — record it.
- The port-check file is regenerated whenever `drone.rs` gains a new offset: a mismatch there is
  a port bug (or a stale atlas entry), and it gets fixed or filed in GAPS, never ignored.
