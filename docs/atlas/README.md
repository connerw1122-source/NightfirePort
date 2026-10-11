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
