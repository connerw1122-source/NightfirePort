## Your task: `satchel-stick-play`, the satchel bounces instead of sticking in play

The user (playtest 1015-0046, 07000005, `--give 0x3b,0x3a`, mouse + keyboard): "[The satchel] doesn't stick to walls or
to the ground and it bounces." 109 satchel-tripbomb (`docs/reports/fix-satchel-tripbomb.md`) ported the +0x100 stick arm
and its tests pass. The main agent's headless probe through the real input path (`nf_view::load_weapons` with 0x3b,
equip + `switch_to`, 150 idle ticks, pitch −0.3, one trigger tick) shows the satchel released at press+75, flying, and
STUCK on the snow at tick ~105 (`g.stuck == true`, velocity 0, never moving again) — so the plain case works. Find the
cases the user hit that the probe doesn't cover: a throw straight at a nearby WALL (the 05 spawn's low wall at ~1–3
units, and the user's spot `--bond 105.97,24.87,110.12,-4.369,-0.166`), a throw while moving, a throw at a steep or
non-axis-aligned surface, the first-tick contact (spawned already touching/inside a surface), the world-hit test the
stick arm uses (`Bullet_CollisionHandler` 0x12ce80:255-266) against the port's `segment_surface` (a missed hit
→ the bounce arm runs instead). Reproduce a bounce in a test first, then fix it against the decompile.

