# cutscene-guard-models — report

Task 750q: the user's "cutscenes not matching guard models" (screenshot nf-shots/002-07000001.png — an
07000001 cut-scene with guards). Branch `s/cutscene-guard-models` (from origin/master 50d51ae7).

## What I did

1. **Decompiled the whole actor-skin path** and wrote it up with addresses and evidence tags in
   `docs/systems/cutscene-actor-skins.md`:
   - `Script_AnimStart` 0x1d8f40 → `AnimObjectNew` 0x1be910: the op-0x0A skin hash passes two gates — the
     per-level substitution (0x05000090 → 0x05000087 on 0x0700000C, → 0x05000093 on 0x07000014 [M disasm
     0x1be958-0x1be9ac, `movz`; the level id is `GameState+0xC` @0x2a3774 — `lw $v0, 0xc($s7)` at
     0x12c070 settles the decompiler's misleading "+0x12"]), then the loaded-file hashtable
     (`hashtable_getitem` 0x1ca5b0, filled by `LoaderProcess` 0x1d4880 case 3/4/5 → `AnimLoadFile`
     0x1bdf40; nothing frees between levels).
   - `AnimObjectNew` writes the REQUESTED hash into the skin record's first word [M 0x1becd8] — which
     makes the fresh-boot EE snapshots a direct record of what the game asked for. [L
     ee_start_07000001.bin]: skin records 0x05000009/17/2b/36/38/39/3a/4f carry their own hash; the other
     20 still hold the 0x98989898 malloc fill.
   - The **captain** substitution: `NDrone2_DefaultInit` 0x14b300:528-548 rewrites a drone's skin by kind
     (1 → 0x05000038, 2 → 0x0500002f, 3 → 0x05000006, 6 → 0x05000093, 7 → 0x050000b2) when the
     difficulty-gated captain flag is set. The port already implements this (`captain_setup`, drone.rs)
     and my live check confirms it produces the game's exact set on 07000001 (15 present drones: 36×6,
     39×4, 3a×3, 4f×1 + ONE 0x05000038 captain — a skin NO placement names).
   - The cut-scene drone gate: `Script_EventHandler` case 5 → `Drone_EnableAll(0)` 0x1383e8 →
     `NDrone2_Enable(0)` 0x149dd0 sets `obj+0xF0 |= 0x10` ("not drawn", docs/fields/obj-tag.md) — the
     placed guards vanish during a cut-scene. Already ported (`Request::DroneEnable` → `npc_enabled` +
     per-drone `enabled`; the older WORKLOG note calling event 5 unported is stale).
2. **Built the per-actor table** (in the doc, §"Per-level tables"): for 07000001, 07000005, 07000006 and
   the substitution levels, each cut-scene actor's stream ↔ anim-script ↔ skin, the port's resolution and
   the game's (snapshot-proven where the scene actually plays).
3. **Tests** — `crates/nf-view/tests/cutscene_guard_models_live.rs` (NF_ISO-gated, 3 tests, all pass):
   the intro actors' skins AND stream pairings (the script's exact 0x0600055e/2b, 0600055f/17,
   0600055d/4f, 0600055c/09), the drawable characters per skin, the 15-present-drones + captain-0x05000038
   state, and the guards' cut-scene SP waiting in state 0 exactly like the game's.
4. **Reproduce-first**: my first version of the third test drove the user-visible path (fire the SP's
   channel, tick, assert the actors) — it FAILED on master: the port never starts 0x06000086. Chasing
   that produced the real finding below, and the test was narrowed to the [L]-proven waiting state (the
   game itself never starts the scene, so "it starts" would pin unproven behaviour).

## The findings

- **On 07000001 the port's cut-scene actor skins are CORRECT** — bit-for-bit the game's, proven by the
  game's own snapshot (the four intro skins match; the drawable characters build).
