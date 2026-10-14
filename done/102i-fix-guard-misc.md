## Your task: `guard-misc`, the 0c sniper takes no damage; guards and doors; the 06 start poses

The user's playtest (results 1013-1825):
- 0700000c: "I heard sniper shots, looked at them, and I saw the enemy, but shooting him did nothing." (A rooftop sniper:
  check his hit volume / damage path / whether the port's bullet hits his collision at that range.)
- 07000006: "NPCs still can't open doors."
- 07000006: "as soon as I spawn into the level, the characters are doing crazy models, like weird model stuff" (the
  guards' first frames: a pose/animation blend from frame 0, or unposed bind poses before their start animation).
Reproduce each headlessly and fix from the game's code: the sniper's damage, the drones' door use (`NDrone2_ReachedDestNode`
door case, fix-nav-remainder ported the dispatch: find why 06's doors still don't open), and the drones' first-frame pose
(every drone posed from tick 0). Tests for each; sister assertions.

Files: `crates/nf-game/src/{drone,door,damage}.rs`.

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

**Branch:** `s/guard-misc`.

---
claimed-by: sandbox5 at 2026-10-14T09:01Z

---
## Report (done, 2026-10-14T10:27Z)

Closed by the driver: the report is on `s/fix-guard-misc` (docs/reports/fix-guard-misc.md); the worker didn't run finish.sh.

---
## Report from the worker (blocked, 2026-10-14T10:33Z)


---
## Report from the worker (blocked, 2026-10-14T10:33Z)

# fix-guard-misc — report

Branch `s/fix-guard-misc` (from origin/master 6cbe522). The task: the 0700000c rooftop sniper
takes no damage, the 07000006 guards can't open doors, the 07000006 guards render "crazy models"
on their first frames. Status: **blocked — the sandbox deadline hit before the fixes landed**; the
three repro harnesses, the census evidence and the discriminating facts are on the branch, and the
next session continues from them. No production code was changed this session (the port files the
task names — `drone.rs`, `door.rs`, `damage.rs` — are untouched).

## What's done

All three symptoms have headless NF_ISO harnesses through the real input path in
`crates/nf-view/tests/fix_guard_misc_live.rs` (new; `nf_view::load` + `Sim::step_bond`):

1. **`the_0c_sniper_takes_damage_when_shot`** — FAILS (the repro). Finds the rooftop sniper (the
   sniper-weapon 0x1E drone on the highest perch: drone #17 at (43.77, 21.43, −205.13)), enables
   him the way the level does — he sits in `WAIT_SWITCH` (0x33) with `switch_channel` 1; the test
   raises the channel via `Drones::set_channel` (the trigger's write) — stands Bond at a scanned
   standable clear-LOS vantage ((46.0, 13.2→14.2, −182.0); LOS verified with `world.segment` from
   eye to chest: None), aims and taps the trigger. Result: the sniper fights (state cycles
   0x94/0x95/0x99 — he shoots back), Bond's clip drains, **his health stays 10.0 / hits_taken 0**.
   Pre-fix state captured.
2. **`the_06_guards_open_the_doors_on_their_routes`** — FAILS (the repro). 7200 ticks on
   07000006 with Bond passive: **no drone ever enters `DSTATE_OpenDoor` (0x81)** and **no door
   opens without Bond's use key**.
3. **`the_06_guards_are_posed_from_their_first_frames`** — PASSES: `Sim::unposed()` is EMPTY at
   ticks 0/1/5/30/60/120/239 on 07000006 (88 characters). The "crazy models" symptom did NOT
   reproduce as missing/unposed animations (see "What's left" — it needs the main agent's renders).
4. **`probe_sniper_boxes_and_los`** — the discriminator dump (kept): drone #17's palette is 73
   matrices and he carries **16 hit boxes** (head part 5 at y 22.19, torso part 4 at 21.84, …) —
   the "no pose → no boxes → unhittable" theory is disproven for him. LOS facts: clear from
   (43.8, 12.2, −185.0) and (60, 12.2, −205.0)-class spots; blocked from the north/west ground.

## The evidence (facts, with addresses)

**The 0c sniper.** The bullet→drone path is `update_player_bullets` (`Collide_PickObj` broadphase
1.05-sphere at [M 0x1eae80:26-30], `Collide_Jointy` 0x1ec180 via `damage::first_box_hit`, msg 8 via
`Drone_BulletHit` 0x13a9f8 → `NDrone2_HitDamage` 0x145558 in `damage.rs`). The damage math is unit-
verified. What the session established: he is damageable (skin 0x05000087 is not in the port's
non-damageable skins [M `NDrone2_DefaultInit`]; boxes + palette exist; the world does not block the
line from the vantage; the shots fire and he does not lose health. What remains unproven is WHERE
the hit is lost — the harness aim drifts high (the sim pulls `bond.pitch` toward ≈0.187 normalised
while AIM is held; the setup-set 0.162 could not be held), so the fired rounds may pass just over
the head box (top 22.30; the crosshair plane at pitch 0.187 reaches y ≈ 22.7 at 23.2 units). The
next session should fire deterministically: push a `Bullet` (pos = the vantage eye, dir = the exact
chest segment) into `player_bullets` and tick — that removes Bond's aim from the equation and
discriminates "the ray never reaches the boxes" (broadphase `seg_point_dist2 > 1.05²·4` at [M
0x1eae80], world-vs-box ordering) from "the hit is dropped after `first_box_hit`" (msg 8 path,
`hit.damage`, `damage_taken`, `hit_damage`).

**The 06 doors.** The census (in the test log): every 07000006 guard stands at a post —
`route: None`, state 0x1 (a few play start scripts, state 0x3) — so the patrol/mission-route door
path (`NDrone2_ReachedDestNode` 0x153358 door case → `DSTATE_OpenDoor` 0x1681f8 → `NDrone2_OpenDoor`
0x14aa38 → `Door_Activate` 0x135be0, ported and green on 07000001) never runs for them in the idle
level. Door-flagged nav node pairs DO exist: path 3 n11–n14 around (4.7, 49.8, −64.2/−65.6) → door
#4 (4.0, 49.0, −64.88), and path 7 n0–n1 (35.9, 40.7, −15.7/−16.8) → door #1 (36.5, 40.0, −16.25).
The user's symptom must therefore come from the CHASE path: a guard pursuing Bond crosses the
doorway under `GoToGoalPosition` 0x63 (`LinkCreep_Handler` 0x15a2c8 serves the dispatch for every
followed route — fix-nav-remainder wired `route_reached_special` into `move_to_goal`). The next
session drives a guard through the pair with the 0x63 input path (the `nav_kick_live` pattern:
`dr.state = GO_TO_GOAL` + ENTER, goal beyond the door) and asserts the door reaches OPEN (state 4)
and the guard crosses; if that fails, the dispatch/binding is the bug; if it passes, the bug is in
the port's chase route-finding (a different link upstream of the dispatch).

**The 06 first frames.** Not reproduced headlessly: `unposed()` (the characters drawn in the bind
pose) is empty from tick 0, every drone's `anim.script` is set at creation (`set_danim(d,
start_danim, 0)` at [M `NDrone2_DefaultInit`]-order), and `palette`/`hit_boxes` resolve for the
sampled drones on both 06 and 0c. What the user filmed ("crazy model stuff" on spawn) may be: the
GPU-palette path's first frames, the spawner-held characters (the load log: "20 character meshes
held back by spawners"), the facial/blink layer (facial3), or a blend-weight glitch — none visible
to the headless checks I ran. I did not guess a fix.

## How it was verified

- The harnesses run on the slim ISO (`NF_ISO`) with `NF_SILENT=1`; both failing tests fail
  deterministically (doors: 23.9 s; sniper: 9–16 s per run; 5 runs).
- `nf-game` lib tests on the final tree: **143 passed / 0 failed**. The full four-crate suite could
  not run in this sandbox (background jobs die between tool calls — nohup/tmux/at all unavailable;
  same as the previous three sessions) and the foreground window was spent on the repros; the next
  session runs the suite on the merged tree before its push.
- No production code changed, so no existing test could regress from this branch.

## What's left (the next session continues from this branch)

1. **The sniper damage path** — deterministic-fire variant of the failing test (push the `Bullet`
   through `player_bullets`), find where the hit is lost, fix, cite [M] the deciding decompile.
2. **The 06 chase-door dispatch** — the GoToGoalPosition drive test through the path-7 n0/n1 pair
   (door #1); fix the failing link; the same test re-run on the path-3 pair (door #4).
3. **The 06 first frames** — needs the main agent's renders (below); no headless repro yet.
4. The full suite on the final merged tree.

## Live checks for the main agent

- **nf-view renders (07000006)**: `--bond 57.18,39.73,22.61,<yaw>,0` at the spawn, frames 0–10 and
  60: do the nearby guards show the crazy pose the user filmed, and which characters (the
  spawner-held 20?). A short capture of the first second is ideal.
- **PCSX2 (07000006)**: at the same spot, confirm the game's guards near door #1 (36.5, 40.0,
  −16.25) — do they patrol across it in the first minute (the port's census says they stand)?
- **PCSX2 (0700000c)**: when the tower sniper (43.77, 21.43, −205.13) is enabled and shooting, does
  the player's first shot draw a blood/spark effect on him (the game's `NDrone2_DoHitEffects`
  0x1458e0 sound 0x413/0x601) — i.e. does the REAL game show the hit landing at that range?
- **nf-view (0700000c)**: `--bond 46.0,14.2,-182.0,−3.22,0.55` — is the sniper visible from the
  yard vantage, and does a shot produce an impact on him in the port's render?

## ntfy failed

(none — the notification step ran; see the queue record.)

## Addendum (10:50 UTC, same session): the deterministic bullet and the miss margin

`probe_sniper_direct_bullet` (new in the test file) pushed the exact bullet — vantage eye
(46.0, 15.7, −182.0) → the settle-time torso centre — through `player_bullets` and ticked the real
`update_player_bullets`. Facts captured at the crossing tick:

- The bullet's step-5→6 segment runs (43.84, 21.65, −204.43) → (+ray (−0.37, 1.02, −3.85)); its
  crossing of the torso plane is (43.78, 21.81, −205.04).
- The live boxes at that tick are sane and where expected: torso part 4 at (44.02, 21.82, −205.04)
  half [0.2, 0.25, 0.15], head part 5 at (44.05, 22.17, −205.05), 16 boxes total; the sniper's
  `anim.script` is Some, `enabled`/`npc_enabled` true.
- `first_box_hit` = **None** on the real boxes — and the geometry says it is RIGHT: the drone
  shifted 0.2 between the settle probe (pos 43.77) and the crossing tick (pos 43.98), so the aim
  point (the stale settle-time centre) passes 0.24 world-x from the live torso centre — 0.04
  outside the 0.2 half-extent. Both the row-axis and column-axis slab variants miss the same way
  (both printed in the probe), so this is not a row/column convention bug.

Conclusion of the session's evidence chain: boxes exist, are placed on the body, the ray test is
sound, the broadphase/gates (`npc_enabled`, `enabled`, `present`) all pass — **the deterministic
shot missed by 0.04 of a unit because it aimed at a stale centre**. The next session's one-line
step: read part 4's live `m[3]` at fire time and aim exactly there (`first_box_hit` then must
return Some, and the health must drop through the real msg-8 path). The full-input-path test
(`the_0c_sniper_takes_damage_when_shot`) still fails for the same reason plus the documented
aim-mode pitch drift; both harnesses are on the branch ready for that step.
