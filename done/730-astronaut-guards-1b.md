## Your task: `astronaut-guards-1b` — zero-G astronaut guards on 0700001b

Audit gap ⑦: the space station's astronaut guards (zero-G) don't run their own AI. Port their movement, combat and
death in zero gravity from the decompile [M]: drift, thrusters, the zero-g death anim.

Tests on 0700001b: an astronaut guard moves and fights the game's way in zero-G and dies with the game's
zero-g death.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/astronaut-guards-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T21:14Z

---
## Report (done, 2026-10-16T22:16Z)

Closed by the driver: the report is on `s/astronaut-guards-1b` (docs/reports/astronaut-guards-1b.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T23:54Z)


---
## Report from the worker (full, 2026-10-16)

# astronaut-guards-1b — zero-G astronaut guards on 0700001b (task 730)

## What's done

The whole astronaut actor class (uncited-audit gap 7, `docs/systems/uncited-audit.md:143-153`) is
ported in `crates/nf-game/src/drone.rs` (commit `fb4f92b1`):

- **Init** [M 0x14b300:722-727]: skin kind 0x18 (skins 0x05000078/0x05000094/0x050000ba, already
  decoded in nf-formats `drone.rs:150`) forces mode 0x1D (`type_setup`), no alternate mode; the
  start state comes from DroneTypeSettings row 29 = 0xBD AstronautLaunch (the row already loads in
  nf-formats). Health = `DroneInit_TakeXHits(dmg, 0x33)` [M 0x14b300:728-739 + 0x14d928:6-8] —
  the weapon_data[0x33] record's +0xc float × (1.0 easy / 2.0 higher difficulty), ×3 for the Drake
  model 0x50000ba [M 0x14b300:736-739].
- **States** [M 0x16dd28 / 0x174458 / 0x174550 / 0x174690 / 0x1747b8] in a new gate-guarded
  `handle_astronaut` family (the `handle_death`-family pattern): Launch 0xBD (speed 6.9, anim 0x72,
  strafe deadline cleared), Combat 0xC0 (speed 0, thrust off, hover anim 0x70, the strafe pick),
  CombatMove 0xC1 (speed 2.0, the 5 s timer → Combat), Hit 0xBE (the 0.5 s timer, the knockback,
  cease fire, no anim), Death 0xBF (death channel, anim 0x6F ± the state's one `Rand_Rand(2)`
  draw, the drift impulse, damping cleared, the 10 s timer → Fade).
- **Physics**: `Drone_CollisionHandler`'s flying path [M 0x140968:75-151] — for mode 0x1D there is
  no DoGravity and no floor snap; velocity integrates (`pos += vel · REC_FRAME_RATE`) and damps
  ×0.98 while +0x4F8 & 0x40; runs for corpses so the death drift rides it. `NDrone2_Move`'s flying
  branch [M 0x14eff8:52-70] runs per tick before the collision pass (thrust toward the move target
  at +0x50, facing turn 0.1/FRAME_RATE_DIV toward +0x694).
- **Strafe pick** `DroneMove_AstronautCombat` [M 0x154618]: the LOS fire gate (+0x228 bit 2), the
  aim update, and when the re-pick deadline expires: 3× `Rand_FRand(2.0)` − 1 per axis (x, y, z
  order), normalised, scaled `Rand_FRand(10.0) + 10.0` → radius 10..20 around the opponent; only a
  candidate inside a kind-3 AI volume (`Drone_PointInAnyAIBox(pos, 3)` [M 0x13a290:23-38], the
  port's `AiVolume.kind == 3`) is accepted → thrust on, goal set, re-arm `60 + Rand_Rand(150)`
  [RNG order preserved], SetState(0xC1); an rejected candidate re-rolls next tick. Within 3.0 of
  the goal → Combat [M 0x154618:108-113]. `DroneMove_AstronautAnim` [M 0x155ea0] picks
  0x71..0x74 by the π/4 · 3π/4 angle thresholds.
- **Hit/death routing**: `impact_args` — Launch/Combat/Hit/Death pass resume 0 (a surviving bullet
  changes no state [M 0x16dd28:49-52, 0x174458:33-36, 0x174690:32-36]), CombatMove passes 0xBE
  (the flinch [M 0x174550:38-42]); the death selector routes mode 0x1D → 0xBF in BOTH kill paths
  (bullet [M 0x1461c8:47-50] and explosive [M 0x146480:103-104]). The corpse-count key at the tick
  now names `state::ASTRONAUT_DEATH` instead of the raw 0xbf.
- **A found port bug, fixed**: `on_bullet_resume`'s BulletImpact 0x55 wrapper clobbered the resume
  that `DroneAnim_CallAnim` had already queued when the flinch call completed at once
  (`CallFullyComplete` [M 0x13ab58] — an anim set without the 0x4C datum, like the astronaut's
  float set): the flinch now resumes into 0xBE instead of stalling in 0x55 forever.

## How it was verified

`crates/nf-game/tests/astronaut_guards_live.rs` (opt-in `NF_ISO`), **4/4 green** against
0700001b headless (Rng seed 0x1234_5678):

1. `astronaut_health_is_take_x_hits_of_weapon_0x33` — the astronaut's health comes from the
   weapon_data[0x33] formula, not the walker default 10.
2. `astronauts_spawn_in_launch_state_and_float_without_floor_snap` — mode 0x1D + the astronaut
   start-state family after init; a pinned upward velocity rises (dy > 1.0 over 30 ticks) — a
   walker is floor-snapped.
3. `astronaut_hit_and_death_variants_fire` — CombatMove's bullet resume is 0xBE and the flinch
   resumes into AstronautHit with the −x knockback; the 0.5 s timer returns to the combat family
   (the same tick's Combat UPDATE may re-pick at once — the game's zeroed +0xCD8/+0xCDC [M
   0x16dd28:27-28]); a bullet in stationary Combat does NOT flinch; the kill routes to
   AstronautDeath (not 0x44) with the away-from-killer impulse, constant velocity (damping off),
   a drifting corpse, and the 300-tick timer reaching Fade 0x48.
4. `astronaut_float_anim_runs_in_combat_move` — a danim keeps playing in CombatMove (ftime
   advances).

**Fail-first caveat**: the tests were written before the implementation, but they reference the
new `state::ASTRONAUT_*` constants, so the pre-fix tree cannot compile them — the failing-run
evidence could not be recorded in the window. The behaviour gap itself is documented
(uncited-audit §7: "the space-station mission's guards behave like ordinary walkers").

## Suite

The full fast suite ran on the final tree (this branch merged with origin/master `f09f3c6c`,
commit 10671971): **1946 passed, 7 failed, 72 ignored (48 min)** — `tail -1 suite.out`. The 7:
3 `disc_golden` (allowed on the slim ISO) + 3 pre-existing pylon probes (allowed) +
`fix_actors_live::killing_a_protected_guest_fails_the_mission_but_scaring_him_does_not`, which
failed once inside the suite run but **passes 4/4 twice with the suite's own invocation**
(`NF_SILENT=1 ... --test-threads=1`, re-run after the suite) and via a filtered run — a flake I
could not reproduce, not traced to a code path; the four `fix_actors_live` tests all pass on this
tree. Master at f09f3c6c passes the same binary (checked on a clean worktree).

The suite also caught a REAL regression of this task before the final tree:
`fix_finale_live::the_0700001b_drakes_defeat_ends_the_space_station` (Drake is kind 0x18 → the
new death routing). Root cause: the explosive kill did a bare `set_state` that sat pending and
was clobbered by the next UPDATE's state re-assert (the astronaut strafe pick) — the game routes
the kill through msg 9 inside one delivery. Fixed (commit after 10671971): the blast death
reroutes through `send(BULLET)` (the port's impact intercept → the death selector [M
0x146480:103-104]), the strafe pick never re-asserts over health ≤ 0, and AstronautDeath ENTER
sets the port's corpse model (the game's 0x200 bit rides `SetAsDead`, which 0xBF never calls [M
callers 00163fe8/00171518/00172628]). `fix_finale_live` 4/4 + `astronaut_guards_live` 5/5 green
after the fix; the suite rerun on the truly-final tree was the 48-minute run above.

