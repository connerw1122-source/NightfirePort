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
| `player.md` | BLData, the player object, M_CONTROL, PlayerSetting, ammo/inventory arrays |
| `weapon-data.md` | weapon_data (0x10C-byte records) |
| `player-port-check.md` | every disagreeing `+0xNNN` in `gun.rs`/`pickup.rs`/`player.rs`/`scene.rs`/`weapon.rs` |
| `trigger.md` | the trigger (`control_create_object` ext, 0x40 bytes, class 0x1D) — struct 5 |
| `door.md` | the door (DOORINFO ext, 0x80 bytes, class 0x1C) and the auto-close question — struct 6 |
| `channels.md` | the switch channels (five global arrays + the object-side channel fields) — struct 9 |
| `trigger-port-check.md` | every disagreement of `crates/nf-game/src/{triggers,door}.rs` with structs 5/6/9 |

Planned next (plan v2 §2 order): pickup, script player, searchlight, wire/grapple, cel/room,
GS leaf state.

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
| [trigger.md](trigger.md) | the trigger ext (class 0x1D) | field table, the per-type create wrappers and their param maps, Trigger_Update's cases with fire conditions, Trigger_Activate per kind, Trigger_Calc/Hit, snapshot-checked 07000005 instances |
| [door.md](door.md) | the door ext (class 0x1C) | field table, the obj+0xF4 state register, the ext+0x70 flag bits, the full auto-close predicate with addresses, Door_Activate/SetState/Interp/ReactToHit, group sync, snapshot + disc checks for 07000001 |
| [channels.md](channels.md) | switch channels (struct 9) | the five arrays, Init, frame order, every writer, every reader, and the complete 07000005 channel table with setter/reader placements |
| [trigger-port-check.md](trigger-port-check.md) | — | every disagreement of `triggers.rs` and `door.rs` with structs 5/6/9 |

## How to use the atlas

- **When porting**: read the struct's atlas page before writing the Rust. If the Rust already reads a field, check
  the port-check page for disagreements.
- **When auditing**: an auditor gets the atlas entry + the decompile and reports which claims it could not
  reproduce. Fix or retag before merging.
- **When a field is `[S]`**: it is a hypothesis, not a fact. Verify it (read the decompile / the snapshot) and
  retag before relying on it.
