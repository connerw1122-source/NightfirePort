# mp-bots-data (401) — the MP-bots data layer: records, BOT_vars, the 55 states

**Status: done.** All three task items are lifted and pinned by tests: the per-bot config
records and the 29 `default_bot_stats` presets are byte-exact data tables with golden tests;
`BOT_vars_t` is mapped field by field in `docs/fields/bot-vars.md`; the 55 bot states
(0xC3..0xF9) are inventoried with their addresses and a phase-4 classification, and the
`BotGlobal` 0x16E060 message-dispatch skeleton is ported behind the `mp-bots` feature with a
routing unit test. Nothing in the sim calls any of it yet — this is the phase-4 list of arms,
not the arms themselves.

## Files

- `crates/nf-game/src/mp/bots.rs` (new, 727 lines): the four data tables + the dispatch
  skeleton + 4 tests. Feature `mp-bots = []` in `crates/nf-game/Cargo.toml` (+3 lines, additive)
  gates the dispatch; `pub mod mp;` in `crates/nf-game/src/lib.rs` (1 line, additive) mounts the
  module — those two lines are the only shared-file touches.
- `docs/fields/bot-vars.md` (new): the `BOT_vars_t` field map, in the `docs/fields/` style.
- `docs/atlas/multiplayer.md`: the §4 status block + two corrections (below).
- `docs/WORKLOG.md`: the `mp-bots-data` entry (append-only).

## 1. The per-bot config records (0x12 bytes × 4 @ 0x2DEEAA)

`BotRecord` wraps the 18-byte record; accessors for every field. Layout evidence:

- [M] `MP_Start` 0x183a50:151 passes `0x2deeaa + i*0x12` to `BOT_init` as the MPBOTS* record and
  reads the team byte `0x2deeb9 + i*0x12` (= record+0x0F) into `MPSettings+slot*0x30+0x20`;
  `BOT_respawn` 0x124e38:74 rebuilds the same pointer on respawn.
- [M] `BOT_init` 0x1249e8 reads the 0xE stats bytes 0x00..0x0D (the copy into `BOT_vars+0xA0`
  and `Drone+0xB4..`), tests `stats & 0x10` for HP regen, and takes the character index from
  record+0x10.
- [M] `Menu_PrepareBots` 0x1f94a0 compacts enabled records (record+0x0E) to the front, writes
  the count byte 0x2DEEA9 and `MPSettings+0x432`, and forces count 0 for map 0x700004b.

Fields: aggression 0x00, aim 0x02 (u8s 0..9, with 0x01/0x03 padding), max HP 0x04 (u16),
speed 0x06, recovery A/B 0x07/0x08, stats[9] 0x09, preferred weapon class 0x0A, **trait** 0x0B
(0..8), flags 0x0C (bit3 0x08 = 360° vision, bit4 0x10 = HP regen), trailing 0x0D, enabled
0x0E, **team** 0x0F, **character** 0x10, dirty 0x11. The boot image [L ee_snapshot @0x2DEEAA]:
slot 4 = preset 0 stats, disabled, team 0, character 6; slots 5..7 empty with characters 7/8/9;
`dirty` 0 everywhere — `golden_initial_bot_records` pins all of it.

## 2. `BOT_vars_t` (0x780 × 4 @ 0x26D660) — `docs/fields/bot-vars.md`

Mapped field by field with per-row proof tags; method was a mechanical census (19 files touch
the `BOT_vars` global; 81 files deref `Drone+0xD1C`, 59 carry BOT_vars-range offsets through
it), then every candidate row verified by reading the citing function. Coverage: the stats
block (+0xA0..0xAD, the 0xE-byte record copy), both 0x50-byte goal slots (+0x000..0x9F, 17
goal-relative fields), and 36 instance fields ≥ +0xB0 ending at +0x77F. Highlights:

