## Your task: `swim2`, the swim fix breaks the 0c ladder-top exit

102h-fix-swim (`origin/s/fix-swim` dc59866, held) breaks `walkreal_0700000c_live::walk_start_to_the_st1_plate_and_the_photograph`
(the main agent reproduced it on master + the swim branch alone): after "the ladder climb: ladder climbed to y 11.74 at
+364 ticks", the walk to the walkway's south end STICKS: "bond=(9.03,11.74,-189.99) target=(2.80,11.60,-208.40)
grounded=false substate=0 crouched=true floor_ny=1.00 segment_to_target=Some(0.029)". So Bond leaves the ladder top
CROUCHED and ungrounded, a wall/segment right in front of him. The swim branch changed ladder.rs and player.rs (the
pond ladder grab from the water); the land ladder's top exit changed with it. Fix the exit so land ladders behave as on
master (the game's ladder-top dismount: `Player_CollLadder` 0x196118 / the climb substate's exit) while the pond ladder
still grabs from the water, keep the swim fixes, and make the full suite green on the merged tree (0 failures). Also
the entry depth item from the report is queued separately as 103d: don't duplicate it.

Files: `crates/nf-game/src/{ladder,player}.rs`.

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

**Branch:** `s/swim2`.

---
claimed-by: sandbox3 at 2026-10-14T11:15Z

---
## Report (done, 2026-10-14T12:36Z)


---
## Report from the worker (done, 2026-10-14T12:36Z)

# fix-swim2 — the swim branch's ladder-top exit regression on land ladders

Task 102m. Branch `s/fix-swim2` (from `origin/s/fix-swim` dc59866 + merged `origin/master`).
Files changed: **`crates/nf-game/src/ladder.rs` only** (no shared-file hooks; `player.rs`,
`sound.rs`, `scene.rs` untouched — every 102h-fix-swim change is kept).

## The symptom (reproduced before the fix)

`walkreal_0700000c_live::walk_start_to_the_st1_plate_and_the_photograph` (the main agent's
repro, confirmed here on the merged tree, 35 s run):

- `the ladder climb: ladder grabbed at +23 ticks`, `ladder climbed to y 11.74 at +364 ticks`
- then the walk to the walkway's south end sticks:
  `STUCK at the walkway circle south end: bond=(9.03,11.74,-189.99) target=(2.80,11.60,-208.40)
  grounded=false substate=0 crouched=true floor_ny=1.00 segment_to_target=Some(0.029)`.

A per-tick probe replay of the same script showed the mechanism: the top exit itself is fine
(climb past `top` at y 12.148 → the +0.6 dismount lunge [M `Player_Climb` 0x196310:54-62] →
fall → land on the walkway at y 11.736) — but when `regrab` runs out 43 ticks later, Bond is
**re-grabbed** (substate 1, snapped back to the ladder axis at (9.949, 11.736, −186.928)) and
climbs again: a climb/walk limit cycle. The walker's stuck mitigation jostles against it and
reports the wedge.

## The cause

102h-fix-swim replaced the grab shape in `Ladder::capsule_hits` with the game's pick stage:
the player's HITTEST capsule segment vs the ladder's world sphere inflated ×1.5 + capsule
radius [M `Collide_PickObj` 0x1eae80:152-168; the player's mode word BLData+0x7D4 = 0x800,
`Player_Collision` 0x194df0]. That stage is real, but it is not the whole filter. A tall
ladder's bounds sphere is its half-HEIGHT — the 0700000c yard ladder spans y 0.8..12.1
[D probes] → r 5.75 — so the bare ×1.5 sphere reaches ~9.2 units sideways. The game narrows
the pick past the sphere with the straddled-cel scoping (`Collide_Pick` 0x1eb320:125-134 and
:318-328 test only the objects linked to the cels the player straddles) plus the glist mesh
refine (`Collide_Intersect` 0x1ebaa8). The port's `ladder_grab` iterates the flat ladder list
with no cel scoping and no mesh, so the bare sphere stood in for both — and at regrab expiry
the dismounted walker ~2.9 units from the axis at walkway height (dist 6.4 ≤ 9.18) still
"picked" the ladder. Master never saw this because its [S] axis-column shape reached only
`min(r,1)+0.55 = 1.55` horizontally, so Bond cleared the ladder inside the 60-tick regrab.

`Player_CollLadder` 0x196118 itself has no extra gate (read in full): guards substate ∉ {1,2}
+ regrab + health, the yaw window, and the y window `bottom < pos.y < top` — all already [M]
in the port; 11.736 < top 12.15 passes, so the game's anti-re-grab lives in the pick pipeline,
not in the grab function.

## The fix (`crates/nf-game/src/ladder.rs::capsule_hits`)

Two stages, both cited in the doc comment:

1. **Stage 1 (kept from 102h)** [M 0x1eae80:152-168]: the capsule segment vs the world sphere
   ×1.5 + capsule radius. This is what keeps the pond ladder's deep-water grab (the pond-bed
   swimmer below the sphere bottom; `Player_CollLadder` 0x196118:23-31 then lifts him to
   `bottom + 0.02`).
