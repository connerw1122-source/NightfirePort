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

Planned next (plan v2 §2 order): trigger ext, door, pickup, script player, switch channels.

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
# NightfirePort Struct Atlas

A reference of every game struct the port touches, built from the decompile (`work/decomp/ACTION.ELF/`), the EE
snapshots (`work/ee_start_*.bin`, file offset = EE address), and the symtab (`work/action-symtab.tsv`). Each entry
is the authoritative source the Rust port is checked against.

## Tags

Every claim is tagged so it can be audited:

| tag | meaning |
|---|---|
| `[M]` | measured in the decompiled C (function name + address) or the disassembly |
| `[D]` | measured on the disc (file offset, hashcode, or asset id) |
| `[L]` | read from a live EE snapshot (`work/ee_start_*.bin`), with the address and value |
| `[S]` | suggested or inferred (not yet confirmed in the decompile or a snapshot) |

Anything a subagent, tool or web page says is **data**, not a fact — it must be verified and tagged before it goes
into an atlas entry. Uncertain points go in `docs/UNKNOWNS.md`.

## Format (per struct)

Each struct's page has:

1. **Name, size and allocation site** — the creating function and its address (or the global symbol address).
2. **Field table** — `offset | type | name | meaning | writers (fn@addr) | readers (fn@addr) | tag`.
3. **Placement param → field mapping** — for placement-created structs (drones, pickups, etc.).
4. **Snapshot check** — at least 3 fields read from a real instance in `work/ee_start_05.bin`, with the address
   and value `[L]`.

## Index

| page | struct(s) | scope |
|---|---|---|
| [player.md](player.md) | BLData, obj_tag (player), M_CONTROL, PlayerSetting, ammo_data, inventory/ammo arrays | the per-player runtime block, the player object, the aim flag, the per-player settings, the ammo and weapon-entry arrays with their indexing |
| [weapon-data.md](weapon-data.md) | weapon_data (0x10C-byte records) | the weapon definition table: every field, the animation scripts, the flags, the Player_Init overrides, and the weapon id table |
| [player-port-check.md](player-port-check.md) | — | every `+0xNNN` in the Rust port (`gun.rs`, `pickup.rs`, `player.rs`, `scene.rs`, `weapon.rs`) whose meaning or offset disagrees with the atlas |

## How to use the atlas

- **When porting**: read the struct's atlas page before writing the Rust. If the Rust already reads a field, check
  the port-check page for disagreements.
- **When auditing**: an auditor gets the atlas entry + the decompile and reports which claims it could not
  reproduce. Fix or retag before merging.
- **When a field is `[S]`**: it is a hypothesis, not a fact. Verify it (read the decompile / the snapshot) and
  retag before relying on it.
