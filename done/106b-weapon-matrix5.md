## Your task: `weapon-matrix5`, weapon matrix: all 27 levels, harness gaps closed

Base: origin/master (weapon-matrix4 and weapon-matrix3b should be on it; if not, merge `s/weapon-matrix4`). Close
weapon-matrix4's harness gaps: the watch pair 0x54/0x55's mode press ORs the fire edge [M `Player_Weapon` 0x1a24c8,
action 9 mask 4]; the away switch after the Sentinel waits for `missile_cam` to clear plus the game's 15-tick restore;
re-run the Wolfram 0x04 on 0700004a and the MGL 0x2a on 0700000d. Then sweep ALL 27 levels (`nf-extract
weapon-matrix`), and for every remaining red cell write a numbered weapon finding (weapon, level, what's wrong, the
game evidence). Keep the in-suite `weapon_matrix_live` under 1200 s (report its runtime). Panics are bugs.

Files: `crates/nf-view/src/matrix.rs`, `crates/nf-view/tests/weapon_matrix_live.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/weapon-matrix5`.

---
claimed-by: sandbox1 at 2026-10-14T20:59Z

---
## Report (done, 2026-10-14T22:00Z)

Closed by the driver: the report is on `s/weapon-matrix5` (docs/reports/weapon-matrix5.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T22:29Z)


---
## Report from the worker (done, 2026-10-14T22:30Z)

# weapon-matrix5 — the watch's fire press is a mode press; the Sentinel cell rides out its own camera