- **+0xB0 per-player info block** (8 × 0x10, filled by `BOT_setOtherPlayerInfo` 0x125390):
  +0x04 f32 sq-dist, +0x0C flags — bit1 = candidate, bit3 = same team. `getPreferredTraitOpponentObjIndex`
  0x128b50 selects on it: trait 2 wants bit3 **set** (same-team pool), traits 4/8 want it
  **clear** — so the old [S] trait-name reading (trait 2 = guardian-like shadowing) needs a live
  check to settle (queued below).
- **+0x6DC opponent-history ring** (u32[16], `BOT_handleOpponentHistory` 0x1257c0): the current
  target counts ×5, winner = max count.
- **+0x768 current weapon id**, **+0x769 armour pool** (`BOT_handlePain` 0x125ed8 subtracts
  absorbed damage; `Pickup_Handler` adds — and `BOTWEAP_setDroneArmour` 0x12af70 is **empty**,
  confirming the atlas's suspicion), **+0x765 active goal index** (0xFF none),
  **+0x771 attack re-entry latch** (`BotIdle` 0x171b80 sets/tests; `BotGlobal` 0x45 →
  SetState(0xCF) when set).
- The **+0xAC alias trap** is called out: `Drone_tag+0xAC` is the health f32 while
  `BOT_vars+0xAC` is the stats-flags byte — same offset, different structs.
- Two atlas corrections, applied to `docs/atlas/multiplayer.md`: the per-bot pickup-visit
  floats are at **MP_PICKUP+0x80**, not +0x70 [M `BOTSTATE_setPickupVisitTime` 0x129958:24];
  `processGoals`' melee gate target is state 0xF7 `BotGuardFriendIdle` (the guard/hunt arm).

## 3. The 55 states (0xC3..0xF9) and the `BotGlobal` dispatch skeleton

`BOT_STATES` in `bots.rs`: every id with its function address, name, and a phase-4
classification (`Full` real logic / `Thin` a few helper calls / `Stub` ENTER→one SetState /
`Empty` / `Global`). Addresses are from the game's `NDrone2_StateFuncs` @0x29B300 [L
ee_snapshot, byte-identical in all four]; the id→type bytes are the 0x37-byte table @0x26F460
(one past `BOT_vars` — i.e. `BOT_vars[state+0x1D3D]` in `BOTSTATE_getStateType` 0x1291f0's
framing) [L, reader M]. Types group the family: 1 init, 2 global, 3 trait, 4 attack, 12 cover,
5 stuck/alert/goto-goal, 6 seen/heard, 7 impact, 10 death, 8 door, 13 guard-friend, 9 goal-idle.

Shape of the inventory (the phase-4 workload): 9 trait states (0xC6..0xCE) are all **pure
stubs** — their ENTER just switches to 0xF9 and the real trait behavior lives in
`BOTSTATE_getPreferredTraitOpponentObjIndex`; 17 attack states (0xCF..0xDF) carry the movement/
aim/fire logic (`Full`: 0xCF/0xD0/0xD2/0xD4/0xD5/0xD6/0xD7/0xD8/0xDD); the 9 cover states
(0xE0..0xE8) are all `Full`; death/impact/seen/heard are stubs except `BotImpactBullet` 0x1713F0
and `BotDead` 0x171518; the goal hub `BotIdle` 0x171B80, `BotGotoGoalPosition` 0x171240 and
`BotAlertToPosition` 0x171048 are the goal-machine `Full` states. **`BotGlobal` is itself state
0xC5** in the function table — the atlas now says so.

