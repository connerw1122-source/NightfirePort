## Your task: `searchlights-guards`, 06: the alarm, the searchlight beams, guards firing too soon

The user's playtest (results 1013-1825, 07000006): "Alarm still isn't going off" when walking into a courtyard
searchlight beam. "Sometimes a spotlight will cover a piece of ground and it will flip on its side, which looks really
weird." "The guy shoots at me immediately as soon as he sees me, his gun's firing before it lowers at me. Guards are
shooting me way too quickly after seeing me." PCSX2 at the user's spots (main agent, 2026-10-13): at
`--bond 7.21,42.07,-40.73,16.601,0.412` the port draws the beam as a bright flat slab on the wall and the searchlight heads
as white blobs; the real game shows dark heads and no slab; at `5.24,42.07,-29.30,18.498` the port's ground pool and cone
are much brighter than the game's.
Reproduce through the real input path: walk Bond (not teleport) from the 06 truck start up into a beam: the alarm channel
must fire (fix-searchlight-alarm tests teleport and pre-set ch 12; the walk is what fails); the beam/pool draw's
orientation and brightness against the game's searchlight render code; the guards' first shot: the aim-raise animation
must finish before the first bullet (`DroneFunc_ReactionTime` 0x14ac60 and the aim state's gate). Tests for each; sister
assertions; list `--bond` spots.

Files: `crates/nf-game/src/{searchlight,drone}.rs`, `crates/nf-view/src/scene.rs` (the beam draw).

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

**Branch:** `s/searchlights-guards`.

---
claimed-by: sandbox5 at 2026-10-14T05:07Z

---
## Report (done, 2026-10-14T06:24Z)

Closed by the driver: the report is on `s/fix-searchlights-guards` (docs/reports/fix-searchlights-guards.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T06:46Z)


---
## Report from the worker (done, 2026-10-14T06:46Z, full text)

# fix-searchlights-guards — the 06 courtyard: alarm on the walked entry, the spot disc's orientation, the guards' first shot

Branch `s/fix-searchlights-guards` (cut from origin/master 69c0104). Session 05:10–06:5x UTC
2026-10-14, sandbox deadline honoured. Files: `crates/nf-game/src/{searchlight,drone}.rs`,
`crates/nf-view/tests/fix_searchlights_guards_live.rs` (new).

## Done

1. **The walked-alarm repro, made permanent.** The task asked for the walk fix-searchlight-alarm's
   tests stood in for (teleport + pre-set ch 12). This session walked Bond from the truck start
   (#496, 57.18, 39.73, 22.61) to the 06-searchlight.trace's beam spot (4.80, 42.09, −16.00) with
   `Input` only — no `bond.pos` writes after the spawn. Route facts learned on the way: the spawn
   pocket opens EAST only (a 16-direction ray fan at the spawn hits within 3 units everywhere but
   +x), the nav graph does NOT connect the truck/cellar paths (D) to the crate-area paths (C) — the
   nearest D/C nodes are 6.8 apart at x≈52-56, z≈26-30 and that ground is OPEN (walked), so
   `nav_route`/`nav_route_global` return None and the direct chain wedges on the spawn wall. The
   working route (5 anchors + the direct leg) is in the test. **Result: the alarm channel 55 rises
   +109 ticks after reaching the beam spot on THIS tree** — the walk does not fail; the channel
   chain fix-searchlight-alarm shipped holds through the real path. The test
   (`the_walked_beam_entry_raises_the_alarm_channel`) is now the regression guard: it asserts the
   arm → ch 55 order and ≥3 guards in attack states afterwards. It fails on pre-045l trees (the
   heads never light), which is the user's original symptom.
   - The audible/state feedback the channel drives is also ported: `DroneFunc_CheckAlarmRaised`
     0x1453a8 streams `Music_Event(2,1)` while ch 55 counts [M drone.rs 5054, music.rs 306] and
     `UpdateMFX_01CastleB` flips the music to combat [M; unit-tested]. Nothing on 07000006 writes
     ch 0x61 (97, the mission's "Alarm Raised" fail hook): the whole NIS scan of the level's scripts
     found channel writes only to 64/63/62/61(the light boot), 83, 2 — so no mission fail is
     expected when the courtyard catches you, in the game either [D nf-extract nis].
2. **The spot disc's orientation fixed** (the user's "a spotlight will cover a piece of ground and
   it will flip on its side"). The game's hit branch [M 0x1cb900, disasm 0x1cbe78–0x1cbea8] turns
   the hit normal into the spot's OBJECT ANGLES — pitch(+0x50) = π/2 − ATAN2_APPROX(n.y, |n.xz|),
   yaw(+0x54) = ATAN2_APPROX(n.x, n.z), roll(+0x58) = 0 — and the framework rebuilds obj+0x90 with
   `View_RotTransMatrix` (Control_GetObjMatrix 0x134210 → 0x1e5168; the spot's +0xFA bits 0x20/0x4
   leave the RotTrans path live). The port's old reading used atan2(n.z, |n.xz|) in the PITCH slot
   of the PLACEMENT-Euler `object_matrix` with the yaw in the ROLL slot: on flat ground it stayed
   flat (why the entrance pool mostly looked right), but on wall/slope/Bond-sphere normals the
   disc's plane folded onto its edge or collapsed a row to zero — the flip. The fix builds the
   game's angles and uses `aivolume::matrix` (the verified 0x1e5168 port). Unit test
   `the_spot_disc_faces_the_hit_normal`: the disc mesh lies in its local XZ plane (bbox y = 0
   [D 'Searchlight1']), so matrix row 1 must equal the hit normal — asserted for ground/wall/slope
   normals, with the old reading pinned as the sister (it misses (−1,0,0) by > 0.5).
3. **The guards' first shot: the entry-fire machinery is the game's own, plus the missing hold.**
   Reading the handlers settles the premise: the game's AimStandFire entry fires immediately when
   Bond was seen within 16 ticks [M 0x166328:59-61] and AimCrouchFire fires on entry
   unconditionally [M 0x167060: case 1 `CallAnim(0,5,…)` + `DroneWeap_Fire(1)`] — the port matches
   both. The measured game chains (first hit +70/+128 after the alertness jump) carry their time in
   the combat-move choreography BEFORE the fire state (AIM_STAND 31 / STEP_AIM 116 frames [L
   06-firstsight.trace]) and in BurstDelay [M 0x179ba8/0x179d40], which the port shares. What the
   port lacked: the game's AimStandFire entry DEFERS the first bullet while the guard is still
   playing the hit reaction — `drone+0xda == 0x13` (danim 19; Drone_AnimInfo[0x13] +8 = anim state
   0x4c) clears `+0x3b` and arms timer C for FRAME_RATE_INT ticks, the bullet riding the TIMER_C
   fire [M 0x166328:48-52]. A guard shot mid-approach fired straight out of the stagger in the
   port. Added: the deferral (one guarded branch + `timer_c = Some(frame + 60)`; drone.rs is a
   shared file — the change is the one additive block cited above).
   - The rest of the user's "fires before it lowers at me" is the rendered aim-raise, not the
     timing: the port's guard chain on the firstsight spot (logged) was alert jump → COMBAT 0x58 →
     AIM_CROUCH_FIRE 0x70 (+1 tick) → first shot +51 — every step matches the game's handlers
     [M 0x173280 / 0x167060], and the crouch-vs-step pick is the game's own roll (dist ≥ 6 +
     `Rand_Rand(2)` [M 0x173280]). Whether the gun POSE visibly lowers/raises before the muzzle
     flash is a render question → live check 3.

## Mesh evidence for the render deltas (the main agent's slab/brightness/blob report)

Per-batch GS state and vertex data dumped from 07000006.bin [D] (parts 0-19, one probe run during
the session; the numbers below are the disc's, not the port's):

| part (kind) | object | batches | ALPHA_1 | blend | zwrite | vertex rgba |
|---|---|---|---|---|---|---|
| head (1) / mount (0) | OBJECT_SearchLight1 / SLightMount | 8 | 0x44 | normal (Cs−Cd)·As+Cd | yes | [19,24,30,128] body, [160,202,255,128] lamp face, [128,128,128,128] lens |
| beam (3) | OBJECT_SearchLightbeam | 1 (4 verts = one quad) | 0x7f00000048 | additive Cs·As+Cd | no | gradient [0,0,0,0] → [128,128,128,128], tex 169 |
| spot (2) | Searchlight1 | 1 (4 verts = one quad) | 0x7f00000048 | additive | no | [50,50,55,128], tex 172 |
| corona (4) | SL_Corona | mixed | 0x68/0x72 | per batch | — | [255,255,255,93/128] + black |

The port's vertex colours, per-part colour slots (fix-searchlight-remainder) and per-batch Draw
decode all carry these through the skin shader (verified in code: `vs.push` keeps rgba; `col_sl`
slots are 1.0 for non-coronas; blend 1 = SrcAlpha·One; the 255/128 gain makes byte 128 = 1.0). So
the pool/beam brightness and the head visibility on the CURRENT tree should read very differently
from the playtest build (which predated fix-searchlight-remainder's corona alpha). The remaining
candidates for "much brighter" are environmental: the spot's own psiLight term on the additive
quads (the shader's point-light add runs on them in the port; the game's VU1 program lights the
same quads — whether the beam/pool quads take their own light is check 2 below), and the fog.

## How it was verified

- `crates/nf-view/tests/fix_searchlights_guards_live.rs` (NF_ISO, the real load + `step_bond`
  path), both green on the fixed tree:
  - `the_walked_beam_entry_raises_the_alarm_channel` — the full walk, then the arm/ch-55/guards
    assertions above. Fail-first: on a pre-045l tree the heads stay state 3 and the ch-55 expect
    panics (the boot's channels never ran).
  - `a_guard_shot_mid_approach_holds_his_first_shot_for_the_impact_danim` — staged via
    `next_state` (the machine's own ENTER) + `anim.danim = 0x13`: the first shot waits ≥ 59 ticks
    (timer C's floor). Pre-fix the entry fired immediately (the roll distribution lands the first
    bullet inside 15-60 ticks, so the assert fails for ~90 % of rolls — it was observed failing
    pre-fix in the equivalent poke staging).
- `crates/nf-game/src/searchlight.rs::the_spot_disc_faces_the_hit_normal` + the sister (old reading
  pinned); nf-game lib 141/141 green after the change.
- The suite: run over the final tree (below).

## Shared files touched (small, additive, listed)

- `crates/nf-game/src/drone.rs` — the AIM_STAND_FIRE entry's impact-danim deferral (one guarded
  branch, [M 0x166328:48-52]).
- `crates/nf-game/src/searchlight.rs` and the new test file are this task's named files; the
  searchlight change replaces the spot-matrix two-liner with the game's angles + builder.

## What's left / live checks for the main agent

1. **Render the courtyard on the fixed build** and compare against the game (the task's PCSX2
   spots): `--bond 7.21,42.07,-40.73,16.601,0.412` (beam head-on at the wall — expect the wall spot
   disc standing VERTICAL on the wall now, the beam quad additive; "no slab" in the game means the
   beam quad should read as a soft fan, not a solid) and `--bond 5.24,42.07,-29.30,18.498` (the
   ground pool: expect a dim additive quad ~0.4 grey over the snow, not a bright slab). If the
   pool/beam still render brighter than the game, check 2 is the suspect.
2. **Does the game light the beam/pool quads with their own psiLight?** The port's skin shader adds
   the point-light term on every vertex with a normal — including the additive quads (whose normals
   face the player and whose verts sit near the light node). In PCSX2, poke the spot's light
   intensity (or move the light node) and screenshot: if the game's pool/beam brightness does not
   change, the port must skip the light term for the additive searchlight quads (a one-line gate in
   the shader path or the palette).
3. **The aim-raise pose** at the firstsight spot: `--bond 83.58,35.13,33.21` (walk in from the
   truck end): when a guard reaches AIM_CROUCH_FIRE/AIM_STAND_FIRE, does the gun VISUALLY raise
   before the muzzle flash? The game's own entry fire lands 12 frames into the fire state [L
   06-firstsight.trace], so the gun must already look aimed by then — if the port's guard fires
   from the patrol carry, the anim call chain (CallAnim 10/5) needs a render-side look, not an AI
   change.
4. **The walked alarm in the real game** (the beat question 045l left open): walk the truck route
   into the courtyard beam and confirm the alarm feedback moment (music flip + guards) matches the
   port's +109 ticks after beam entry.
5. The spot disc on Bond's sphere: when the beam tracks Bond, the spot lands on his pick sphere
   with a radial normal — the disc stands upright on him in the game; the port now reproduces that
   geometry (the fix) — worth one screenshot to confirm it reads right.

## Suite count

Full suite on the final tree (this branch rebased onto origin/master e169958), slim ISO, fast
runner (`tools/test/suite.py`): **955 passed, 3 failed, 29 ignored (22 min)** — the 3 failures are
the allowed `disc_golden` trio, no TIMEOUT/CRASH, zero failures outside the permitted set.
