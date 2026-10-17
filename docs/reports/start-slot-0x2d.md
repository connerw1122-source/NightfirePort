# start-slot-0x2d — the second start-position set (0x2D)

Branch `s/start-slot-0x2d` (base: origin/master 50d51ae7, no drift). Task 751.

## What was done

The game registers TWO player-start sets from the main map's placements — type 0x24 as kind 1 and
type 0x2D as kind 0 — and `Player_Start` picks **kind-1 only**. The port registered only the 0x24
set (the census gap, task 709's #9). Ported:

- `crates/nf-game/src/player.rs`:
  - `LevelStart.kind` — the u16 `Player_AddNewStartPos` 0x195c00 stores at entry+0x22 [M 0x195c00:33]:
    type 0x24 passes 1 (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x24, disasm 0x1d0a00
    `addiu a2,$zero,1`), type 0x2D passes 0 (case 0x2d, disasm 0x1d0e98 `move a2,$zero`).
  - `starts_from_placements` — the shared registration: main-map placements in parse order,
    `flags & 0xa000 == 0` eligibility [M 0x1d1b68:93], gate = param 0, p3 = param 3 (the parse
    `level_starts` already pinned, unchanged). `nf-view`'s `level_starts` (now `pub`) delegates to
    it, so the load path and the tests run the same code.
  - `pick_start` — the game's two passes, both restricted to kind-1 [M 0x190fd0 disasm 0x191028 /
    0x1910b0 `bne v0,1` on entry+0x22]: first kind-1 with gate 0 or channel on, else first kind-1
    (the game zeroes its gate; the port runs one pick per load).
- `crates/nf-formats/tests/script_census.rs`: handled sets 61 → 62 (0x2D moved to handled); the
  unhandled list 330 → 279 records (141 game-handled + 138 neither side touches).
- Docs: `docs/systems/player-starts.md` (new, the full rule with addresses), the census tables +
  task 9 struck PORTED with the correction, `docs/levels/07000007.md` §1 lists the five 0x2D
  records, WORKLOG entry.

## The census's spawn worry is REFUTED, not fixed

The census speculated Bond "may spawn at the wrong start" on levels shipping both sets. The
decompile says otherwise: **no code reads the kind-0 records** — `player_start` has exactly three
references in the decompile (the writer 0x195c00, the reset 0x195bf0, and the reader 0x190fd0 whose
kind check the 0x2D records fail), and MP matches spawn humans from `MP_Start` 0x183a50's
`MP_GetSpawnPoint`, not `player_start` [M :118-128]. So the port's spawn behaviour was already
game-accurate; what was missing was the registration itself (the 0x2D placements were silently
dropped). The fix registers both sets with the game's kinds and makes the pick kind-aware, which
keeps every pinned spawn where it was (the two load-path sisters passed before, during and after
the fail-first probe).

## Disc facts pinned [D, through the port's parsers]

- Main maps carry 38 kind-1 and 18 kind-0 starts; the census's 51 type-0x2D records = those 18 plus
  one (0,0,0) shell-map dummy per archive (33) that the main-map parse never reads [M
  `ResetMap_Load` 0x1d6360:150 parses the loaded map's dynamic block].
- 07000007: two 0x24 (33.40,44.50,−70.63 and 115.56,44.50,−88.17, both gate 0) with four main-map
  0x2D hall starts interleaved in parse order.
- 0700000b: a real 0x2D (9.92,48.53,21.48) comes FIRST, before the level's only 0x24
  (28.92,13.28,28.67) — the load-path sister level.
- 0700000c/0700000d: a 0x2D twin ~0.03 from the 0x24.
- Every 0x24/0x2D placement on every main map resolves to an object [D sweep] — the game's
  failed-lookup gate (`beqz a3` 0x1d1c90) never bites, so the port's data-level registration
  matches the game's effective behaviour.

## How it was verified

- **Fail-first (reproduce before fix)**: with the 0x2D registration arm disabled — the exact
  pre-task behaviour — `registration_maps_0x24_to_kind_1_and_0x2d_to_kind_0` fails
  (`left: 2, right: 3`) and `disc_sweep_registers_both_sets_with_the_game_kinds` fails on
  07000007 (`left: (2, 0), right: (2, 4)`); restored, both pass. Failure output captured in this
  session, 05:10-05:12 UTC.
- `crates/nf-game/tests/start_slot_0x2d.rs` (5 headless tests, no ISO): the registration mapping
  (kinds, 0xa000 eligibility, order, gate/p3/pos/yaw) and the pick sisters — a leading kind-0
  entry loses, the gate pass, the kind-1-only fallback, only-kind-0 → None.
- `crates/nf-view/tests/start_slot_0x2d_live.rs` (3 ISO-gated tests): the disc sweep (totals +
  the 07000007/0b/0c layouts through `nf_view::level_starts`) and the real load path — 0700000b
  spawns at the kind-1 start (28.92,13.28,28.67) despite the leading 0x2D, 07000007 cold-loads at
  the party plate #0 (33.40,44.50,−70.63) with the second set registered.
- `crates/nf-formats/tests/script_census.rs` (6 tests): all green with the moved constant.
- Disassembly: `Player_Start` 0x190fd0 read end-to-end (0x190fd0–0x1910f4) — two passes, both
  `lhu +0x22` vs 1, no third pass; the parsemap arms read at the call sites (0x1d0a00/0x1d0e98).
- **Suite**: the fast runner (`tools/test/suite.py`) on the final tree (= origin/master + this
  branch, no drift): **2057 passed, 6 failed, 74 ignored (48 min)**. The 6 failures, none from
  this change: the 3 `disc_golden` tests (the known slim-ISO allowance) and 3 no-GPU render tests
  — `fix_0c_pylon_live` × 2 and `probe_pylon_render` × 1 — which panic on
  `request_adapter(..).expect("adapter")` in this GPU-less sandbox; the identical panics are in
  the earlier sessions' suite logs (e.g. /home/z/suite.log 2026-10-13, same expect at the same
  lines), so they pre-date this branch. Everything that touches the changed code is green:
  start_slot_0x2d (5), start_slot_0x2d_live (3), script_census (6), plus nf-game/nf-view in full.

## What's left

- The two live PCSX2 checks below (this sandbox cannot run the game). Everything else is pinned.
- The kind-0 records' purpose stays unknown — the game writes them and never reads them. [S]
  suggested: they may be leftover tooling data (e.g. designer-placed camera/escort anchors) or
  consumed by a path this port does not model; if a future live probe finds a reader, the kind
  model is already in place.

## Live checks for the main agent

1. **07000007 cold-boot spawn** (no measurement in docs/live): PCSX2, FRESH boot (a save-state
   resume skips the boot path), load the party level, read Bond's position (the trans probe
   fieldprobe uses). Expect ≈ (33.4, 44.5, −70.6), yaw ≈ 1.57 — the kind-1 plate #0 [D; M
   0x190fd0]. If the game stands Bond at (52.1, 44.5, −50.7) or another 0x2D spot, the kind
   semantics are wrong and the pick must flip.
2. **0700000b cold-boot spawn**: expect ≈ (28.9, 13.3, 28.7) — NOT (9.9, 48.5, 21.5), the 0x2D
   that precedes the 0x24 in parse order [D].

## Usage

Not visible to me — this session's environment shows no token, context-window or cost figures
(the IM gateway metadata carries only session/chat/trace ids; no usage counters are exposed to
the assistant).
