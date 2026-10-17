# Player starts — the registration and `Player_Start`'s pick

*Task start-slot-0x2d, 2026-10-17. Evidence tags: [M] decompile/disassembly (ACTION.ELF), [D] disc
bytes read through the port's parsers, [S] suggested.*

## The two sets and their kinds

Every level archive (and every MP map's shell) ships two start-position placement types
(`docs/systems/script-census.md`): type **0x24** and type **0x2D**. The dynamic-object parser
registers both into the same `player_start` array through one function:

- `Player_AddNewStartPos(pos, rot, kind, level_tag)` 0x195c00 appends a 0xE0-byte record:
  pos at +0x00, rot at +0x10, **the u16 `kind` argument at +0x22** [M 0x195c00:33], and a 176-byte
  copy of the level_tag at +0x24 (the gate switch channel the port uses sits at record+0x50 =
  tag+0x2C [M 0x190fd0 reads +0x50]).
- `parsemap_create_dynamic_objects` 0x1d04f0 dispatches on the placement's type byte:
  case **0x24 passes kind 1** [M :91-98, disasm 0x1d0a00 `addiu a2,$zero,1`], case **0x2D passes
  kind 0** [M :154-161, disasm 0x1d0e98 `move a2,$zero`]. Both arms sit behind the same gates:
  `n != 0` (the main-map parse flag — `ResetMap_Load` 0x1d6360:150 calls
  `parsemap_block_map_data_dynamic(.., 1)`) and `MPSettings._384_4_ == 0` [M disasm
  0x1d09e0-0x1d09e8 / 0x1d0e78-0x1d0e80]. The dispatch itself only runs for placements with
  `(flags & 0xa000) == 0` [M 0x1d1b68:93], and the caller skips a placement whose object lookup
  failed before the call [M `beqz a3` 0x1d1c90] — on this disc every 0x24/0x2D placement on a main
  map resolves [D, start_slot_0x2d_live's sweep], so that last gate never bites here.
- `Player_ResetStartPos` 0x195bf0 zeroes the index; `player_start` has no other readers [M — the
  only decompile references are 0x195bf0/0x195c00/0x190fd0].

## The pick

`Player_Start` 0x190fd0 (called from `ResetMap_Load` 0x1d6360:154 after the map load [M]) makes
two passes over the records in registration order (= placement order, 0x195c00 appends):

1. the first record with **kind == 1** whose gate is 0 or whose switch channel is on
   [M disasm 0x191014 (`t0 = 1`), 0x191024-0x191040: `lhu +0x22` compared to 1, then the +0x50
   channel checked against `switch_channels`];
2. fallback: the first record with kind == 1, zeroing its gate channel [M disasm 0x191098, 0x1910b0,
   0x1910c4 `sw zero, 0x50`].

**Kind-0 records never win either pass** [M both loops `bne v0, 1`]. The 0x2D set is written and
then read by nothing in the on-foot runtime — the task-709 census speculation that Bond "may spawn
at the wrong start" on levels shipping both sets is refuted by the kind check: the port's 0x24-only
pick already matched the game's outcome; what the port lacked was the second set's registration
(the census gap). MP matches do not use `player_start` at all — `MP_Start` 0x183a50 spawns humans
from `MP_GetSpawnPoint` [M :118-128], and in MP mode the parser registers no starts (the
`MPSettings` gate).

With no kind-1 record, `Player_Start` returns without calling `Player_Init` [M the function ends
at 0x1910f4 after the second loop]. The port's `pick_start` returns None and the load keeps the
"Player1" stand-in / runs the MP core — same outcome on the disc's data, where every on-foot main
map has at least one kind-1 start [D].

## Disc layout [D, read through the port's parsers]

- Every archive's kind-0 shell map carries a dummy pair, 0x24 (0,0,0) and 0x2D (0,0,0) — the
  main-map parse never reads them (above), and the port's `level_starts` reads the main map only
  (the pinned convention behind the spawn playtests).
- Main maps: **38 kind-1** starts and **18 kind-0** starts across the disc's 07* archives
  (the census's 51 = 18 main-map + 33 shell-map dummies).
- 07000007: two 0x24 (33.40,44.50,−70.63 gate 0; 115.56,44.50,−88.17 gate 0) then four 0x2D in the
  hall (52.12,44.50,−50.67 / 91.59,44.25,−27.98 / 83.28,44.50,−53.29 / 95.49,50.50,−85.24), then
  the second 0x24 — parse order interleaves the sets.
- 0700000b: a real 0x2D (9.92,48.53,21.48) comes FIRST, before the level's only 0x24
  (28.92,13.28,28.67) — the load-path sister level: a pick that ignored kinds would stand Bond at
  the 0x2D.
- 0700000c/0700000d: a 0x2D twin sits ~0.03 from the level's 0x24 (8.96,15.07,−149.62 vs
  8.98,15.07,−149.65) — the pick must still land on the 0x24.

## The port

- `nf_game/src/player.rs`: `LevelStart.kind` (the +0x22 u16), `starts_from_placements` (the
  registration mapping above), `pick_start` (the two kind-1 passes).
- `nf-view/src/lib.rs`: `level_starts` delegates to `starts_from_placements` over the main map's
  placements; the load path picks and calls `Bond::stand_at` (the `Player_Init` stand-in).

## Tests

- `crates/nf-game/tests/start_slot_0x2d.rs` (headless): the registration mapping (0x24→1, 0x2d→0,
  0xa000-eligibility, order, gate/p3), the pick sisters — kind-0 first loses, gate pass 1, the
  kind-1-only fallback, only-kind-0 → None.
- `crates/nf-view/tests/start_slot_0x2d_live.rs` (ISO-gated): the disc sweep totals + the
  07000007/0b/0c layouts + the real-path loads (0700000b spawns at the kind-1 start despite the
  leading 0x2D; 07000007 spawns at the party plate #0 with the second set registered).
- Fail-first proof (2026-10-17): with the 0x2D arm disabled (the pre-task 0x24-only behaviour),
  `registration_maps_0x24_to_kind_1_and_0x2d_to_kind_0` fails (2 registered vs 3) and
  `disc_sweep_registers_both_sets_with_the_game_kinds` fails on 07000007 ((2,0) vs (2,4)); the two
  load-path sisters keep passing (the spawn outcome was already kind-1-correct). Restored → 8/8.

## Live checks for the main agent

1. **07000007 cold-boot spawn** (no measurement exists in docs/live): PCSX2, FRESH boot
   (a save-state resume skips the boot path), select the party level and read Bond's BLData
   position (the trans at BLData+0x0-ish — the same address fieldprobe reads for Bond). Expect
   ≈ (33.4, 44.5, −70.6) facing yaw ≈ 1.57 — the kind-1 plate #0 [D placements; M 0x190fd0]. If
   the game instead stands Bond at (52.1, 44.5, −50.7) or any of the four 0x2D spots, the kind
   semantics above are wrong and this task's pick must flip.
2. **0700000b cold-boot spawn**: expect ≈ (28.9, 13.3, 28.7) — NOT (9.9, 48.5, 21.5), the 0x2D
   that precedes it in parse order [D placements]. Same decision rule.
