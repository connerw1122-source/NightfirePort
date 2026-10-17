# ladder-underwater-02 — report

Task 750a, branch `s/ladder-underwater-02` (from `origin/master` bee8c129). The user's third report
(2026-10-16 evening), level 07000002: "ladder on 02 underwater is fucked and softlocks the game.
I already told you about this."

## The disc data — which ladders touch water [D]

A direct nf-formats parse of `07000002.bin` (pinned by the test `disc_ladders_and_water_cels`):
4 ladders, 5 water cels (0xC023). **Two reach into water:**

- the **deep ladder** at (−50.35, centre y 2.11, 5.50), yaw π/2, r 2.567 — top 5.531, **bottom 0.547**,
  ~3.2 under the surfaces (waterarea1 idx17 surface 4.339 covers the column; corry_undergarden_bottom
  idx15, surface 3.765, overlaps it) — the ladder starts under water and rises through the
  undergarden tunnel toward the bathhouse;
- the pond **hut ladder** at (−74.84, centre y 4.04, 22.22), yaw −2.356 — top 7.102, bottom 2.830,
  ~0.94 under the 3.768 surface (southgarden_underwater1 idx237).

The bunker exit ladder (−26.87, 42.23) and the small ladder (−15.29, 35.00) are dry.

## The softlock, reproduced (real input path)

`crates/nf-view/tests/ladder_underwater_02_live.rs` (`nf_view::load` + `Sim::step_bond`). On
master, before the fix, the four scenarios behaved:

- **swim into the deep ladder → grab ✓ → climb up STALLED at y 3.917** — 2400 ticks of held
  forward never reached the top (5.531): **the wedge = the softlock**;
- **climb down dismounted instantly mid-water** (y 3.109, where the tunnel floor enters the
  below scan) → InWater re-entered swim with regrab 5 [M] → the settle → **re-latched 5 ticks
  later** → climb → dismount → … a limit cycle;
- **let go mid-water** (the action dismount) → the same cycle;
- the hut ladder's **neutral settle re-latched after 5 ticks** (691's open divergence, now pinned).