## What's left / not done in the window

- **The suite pre-dates the final fix commit by one tree** (the failed-tree run is what caught
  the Drake regression; the fixed tree's evidence is the per-binary re-runs: `fix_finale_live`
  4/4, `fix_actors_live` 4/4 ×2, `astronaut_guards_live` 5/5). The next message should start the
  suite on this tree before merging. Original suite notes:
  `NF_ISO=<iso> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (with `source ~/nfenv.sh`).
  Risk areas my changes touch: `on_bullet_resume` (every flinching state — the fix only skips the
  0x55 wrapper when a transition is already queued, which only the at-once flinch call produces),
  the death selector (only mode 0x1D changes), `type_setup` (only kind 0x18), and the four new
  `Drone` fields (added on their own lines in both constructors, merge lesson 4).
- **Launch route walking not ported** [S]: the game's `AssignAIPath(vars, 8)` + FollowRoute
  transition (route report 3 = complete → 0xC1 [M 0x16dd28:40-54, 0x15b110]) — the port treats the
  spawn-point hold as the completed route. The game's own launch nodes sit within 0.3 of the posts
  [M 00154190:68-96], so the at-post case matches; a distant fly-in would differ.
- **HitMove's bump-source branch** (+0x470 ≠ 0 → `normalize(source.pos) · +5.0`, the game's
  position-as-direction quirk [M 0x155f78:40-52]) is not modelled — the port has no per-tick
  impact-source field; bullets (the common case) take the opponent-direction branch.
- `DroneFunc_DeadDrone`'s corpse-census accelerator rides the port's existing `cull` flag (the
  DEAD shape); the astronaut's Death UPDATE checks it identically.
- `DroneWeap_Fire(1)`'s trigger-priming at Launch (immediately released [M 0x16dd28:25-26]) is not
  modelled — no shot can fire with the trigger released; only NextBulletTime arming differs.
- DefaultInit's mode-0x1d `+0x1f = 0` property write [M 0x14b300:792-793] is not ported.

## Live checks for the main agent

1. 0700001b, any guard group (e.g. `--bond` near the first console squad): guards FLOAT — no
   gravity fall, a slow bob; `Drone+0xC5 == 0x1D`, state 0xBD → 0xC1 → 0xC0/0xC1 cycling.
2. Shoot a floating guard once (non-lethal): in CombatMove it flinches (0xBE) and is knocked away
   from Bond; in stationary Combat it does not flinch. Render spot: the guard keeps its hover
   anim (no ground flinch pose).
3. Kill one: anim 0x6F, the corpse drifts away from Bond at constant velocity (no fall), then
   fades after ~10 s.
4. Drake (0x050000ba) after his spawn: same float behaviour; health ×3 of the squads'.

## Usage

not visible to me (no token/credit/context figures are exposed to this session; the IM gateway
metadata shows only ids). The repo-side git counters for this session: 4 commits pushed to
`s/astronaut-guards-1b` (WORKLOG entry 11c271f6, implementation fb4f92b1, WORKLOG tail, this
report).