- **The guards' cut-scene (0x06000086) is unreachable content in the shipped game**: no placement on any
  of the 54 maps and no script event writes `switch_channels[0x33]` (its only trigger), and its SP is
  still state 0 in the boot snapshot [L 0x018fbfc0]. Its three actors (0x0500003a w6, 0x05000038 w0x1c,
  0x05000036 w0x1c) can never appear.
- **The 07000001 screenshot's "guards" in the cut-scene are most plausibly the placed drones** (skins
  36/39/3a/38/4f), which the game HIDES during cut-scenes (event 5) and the port now hides too (already
  ported; verified the chain end-to-end). If the render-check still shows guards inside the intro, the
  spot to look at is the view's `enabled` gate on the drone characters (scene.rs character build).
- **Open port bug found on the way (not fixed — out of budget)**: a hand-written switch-channel value is
  cleared by some port system within one tick (`Channels::set(0x33, 1)` then one `step_bond` reads it
  back 0), where the game's `switch_channels` persist until written. No shipped 07000001 content reads
  ch 0x33, so it is invisible there; a census on other levels is the follow-up (the 0700004a lift's
  ch-0x22 stand-in at nis.rs:1300 is the same shape).
- **0x05000090 substitution on cut-scene actors**: no cut-scene script on 0700000c/07000014 carries op-0x0A
  entries [D dumps of all seven SP scripts] — the substitution applies to placed drones only, which the
  existing G-021 load-time merge already covers.

## What I ran

- `cargo test --offline --release -p nf-view --test cutscene_guard_models_live` — 3/3 pass (twice, before
  and after the final edit).
- `python3 tools/test/suite.py` (fast runner, background): build ok (489 binaries in 511 s); the run was
  still executing when the deadline hit (nf-game's binaries take ~30 min each). Lines so far: 4 FAILED —
  the 3 allowed slim-ISO `disc_golden` failures (bigf header/tables, directory tree) plus
  `probe_pylon_render::probe_render_pylon_only`, a pre-existing master probe test (present on
  origin/master, commit 0146700c lineage) that fails there too, untouched by this branch. My branch's
  changes (one new test file, two new docs) were exercised by the foreground run above; nf-game/nf-view
  compile clean.
- Tools used for evidence: a Python reader of the disc (level archives, placements, NIS scripts) checked
  against `nf-extract bin-list` (616/703 entries, 0 mismatches) and `golden/levels.tsv` (331/363
  placements, matching); an EE-snapshot reader for the hashtable, skin records, SP instances and drone
  skins.

## What's left (for the next message, if any)

1. The switch-channel persistence bug: find the port system that clears a foreign channel write in one
   tick (suspects: a placement type whose port arm writes 0 unconditionally — sensors 0x28 / world_fx /
   the SP out-writeback are the shapes), fix it, and pin it with a sister assertion per level.
2. The main agent's render-check of the intro cut-scene (below) — if guards still show, trace the view's
   `enabled` gate.

## Live checks for the main agent

- **Intro actors** [L-prepared]: `--bond -2.78,6.84,-12.4` (07000001 start), let the intro play ~2 s.
  Expected: exactly four script characters — skins 0x0500002b, 0x05000017, 0x0500004f, 0x05000009 (Bond);
  NO placed guards visible (event 5 hides them; the port's `enabled` gate should already do this).
  PCSX2 cross-check: the same spot in the real game.
- **The captain** [L-backed]: 07000001 gameplay, the courtyard guard group — one guard wears skin
  0x05000038 (the captain, the same model the cut-scene 0x06000086 would have mirrored). Compare against
  PCSX2 at the same spot.
- **guards_cutscene_actors** [S — unreachable]: firing ch 0x33 headless should create actors 0x0500003a/
  0x05000038/0x05000036 with weapons 6/0x1c/0x1c on streams 1/3/12. It does NOT in the port (channel
  cleared in-tick) — do not spend a render on it until the channel bug is fixed.

## Usage

not visible to me — this sandbox exposes no token, context-window or cost figures.