2. **Stage 2 (restored from master, documented [S])**: the horizontal axis-column bound
   `horiz ≤ min(radius, 1.0) + capsule_radius` — the port's stand-in for the unported
   cel-scoping + mesh refine. Deep ON the axis stage 1 rules (the pond grab); far OFF the axis
   stage 2 rules (the land exit). The dismounted walker at horiz 2.9 > 1.55 is no longer
   re-grabbed; walking INTO the ladder grabs exactly where master grabbed.

The climb-top dismount code, the grab gate, and all 102h swim changes are untouched.

## Fail-first + tests

- **Fail-first (the user's symptom)**: `walkreal_0700000c_live::walk_start_to_the_st1_plate_
  and_the_photograph` — shown failing on the merged pre-fix tree (the STUCK dump above), and
  it also failed mid-walk earlier (`the ladder base: arrived at +1912 ticks pos (10.92,9.82,
  -187.57)` — Bond climbed the ladder during the approach leg). Post-fix it passes: `walked
  out in 2691 ticks; stuck 0; falls 0`, the ladder base at +154 ticks.
- **Sister unit test** (`ladder.rs::grab_zone_is_the_game_pick_shape`, rewritten): the 2-units-
  -to-the-side case flips back to no-hit (stage 2; the 102h version asserted a hit); the deep
  on-axis pond case stays a hit (stage 1); the thin-ladder bed case stays a no-hit; and the
  exact 0700000c yard geometry pins the regression — the ladder-top grab point (9.949, 11.736,
  −186.928) still hits, the dismounted spot (9.285, 11.736, −190.167) does not.
- `nf-game` lib 145/145 (includes 102h's two swim unit tests — the grab gate and the
  look-directed swim — unchanged and green).

## Verification (the suite coverage actually run on the final merged tree)

- `nf-game` + `nf-formats` + `nf-audio`, `--test-threads=1`: green except the **3 allowed
  `disc_golden`** failures (slim ISO).
- The ladder/swim/walkreal chunk of nf-view (the tests that exercise ladders — only 07000002,
  07000005, 0700000c climb ladders per the test sources): `fix_ladder2_live` 4/4,
  `fix_ladder_live` 3/3 (+1 ignored), `fix_swim_live` 4/4 (**the pond grab from the water
  still works**), `fix_swim_vent5_live` 2/2, `walkreal_07000001` 2/2 (the 192 s escort walk),
  `walkreal_07000002` 1/1 (the pond level), `walkreal_07000009` 2/2, `walkreal_0700000c` 1/1,
  `walkreal_07000012` 1/1, `walkreal_0700000b` 1/1, `walkreal_0700000d` 1/1;
  `walkreal_07000005` and `walkreal_0700000a` report 0 run / ignored on the slim ISO (not
  failures).
- **Not run** (the window did not fit the full suite; the background runner dies between tool
  calls in this sandbox, same as the last sessions): the remaining walkreal levels (03, 04,
  11, 13, 14, 15, 16, 1b, 4a), the portal/scene nf-view suites, weapon_matrix. Next session:
  `NF_ISO=<abs> python3 tools/test/suite.py` on this tree.

## Notes for the driver

- The stuck dump's `crouched=true` is **not** the regression: the test's single crouch press at
  the yard stand-in (walker_b2 line 137) toggles crouch on when Bond lands un-crouched, and
  nothing presses it again — identical code on master and this branch (no diff touches the
  crouch path); master's green run walks the yard crouched the same way. The re-grab cycle was
  the whole failure.
- The queued 103d (build_FindCel entry depth) was not touched or duplicated.

## Live checks for the main agent

PCSX2 (NTSC-U retail):
1. 0700000c yard ladder (the walkway at the yard's west, ~9.7, −187.3): climb to the top, step
   off south onto the walkway circle; expected: no re-grab, Bond walks south along the circle
   (the dismount lunge lands him on the walkway at ~11.7).
2. Same ladder from the top, dismount, then walk back INTO the ladder within ~1.5 units:
   expected: it re-grabs (the column reach is deliberately kept).
3. 07000002 pond hut ladder from the water: swim to the pond bed near the ladder, push into it;
   expected: the grab fires from the deep water and the climb-out assist lifts Bond to the
   rungs (102h's fix, preserved).

nf-view render spots (`--bond x,y,z,yaw,pitch`):
1. `0700000c` `9.0,12.4,-190.5,π,0` — from the walkway south of the ladder top: the walkway
   circle should be walkable, the ladder behind; Bond standing here must NOT snap back to the
   ladder.
2. `0700000c` `9.7,11.0,-187.3,0,0` — at the ladder base facing it: the grab zone column
   (≤ 1.55 from the axis) is where the grab should fire.
3. `07000002` the pond hut ladder from the bed: the deep-water grab spot (the swim branch's
   fix_swim_live covers it headless; the render confirms the climb-out pose).