Files: `crates/nf-game/src/grenade.rs`, `crates/nf-game/src/drone.rs` (the Stick consumer), a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/satchel-stick-play`.

---
claimed-by: sandbox5 at 2026-10-15T07:49Z

---
## Report (done, 2026-10-15T10:37Z)

Closed by the driver: the report is on `s/fix-satchel-stick-play` (docs/reports/fix-satchel-stick-play.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-15T10:41Z)


---
## Report from the worker (done, 2026-10-15T10:41Z)

# fix-satchel-stick-play — the satchel bounces instead of sticking in play

**Verdict: no port bug found — nothing reproduces; the port's satchel cannot bounce off the
world.** Every case the driver listed, run through the real input path on this exact tree, ends
STUCK. The most likely explanation of playtest 1015-0046 is that its build predated the
satchel-tripbomb fix (8159e452, merged 2026-10-14 23:13 UTC; the playtest ran ~90 min later).
[S] — the one claim in this report that is a hypothesis, not a measurement.

## What was wrong (the user report)

Playtest 1015-0046 (07000005, `--give 0x3b,0x3a`, mouse + keyboard): "[The satchel] doesn't stick
to walls or to the ground and it bounces." The tripbomb task's probe had already shown the plain
lob sticks on the snow (released at press+75, stuck at ~tick 105). This task hunted the cases the
probe missed.

## What was tried — the case matrix (all STICK)

New test file `crates/nf-view/tests/fix_satchel_stick_play_live.rs` (5 tests, all green on this
tree), every throw through the real input path (`nf_view::load_weapons` give 0x3b+0x3a → the
panel switch → `switch_to` → the fire pipeline → `Sim::step_bond`):

1. **The user's exact spot** `--bond 105.97,24.87,110.12,-4.369,-0.166`, aimed at the spawn's low
   wall (~1–3 units): STUCK at t30 on the wall's top, normal (−0.029, 0.999, 0.046).
2. **The throw while moving** (forward held through draw + trigger + flight): STUCK at t11 in the
   corner on the yawed wall, normal (−0.913, 0, −0.407).
3. **A steep / non-axis-aligned face** (a 16-ray × 3-pitch fan re-aims Bond at a hit face with a
   tilted normal, 0.35 < ny < 0.85): STUCK at t11.
4. **The repeat throw** — a second satchel at the same wall while the first is stuck: BOTH stick.
5. **The straight-down throw** (pitch −1.2 ≈ the first-tick contact case — the grenade meets the
   ground within ticks of spawning): STUCK at t20 on the tilted floor, normal (−0.046, 0.995, 0.083).
6. **First-tick contact unit sisters** (in grenade.rs's `mod tests`, +35 lines, test-only): a
   grenade spawned ON the wall face moving into it sticks on the FIRST update; a grenade spawned
   INSIDE the face is skipped by the one-sided ray exactly like the game's
   `Intersect_RayGeom` 0x10a8c8:150-168. Both green (nf-game lib 164/164).

Sister binary `fix_satchel_tripbomb_live` stays 5/5 green.

## Why the port cannot bounce — the world-hit test, with the addresses

The driver's last hint was a missed world hit routing the satchel into the bounce arm. The
decompile says that cannot happen, on the game's side or the port's:

- **The game's satchel world test is a ZERO-radius, one-sided ray.** [M `Bullet_update`
  0x12e7b0, case-1 tail, decomp lines 393-414 + disasm 0x12f0a4-0x12f0b8]: the move builds the
  HITTEST each tick; the id switch sets radius 1.0 for the MGL 0x2a/0x2c-0x2f and **0 for the
  whole grenade family 0x2b/0x34-0x37/0x3a-0x40** (0x3b included, decomp line 407 → 413). The
  version field is 0x201 (the ray form) when the move ≥ half of obj+0x8c, else the 0x101
  point/sphere form [M lines 430-432]. One-sided entry faces: [M `Intersect_RayGeom`
  0x10a8c8:150-168]. The port's `segment_surface(pos → pos+vel·dt)` is a transcript of exactly
  that shape, and the port's segment includes the endpoint (t ∈ [0,1]), so it hits at least
  everything the game's test hits — **a missed hit cannot turn the port's satchel into a
  bouncer.**
- **The stick arm pre-empts the bounce arm on ANY world hit.** [M `Bullet_CollisionHandler`
  0x12ce80:234-266, re-read in the decompile this session]: `+0x70 & 0x100` record + `bVar7 == 0`
  (a world hit; a char hit takes the :239-249 bounce) → `obj+0xF4 = 3`, the matrix aligned to the
  NEGATED hit normal, park at `hit + normal·0.04` + Sound 0x4a7. The port mirrors the order:
  [M grenade.rs:299-349] on any `segment_surface` hit the 0x3b arm sticks and **returns**; the
  bounce block below it is unreachable for 0x3b world hits.
- The 0x101 slow-move form degenerates to the same point test for the family because the radius
  is 0 (AABB + Dist2Tri [M fix-08-walls/fx-safe08b]) — still no bounce path.

**No code fix: nothing reproduces, and the task's rule is don't guess a fix.** The task's file
list (grenade.rs, drone.rs) needed no production change; the only grenade.rs lines this task
added are the two first-tick unit sisters in `mod tests`. Shared files: **none touched** by this
task (drone.rs/scene.rs/player.rs/lib.rs unchanged; the scene.rs/hud.rs/gadget.rs lines in
commit a6f536a6's stat belong to the merged fix-laser-watch-look task).

## How it was verified (this session, fresh sandbox)

- Fresh tree at 92c85643 (merged origin/master 56d6dc24): `fix_satchel_stick_play_live` 5/5,
  `fix_satchel_tripbomb_live` 5/5, nf-game lib 164/164.
- Full four-crate suite via `tools/test/suite.py` in foreground chunks (limit 90 s; wall-cut bins
  re-run at 270 s): **1289 passed / 55 ignored**; failures: 3 `disc_golden` (the allowed slim-ISO
  three) + 3 pylon-render (`fix_0c_pylon_live` ×2, `probe_pylon_render` ×1 — another session's
  known scope, pre-existing on master). Wall-cuts at 90 s, all green at 270 s:
  fix_guard_weapons_live, levels_sweep_live, campaign_flow_live, walkreal_07000001_live.
  `weapon_matrix_live` needed a longer window and passed solo at limit 600 s (6 passed / 0 failed /
  24 ignored, ~5 min) — a long binary, not a hang.

## What's left

- Nothing blocking. The [S] build-predates-fix hypothesis can only be closed by a playtest
  rebuild: the main agent should re-run the user's exact repro (07000005, `--give 0x3b,0x3a`,
  throw at the low wall).

## Live checks for the main agent

1. **PCSX2 rebuild playtest** (the decisive one): 07000005, `--give 0x3b,0x3a`, stand at
   `105.97,24.87,110.12`, aim at the low wall (yaw −4.369, pitch −0.166), throw, then throw again.
   Expected (port = game): both satchels stick, first on the wall within ~0.5 s, the second
   sticks too. If the game bounces, attach `gdbwatch.py` on the stuck bullet's obj+0xF4
   (expect write 3 at the stick) and obj+0x30/0x34 (position/velocity frozen).
2. **nf-view `--bond` spots** (what each should show):
   - `--bond 105.97,24.87,110.12,-4.369,-0.166` — Bond at the user's spot facing the low wall;
     after a throw the satchel visibly parked ON the wall's top face.
   - `--bond 105.97,24.87,110.12,-4.369,-1.2` — after a straight-down throw the satchel parked on
     the tilted floor at Bond's feet.
3. **Optional trace**: `gdbwatch.py` on `Bullet_CollisionHandler` 0x12ce80's stick store
   (`obj+0xF4 = 3`, 0x12d1fc block) during a wall throw — expect exactly one write per throw; no
   writes to the bounce store (LAB_0012d4bc block) for 0x3b.