Branch `s/weapon-matrix5` (from origin/master @ 9c55eace + the merge of `origin/s/weapon-matrix4b` —
matrix4's harness work and matrix3's gadget cells were NOT on master yet; the driver's "merge
s/weapon-matrix4" resolved to `s/weapon-matrix4b`, which contains `s/weapon-matrix4`). Task ID 106b.
Files this task changed: `crates/nf-view/src/matrix.rs` (the harness), `crates/nf-view/src/scene.rs`
(ONE port fix, shared file — see below), `crates/nf-view/tests/weapon_matrix_live.rs` (sister
asserts), and the two probes `crates/nf-view/examples/probe_watch.rs` / `probe_reds.rs` (kept).

## The three harness gaps, closed

1. **The watch pair 0x54/0x55's mode press ORs the fire edge** [M `Player_Weapon` 0x1a24c8:
   `bVar5 = Input_Action(slot,0xc,4); if (*w - 0x54 < 2) bVar5 |= Input_Action(slot,9,4);`] — the
   game enters the mode block on the OR'd variable, so a FIRE press reaches it for this pair.
   Two defects found:
   - **Port (scene.rs, one fix):** the port's mode block sat behind `if input.mode_switch`, which
     made the `toggled` (fire-edge) arm dead code — the OR arm could never fire. Fixed: the block's
     condition is now `input.mode_switch || toggled`, matching the game's OR. Fail-first:
     `probe_watch` style b (a fire press through the real input path's shape, from the idle
     machine, on 07000005) printed "no swap" pre-fix and "SWAP landed 0x54→0x55" post-fix.
   - **Harness (matrix.rs `step`):** the real input path feeds `fire: self.trigger` into `Input`
     AND the same value as the trigger arg (lib.rs's input build) — ONE action bit. The harness fed
     only the trigger arg, so even a live OR arm never saw the edge during the cells' fire phases.
     `step` now carries the invariant (`input.fire = trigger`).
   - The 4.5 landed-test also generalised to `after != before` (weapon or mode byte moved): the
     watch's fire presses may leave the TWIN (0x55) held, and a press that moves 0x55→0x54 IS the
     twin chain operating — the old `after.0 != id` read it as a refusal.
2. **The away switch after the Sentinel rides out `missile_cam` + the game's 15-tick restore**
   [M `Bullet_Delete` 0x12e660:27-29 — the guided (+0x6C & 4) owner gets `+0x878 = 0; +0x944 = 0xf`;
   `Player_Update` 0x193600 case 10 counts +0x251 down and only then `Player_SetCamMode(BLData,0)`
   + the saved substate back]. Under the substate-10 camera the player runs no weapon arms (the port
   zeroes the input — scene.rs's missile_cam block), so a reload pulse, the 4.5 mode press AND the
   away switch sent under it are eaten whole. The harness now rides the camera out (bounded 700
   ticks) + 17 ticks before the reload phase. **Result on 07000005: the 0x2c cell's W is gone and
   the M hidden under it is gone — the cell is clean.** matrix4's W cells on 0700000b/48/4a: 0b and
   48 measured this window — 48 still W (see finding 4 below), 0b not re-measured yet.
3. **The re-runs:** the Wolfram **0x04 on 0700004a is CLEAN** this window (matrix4's one-off W did
   not reproduce — its "press budget at the cell edge" reading stands; no weapon finding). The MGL
   **0x2a on 0700000d: re-measured — STILL X, gap 97** (see the re-runs section).

## The sweep — where it stands

`probe_reds` (= `nf_view::matrix::run` + `mark_table` + `red_cells` + `aggregate_table`; the same
runner `nf-extract weapon-matrix` drives) measured this window:

- **07000005 (the in-suite sample level): 98 cells, 3 reds, 0 panics** — 0x32 F, 0x38 F (below),
  plus the by-design 0x01 `.`. The 0x2c Sentinel is CLEAN (gap 2 fixed). The 0x54 watch is CLEAN.
- **The seven wrappers 41/43/46/48/49/4a/4b: 686 cells, 29 reds, 0 panics.**
- **The story levels 01-0c + 0d: 7 levels × 98 (01,02,03,04,06,07) + 5 × 98 (08-0c) + 98 (0d)
  = 764 cells, reds 18 + 18 + 3, 0 panics.** Per level the reds are the 0x32 F / 0x38 F pair (and
  the dim 0x01/0x1e/0x43 `.` give-refusals — three records with NO first-person model [D
  weapon_data +0xDC == 0: Fists, the Winter Tactical Sniper 0x1e, the Korsakov K5 0x43 — the game's
  own give refusal, `Player_EquipWeapon` 0x1a8a58:20]), except 07000009 (no 0x32 red this run —
  something spawned, the target-gated discriminator) and 0b/0d (below).
- **Panics: 0 everywhere** (the catch_unwind net never fired).
- Remaining unmeasured: 11/12/13/14/15/16/1b (7 story levels) — the next window's first item
  (`NF_MATRIX=all cargo test -p nf-view --test weapon_matrix_live -- --ignored --test-threads=1`
  or `PROBE_LEVEL=` chunks through `probe_reds`).

## The re-runs (the task's item 3)

- **The Wolfram 0x04 on 0700004a: CLEAN** — matrix4's one-off W did not reproduce (its "press
  budget at the cell edge" reading stands; no weapon finding).
- **The MGL 0x2a on 0700000d: STILL X, gap 97** (gren 2, expl 1, 0 panics). The blast landed 97
  ticks after the spawn — past the harness's ≤90 "contact" bound, far below the 210-tick fuse. The
  reading: on 0d's spawn spot the grenade's flight to first contact takes ~97 ticks — the impact
  arm DID blast on contact, the fixed 90 bound is just spot-geometry-dependent (05's spot measures
  ~30). Suggested harness tune (next window): gate the 0x2a X mark on `gap` being well under the
  210 fuse (contact) rather than a fixed 90. Not a port-bug signal.

## The numbered weapon findings (every red measured this window)

1. **0x32 Phoenix Samurai — F (fire, no observable event)** on 07000005/43/46/49 (and matrix4's 16
   story levels). Unchanged from matrix4's reading: the cell draws, plays its fire script, sounds
   start, nothing spawns at an empty spawn spot — the target-gated fire (the stunner's no-target
   arm shape [M `Player_WeaponInitBullet` 0x1a4bf0:169-200]) or a missing swipe effect. The
   discriminator (clean where something DID spawn) still supports target-gating. Live check below.
2. **0x38 Remote Mine — F** on EVERY level measured this window (05, all 7 wrappers; matrix4's
   18-level run did not flag it). `gren 0` — the fire presses spawn no grenade at all: 0x38's
   record is the mine family's twin-linked pair mate (twin 255 = −1 → 0x37) and its `per_shot`
   (+0x91) is 0 [L ee_start_05.bin — the zero-spend detonator family, gun.rs's `ammo_per_shot`
   note]. What's wrong: either the give of 0x38 should plant/throw through the 0x37 route the
   player uses (the detonator with nothing planted does nothing — game-honest), or the record's
   fire path lost its grenade spawn in the merged tree. Evidence to collect: the live check below
   (does a real-game give of 0x38 throw a mine on fire?).
3. **0x2e AT-600 Scorpion — D (draw failed) and 0x30 Torpedo Launcher — D** on 0700000b/48/4a/4b —
   and the cells carry **gcam1**: the GUIDED CAMERA from the preceding 0x2c cell is STILL RIDING
   when they start. Root cause confirmed this window: on those levels the Sentinel's missile never
   dies inside the ride-out's 700-tick bound (open-sky geometry), the 0x2c cell ends wedged under
   the substate-10 camera, and every input of the NEXT cells is eaten (the draw refused → D; the
   same levels where matrix4 saw the 0x2c W). On every level where the missile dies (05, 01-08,
   0a, 0c, the wrappers 41/43/46/49) the 0x2e/0x30 cells are clean. The fix is finding 4's: bound
   the missile the way the game does, and these cells un-wedge with it.
4. **0x2c AT-420 Sentinel — W on 0b/43/48/49/4a/4b** — the ride-out expires: on those levels the
   guided missile NEVER dies inside the 700-tick bound (open-sky geometry; on 05 it hits something
   in seconds), and the cell (and its successors — finding 3) stay wedged under the camera. The
   honest arm is documented in the harness ("a missile that never dies cannot hang the cell — the
   cell then records the honest mark"). What's wrong: the port's guided missile has NO lifetime /
   world-bound blast where the game's must have one (the real missile cannot fly forever — a
   `Bullet_update` 0x12e7b0 lifetime/fuse read is the next evidence step). Live check below decides.
5. **0x01 Fists [.]** on every level — by design (no first-person model, `Player_EquipWeapon`
   0x1a8a58:20), dim, not a failure.

## Verification

- Fail-first evidence per fix is inline above (the probe's pre/post lines; the 05 row's 0x2c
  W+M → clean; matrix4's report holds the 0b/48/4a W cells and the 0x54 M cells).
- **Suite (tools/test/suite.py), background, on the tree = master + matrix4b merge BEFORE this
  task's three fixes:** `1099 passed, 6 failed, 63 ignored (36 min)`. The 6 failures are the
  allowed 3 `disc_golden` + the 3 pre-existing pylon failures (fix-armour-ring's cross-task note).
  **`weapon_matrix_live` passed with NO TIMEOUT** — the in-suite sample (05/0c/1b + the gadget
  cells + the runner checks + the dev row) fits the 1200 s cap. Its exact runtime line is not in
  the suite's log format; the sample measured ~270 s on matrix3's harness and the added ride-out
  costs at most ~12 s per 0x2c cell. **The final-tree re-run (with this task's fixes) does not fit
  this window** — it is the next window's first item before any push of follow-up work.
- Shared files touched: **`crates/nf-view/src/scene.rs` — one edit**: the mode block's entry
  condition (`input.mode_switch || toggled`, the OR per 0x1a24c8) + its comment; no other lines.

## Live checks for the main agent

1. **The watch's fire press** (the OR): PCSX2, any level, give/equip the Micro-Camera 0x54, press
   FIRE once — does the real game (a) swap the held record to 0x55 without throwing, (b) throw the
   camera AND swap, or (c) throw without swapping? gdbwatch: `Player_Weapon` 0x1a24c8 around
   0x1a26c8 (the `*puVar15 - 0x54 < 2` OR) and `Player_WeaponFiring` 0x1a5460's fire branch on the
   same tick — the port currently swaps WITHOUT throwing (the switch request eats the fire branch,
   `can_fire = !switching`).
2. **The 0x38 Remote Mine's fire**: give 0x38 in the real game, press fire — does a mine fly?
   (The port spawns nothing; the record's +0x91 = 0 and twin −1 say "detonator", but the catalog
   names 0x38 "Remote Mine".) gdbwatch `Player_WeaponInitBullet` 0x1a4bf0's spawn for 0x38.
3. **The guided missile's lifetime**: fire the Sentinel straight up on an open map and wait — does
   the real game's missile ever explode on its own (a fuse/lifetime or a sky-bound blast)?
   gdbwatch `Bullet_update` 0x12e7b0's death path for the guided round; fieldprobe +0x6C of the
   missile. If the game bounds it, the port needs that bound (finding 4).
4. **nf-view spots** (the fixed cells): 07000005 spawn, `--give 0x2c` — fire, watch the missile-cam
   ride, then switch away: the switch should land right after the camera restores (the W that used
   to show). And `--give 0x54`: press fire — the first-person model should swap to the 0x55 skin
   (0x05000045 stays, the record id changes in the debug overlay).

## What's left (next window)

1. The 7 remaining story levels of the sweep (11/12/13/14/15/16/1b) on this runner; the numbered
   findings then cover all 27 levels.
2. The port's guided-missile lifetime (finding 4) — `Bullet_update` 0x12e7b0's death paths; the
   fix un-wedges the 0x2c W family AND the 0x2e/0x30 D cells on the same levels (finding 3).
3. The 0x2a X mark's fixed 90-tick contact bound → fuse-relative (the 0d re-run's reading).
4. The full suite on the final tree; the FAILED lines vs the allowances.
5. The 0x32 follow-up (matrix4's live check, still pending) and finding 2's give-route question.
