# fix-cutscene-brightness2 — the held brightness fix (102g) makes the intro's cut-scene actors vanish

Task 102g-followup. 102g (`origin/s/fix-cutscene-brightness` 0cbfbf8, held) fixed the "the models in
the game during the cutscene are extremely bright" playtest bug but the main agent's render of
master + branch at the user's spot (`nf-view <iso> 07000002 --bond -82.30,5.30,32.05,1.570,-0.100
--frame 170`) shows the intro room with **NO characters at all** (master alone draws Bond in the
doorway and the bald man by the computer desk — too bright). So the lighting change zeroed or culled
the actors.

## Root cause, with the evidence

A live probe (the reproduction test, run on the held tree) pinned both halves. At frame 170 of the
07000002 intro: Bond [-27.17, 1.05, 29.80]; the intro NIS player's `live_matrix` translation
**[-27.25, 1.41, 34.39] == the live `Obj::Camera`'s translation** (obj[0]).

1. **The vanish — the render object matrix.** The held fix substituted
   `s.script_base_matrix(src.0).unwrap_or(c.matrix)` (the NIS player's `live_matrix`) for the script
   actors' object matrix in `frame_state`'s script branch. But `Player::live_matrix` is **stream 0's
   object ∘ the script matrix** — `Script_Interp` 0x1d9a08:106-133 keys the SP object's `obj+0x90`
   from stream 0 every frame [M nis.rs] — and the intro's stream 0 is the CAMERA stream: the probe
   shows obj[0] Camera trans == player[1] live trans [L]. The actors were drawn inside the camera →
   everything clipped → the empty room. The premise ("VECTOR 0x2d6200 is the script's scratch base
   matrix") was wrong: **0x2d6200 is the origin vector** [L ACTION.ELF, recorded in nis.rs's
   `Obj::Light` doc — `Light_Create` uses it as the origin], so `Script_AnimStart` 0x1d8f40's
   `control_create_object(0, 0x2d6200, 0x2d6200, 0)` creates the anim object **at the origin** — the
   object matrix is identity and the pose's root carries the actor in world space [M `Obj::Anim` doc
   "the object sits at the origin and the sequence's root carries it"; `AnimSeqTick` 0x1c28d0 takes
   the root motion as the object's movement]. That is the render-verified model the port already
   ships (the wine truck, the pack cut-scenes, and master's own render of this very intro, which
   places both actors correctly).
2. **The ramp sample — the wrong window and the wrong base.** The held `characters_script_pos` used
   `script_pose`, whose `script_frame` takes the **LAST covering window** — on the intro that is the
   zero-root overlay track (char[79]: windows (1,1541,0x04000296)/(1,1540,0x04000224); the last
   one's root reads [0,0,0] at frame 170 [L probe]) — and then multiplied that near-origin root by
   the camera matrix, landing the ambient sample in the camera's cel. The bytes were right by luck
   (camera and actors share the bunker cel); the sample point was meaningless. The ramp must sample
   where the actor is DRAWN: the render path's own resolution, `script_pose_together` — the widest
   started body window — whose root IS the actor's world position (char[79] [-25.51, 1.13, 30.14],
   char[80] [-27.18, 1.13, 30.99]; both beside Bond [L probe]).

## The fix (`crates/nf-view/src/scene.rs`)

- `frame_state`'s script branch draws with the character's own matrix (`c.matrix`, identity for
  script actors) again — the render-verified origin model, evidence above.
- `characters_script_pos` resolves the position the renderer draws: `nf_game::anim::script_pose_together`
  (the same function the render path poses with) and returns its root translation in world space —
  no base composition. `ambient_at(root)` = the bunker cel **[20, 30, 20]** — the same dark ambient
  the held test pins, now sampled where the actor actually stands.
- `Sim::script_base_matrix` removed (it existed only for the wrong reading; no other callers).

`crates/nf-game/src/nis.rs` untouched. No shared-file hooks.

## Reproduce before fix (shown failing)

`crates/nf-view/tests/fix_cutscene_brightness2_live.rs` (opt-in NF_ISO), through the real path
(`nf_view::load` + `Sim::step_bond`, 170 ticks — the user's frame):

- `the_02_intro_actors_draw_in_the_set_not_at_the_camera` — the two live script actors must be
  posed, unhidden, carry their own object matrix (identity — the held tree's camera matrix fails
  this), draw at the anim's world root beside Bond (< 3 units), be inside the intro NIS camera's
  frustum, and carry a non-zero, non-white (dark) draw multiplier. Shown failing on the held tree
  (the object-matrix assert: got the camera). Green on the fix: char[79] root [-25.51, 1.13, 30.14]
  screen (-0.18,-0.06) mul [0.075, 0.114, 0.075]; char[80] root [-27.18, 1.13, 30.99] screen
  (0.19,-0.08) mul [0.075, 0.114, 0.075].
- `the_ambient_ramp_samples_the_drawn_position_not_the_camera` — sister: the ramp's sample point is
  the drawn root (never the camera — fails on the held tree at [-27.25, 1.41, 34.39]; never None —
  fails on master, the strict filter), inside a cel, dark. Green on the fix: both samples resolve
  with amb [20, 30, 20].

The held branch's own `fix_cutscene_brightness_live` stays green (3/3) — the ambient bytes it pins
are unchanged; only the sample point became the true drawn position.

## Regression set, all green on the fixed tree

fix_cutscene_brightness_live 3, render2_live 8 (the 07000001 cut-scene's [L] (40,30,26) courtyard
ambient pin), fix_objects_live 6 + fix_objects2_live 5 (the 05 truck's script characters — the same
script-branch draw), fix_cutscenes_live 4, fix_cutscenes2_live 5, facial2_live 2, facial3_live 3.

## Suite

Full `tools/test/suite.py` on the final tree: see the total line below (slim ISO — the 3
`disc_golden` tests and `pss_movies_live` are the allowed failures).

- This tree: **(filled from suite_cb2.out)**.

## Render spots for the main agent (nf-view, `--bond x,y,z,yaw,pitch`)

- `07000002 -82.30,5.30,32.05,1.570,-0.100 --frame 170` — the user's beat: Bond (blue suit, left, in
  the doorway) and the bald man (centre, by the computer desk) DRAWN AGAIN, lit dark to match the
  room (multiplier ≈ [0.075, 0.114, 0.075]) — neither near-white (master's bug) nor missing (102g's
  bug).
- `07000002 -27.2,1.1,29.8,3.14,0` — beside the intro's Bond: both actors in the set, not at the
  world origin.
- `07000005` truck scene — the truck and its driver draw where master drew them (the script-branch
  revert must not move them; fix_objects_live/fix_objects2_live cover the sim side).

## Live checks for the main agent

1. PCSX2 07000002 intro, frame ~170: Bond and the man at the desk lit dark to match the room
   (neither near-white nor missing). Compare with the nf-view render above.
2. gdbwatch (optional): during the intro, watch the intro actor's `obj+0x30` (the object matrix
   translation) — expect it at the ORIGIN (identity matrix) the whole scene, never the camera's
   [-27.25, 1.41, 34.39] [M `Script_AnimStart` 0x1d8f40 creates at the origin vector 0x2d6200
   [L ACTION.ELF]; `AnimSeqTick` 0x1c28d0 moves the object by the root motion].
3. gdbwatch (optional): the same object's `obj+0x100..0x102` — expect the bunker cel's ambient bytes
   ([20, 30, 20] per the port's parse), snapped on the first link, ramping with `ClrStep` as the
   actor crosses cels.

## What's left

- Nothing known for this bug. Note for 102g's held report: its "the sequence roots are LOCAL to the
  NIS placement" premise is refuted — 0x2d6200 is the origin vector, the object is created at the
  origin, and the roots are world space (the render-verified model; the probe's roots sit beside
  Bond). Its ramp test still passes: the ambient bytes it pins ([20,30,20]) are the same, now
  sampled at the true drawn position.
- The `hidden`/no-pose edge (a script character whose `script_pose_together` returns None while
  `script_anim` is Some) still skips the draw — unchanged by this task, pre-existing behaviour.