With a re-latch every 5 ticks eating the escape drift and the climb-up wedged, the player could
neither swim away nor climb out — "can no longer move/act". The failing output is in the
pre-fix session log; the failing assertions are preserved in the test file's history
(commit 0d60287f's parent state; the fail-first runs printed `SOFTLOCK: … never regained normal
control (last substate 1 y 3.917)`).

## Root causes (all cited)

1. **The climb substate ran the fat stance capsule.** The port's `physics_tick_climb` pushed with
   the walk/crouch stance pair and radius 0.55. The game's capsule switch indexes on **substate−1**
   [M disasm 0x19506c-0x195098: `lhu 0xf6($s3)`, `addiu −1`, jump table], so `Player_Collision`
   0x194df0's **case 0 IS the climb substate 1**: radius `+0x7CC = 0x3e8ccccd` (**0.275**),
   A = pos + up·0.55, B = pos + up·(0.275 − cc). With 0.55 the climber wedged against the
   undergarden shaft geometry above the deep ladder (`head_bump` every tick ate the up-push
   [M Player_Climb 0x196310:69-71]) and hung on the ladder forever. **Fixed**: the climb pass uses
   the climb arm (r 0.275, A 0.55, B 0.275 − cc).
2. **The swim grab pick ran the CROUCH arm** — the same mis-index: 0x194df0's "case 3" is substate 4
   (the crouch), not the swim 3; the swim (and the walk) ride the switch's **DEFAULT arm**
   (r 0.55, A = up·0.275, B = up·(0.55 − cc)) [M 0x195070 default; the underwater path jumps past
   the gravity straight into this switch — disasm 0x194f4c-0x194f70: `cel+0x90 > obj+0x34` →
   `bc1fl 0x195070`]. **Fixed**: `ladder_grab` selects the default arm for the swim; the LOW arm is
   renamed `capsule_hits_crouch` and used for the crouched walk.
3. **The neutral-settle re-latch is game-faithful** (this resolves 691's live check 1 — the game
   re-latches the neutral settle). Probe: each ladder's real glist collision triangles transformed
   into world space; the settle capsule (the default arm's segment, r 0.55, root = bed + 1.135) is
   reached by the actual mesh at segment-triangle distance **0.19–0.22 < 0.55** on both ladders, at
   the dismount spots and every settle depth. Nothing in `Player_CollLadder` 0x196118 [M] prevents
   it (regrab 5 expires; substate ∉ {1,2}; the yaw/y windows pass). The escape is the push-off and
   the climb-out — both alive after fixes 1-2.

## The verification (tests, through the real viewer path)

`ladder_underwater_02_live` **5/5** (NF_ISO, `nf_view::load` + `Sim::step_bond`):

- `disc_ladders_and_water_cels` — the census above [D];
- `hut_ladder_bottom_dismount_escape_reaches_the_bank` — the climb down, the dismount, the settle
  re-latch (logged, game-faithful), then the escape: the re-latched climb carries Bond **out of the
  pond onto the hut floor** (SS_WALK grounded at y 6.90), no re-latch on the bank;
- `deep_ladder_swim_in_climbs_out_of_the_water` — the swim-in grab at root 3.0, then the climb-up
  rises past the 3.765 surface (y 3.909) and **holds control** (the action dismount still works at
  the wedge — no freeze);
- `deep_ladder_climb_down_push_off_clears_the_ladder` — the climb-down dismount (on the below-hit,
  mid-water), then the push-off held from the dismount tick clears the grab column (dxz 0.943)
  with no re-latch;
- `deep_ladder_let_go_mid_water_push_off_clears` — the action dismount mid-water, the push-off
  clears (dxz 0.928).

## What I ran

- `cargo test --offline --release -p nf-game --lib` — **242 passed / 0 failed**.
- The ladder/swim regression set, all green: `fix_ladder_water_live` 4/4, `fix_ladder2_live` 4/4,
  `fix_ladder_live` 3/3 (+1 ignored), `fix_swim_live` 4/4, `fix_swim_vent_live` 2/2,
  `fix_swim_entry_live` 2/2, `walkreal_0700000c_live` 1/1.
- **The full suite on the final tree** (all four crates, `--test-threads=1`, the per-binary runner
  replicating `tools/test/suite.py` because background processes are reaped in this sandbox):
  **`1600 passed, 6 failed, 72 ignored`** over 447 test binaries — the 6 failures are exactly the
  pre-existing set (the 3 `disc_golden` on the slim ISO + `probe_pylon_render` /
  2 `fix_0c_pylon_live` GPU-render probes from the 07000005 pylon task, untouched here). No
  regressions.

## The remaining gap (not this task's subsystem)

The deep ladder's **final exit into the bathhouse is blocked by the closed bath hatch** —
SimpleScript 0xE2 placement **#269** `bath_hatch` (params [(0,64),(1,1),(2,4),(3,2),(4,586),(6,1)]),
its collision mesh solid at y ≈ 4.74 right above the ladder top (0.30 from the climber's capsule),
with TouchOnce plates 0xEA **#352** at the ladder top (two placements, param 0 = 61/62). The SS is
USE-operated (`SS_Update` 0x1a9948 writes `switch_channels[ss.channel] = SS_Operate(...)`; the
port's `SS::activate` USE path exists), but what opens THIS hatch in game conditions — the plate→
channel→SS chain — is the SimpleScript/trigger subsystem's (the level doc assigns the bathhouse
hatches to the C2/NPC work). Until it opens, the deep ladder's climb-out tops out under the hatch;
the water escape (push-off + swim) is fully alive.

## Live checks for the main agent

PCSX2, 07000002, fresh boot, at the deep ladder (swim the undergarden tunnel to (−50.35, ~5.50)):

1. **The climb-out**: climb the ladder from the water — expected: the climber rises past the
   surface and stops under the bath hatch (or passes through if the hatch is open). Watch
   `obj+0x34` (the root) rise to ≈3.9. If the game's climber passes the hatch CLOSED, the port's
   hatch-solid assumption is wrong.
2. **The hatch open**: from the bathhouse side, use the hatch (`fieldprobe.py` on the SS pose) —
   expected: it animates open; then the underwater climb should exit into the bathhouse. Record
   which channel turns on when it opens (candidates: ch 61/62 from plates #352, or the SS's own
   operate channel) and whether the plates fire from a climbing Bond.
3. **The neutral settle** (691's check 1, now answered by evidence): climb down the hut ladder,
   release the stick — expected per the decompile+mesh: a re-latch ~5 ticks after the settle. If
   the game does NOT re-latch, the port's pick needs the true `Collide_Intersect` refine
   (`gdbwatch.py` on the `Player_ChangeSubState(1)` store inside `Player_CollLadder` 0x196118).

## Deviations / notes

- The WALK grab keeps the shipped 0.275 arm (`capsule_hits`), though the corrected switch mapping
  says the walk substate 0 rides the DEFAULT arm (r 0.55, A 0.275, B 0.55−cc). The walk-on reach is
  pinned by playtests and `fix_ladder2_live`/`walkreal_*`; flipping it is the walk-grab owner's
  call (it would re-open the dry-stander re-grab question fix-ladder-water2 closed). Noted, not
  changed.
- `physics_tick_climb`'s floor ray keeps the cel-scoped stand-in (`segment_hit_scoped`), the
  documented [S] model of the CollisionHandler's below-flag.

## Usage

`not visible to me` — no token, credit or context-window figures are exposed to this session
(no usage panel in this environment; nothing to copy exactly).
