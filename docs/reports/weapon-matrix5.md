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
   **0x2a on 0700000d is NOT re-measured yet** — 0d is a story level and the window went to the
   fixes + the wrapper sweep; it is the next window's first sweep item.

## The sweep — where it stands

`probe_reds` (= `nf_view::matrix::run` + `mark_table` + `red_cells` + `aggregate_table`; the same
runner `nf-extract weapon-matrix` drives) measured this window:

- **07000005 (the in-suite sample level): 98 cells, 3 reds, 0 panics** — 0x32 F, 0x38 F (below),
  plus the by-design 0x01 `.`. The 0x2c Sentinel is CLEAN (gap 2 fixed). The 0x54 watch is CLEAN.
- **The seven wrappers 41/43/46/48/49/4a/4b: 686 cells, 29 reds, 0 panics.**
- Panics: **0** everywhere (the catch_unwind net never fired).
- The remaining 19 story levels (01-0c, 0e-16, 1b) did not fit the window — the next window runs
  them with `NF_MATRIX=all cargo test -p nf-view --test weapon_matrix_live -- --ignored
  --test-threads=1` or `PROBE_LEVEL=` chunks through `probe_reds`.

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
3. **0x2e AT-600 Scorpion — D (draw failed) and 0x30 Torpedo Launcher — D** on the three front-end
   wrappers 48/4a/4b ONLY. New since matrix4's run (its 4a/48 reds were only 0x2c W / 0x04 W).
   Suspicion: my gap-2/gap-1 changes altered the END state of the preceding cells (the away switch
   now lands where it used to fail, the fire presses swap the watch), so the next cell starts from
   a different held record on these maps; the draw refusal is downstream state, not the Scorpion.
   NOT root-caused this window — exactly what the next window's first story-level chunks will
   confirm or clear (the same ids were clean on 05).
4. **0x2c AT-420 Sentinel — W on 43/48/49 (+ 4a/4b per the mark table)** — the ride-out expires:
   on the wrappers the guided missile NEVER dies inside the 700-tick bound (open-sky geometry; on
   05 it hits something in seconds). The honest arm is documented in the harness ("a missile that
   never dies cannot hang the cell — the cell then records the honest mark"). What's wrong: either
   the port's missile lacks the game's lifetime/world-bound blast (the game's guided round must
   eventually explode — a `Bullet_update` 0x12e7b0 lifetime/fuse read is the next evidence step),
   or the harness should steer the missile down like a player does. Live check below decides.
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

1. The 19 story levels of the sweep on this runner (chunks through `probe_reds`); the numbered
   findings then cover all 27 levels.
2. The MGL 0x2a on 0700000d re-run (the task's item; the X from matrix4).
3. Root-cause finding 3 (the 0x2e/0x30 D on the front-end wrappers) — first check whether the
   story levels reproduce it, then trace the preceding cell's end state on 48.
4. The full suite on the final tree; the FAILED lines vs the allowances.
5. The 0x32 follow-up (matrix4's live check, still pending) and finding 2's give-route question.