The dispatch skeleton `bot_global_dispatch` mirrors `NDrone2_DSTATE_BotGlobal` 0x16E060's
switch on the msg id [M 0x16E060:37-759]: the ignore block (0x00, 0x2F..0x39, 0x3E, 0x3F),
killed (0x03: SMI-state 0 → 0xF9, death-type states stay, else 0xF2), the alert family
(0x11..0x16, 0x1E → react unless the current state's type ∈ {0,1,2,4,10}), 0x18 → 0xF5 stun
unless death-type, state change (0x2E: cover disown / +0x758 clear / +0x766 := state_type),
seek objective (0x3A: no opponent → weight +1.0, pickGoal → 0xEB), the goal messages (0x3B/3C/
3D), demo/protect site re-target (0x40/41), weapon announce (0x42 → +0x76A), kill (0x43:
opponent cleared, grudge cleared, type-13 → 0xF9), hit (0x44 → combatWeaponChangeChoice),
weapon-change-done (0x45: +0x771 set → 0xCF). Each arm returns an outcome descriptor — phase 4
fills in the helper calls.

## Verification

- `nf-game` lib: the 4 new tests — `golden_initial_bot_records`,
  `golden_default_bot_stats_table` (29 rows, spot rows + structural invariants + the
  triplicated row-0 preset), `state_table_and_state_types` (55 ids, spot addresses vs the
  snapshot pointer table, the type table's spot rows, out-of-range ids read 0),
  `bot_global_dispatch_routing` (each arm's outcome vs the decompile's switch) — all pass on
  the final merged tree.
- The full suite (fast runner, merged with origin/master): see the total line below.
- No sim behavior changed — `drone.rs`/`scene.rs`/`player.rs` untouched, the dispatch is
  `#[cfg(any(test, feature = "mp-bots"))]` and nothing calls it.

**Suite total (final tree, origin/master 7ac56424 merge):** PENDING

## What's left for phase 4 (the arms list)

1. The attack family first (0xCF..0xD8): `Full` states sharing `Drone.rs`'s existing
   aim/fire/movement helpers; the trait stubs need only the 0xC6..0xCE → 0xF9 enters plus the
   opponent selector.
2. The goal machine (0xF9 hub + 0xEB/0xF7/0xEA/0xE9): `processGoals` 0x127c78, `pickGoal`
   0x126b88, `gotoGoal` 0x1278a8 — all decompiled and mapped, waiting to be ported against the
   field map.
3. The cover family (0xE0..0xE8) — the biggest self-contained block after attack.
4. The `BotGlobal` arms beyond the skeleton: each outcome descriptor's helper call
   (`ReactToDroneAlertMsg`, `combatWeaponChangeChoice`, `setGoalComplete`, …).
5. The spawn path: `BOT_init` 0x1249e8 / `BOT_respawn` 0x124e38 against `Drone_Create`.

## Live checks for the main agent

1. **Goal timeout constant**: `gdbwatch` `0x26D660+0x28` (w) in an MP match with 1 bot, after
   `BOTSTATE_initGoal` writes — expect 5.0 (atlas §4; the write site wasn't in the decompile).
2. **+0x767 self-index writer**: `gdbwatch` `0x26D660+0x767` (w) — expect a write during
   `BOT_init`/menu prepare with the bot's own player index; the decompile only shows the reader.
3. **Trait 2 = same-team pool?**: `fieldprobe` poke `BOT_vars+0xAB := 2` on a bot in a *team*
   deathmatch, screenshot — expect shadowing a teammate rather than hunting; in FFA expect no
   change (bit3 never sets in FFA per `setOtherPlayerInfo`). Settles the trait-name table.
4. **+0x769 armour absorb**: `fieldprobe` poke `BOT_vars+0x769 := 100`, then shoot the bot —
   expect the HP bar to drop slower and +0x769 to decrease by the absorbed amount.
5. **Pickup visit deadline**: `gdbwatch` `MP_PICKUP+0x80 + botidx*4` (w) when a bot grabs a
   pickup — expect `now + 45.0` [M 0x129958:24].
6. nf-view has no bot render yet — no `--bond` spots apply; once phase 4 lands the attack
   family, spot a bot mid-strafe (`BotAttackStrafeAimLeft` 0x16F850) for the strafe cadence.

## ntfy failed

(none — notification sent at finish)
