# fix-cutscene-brightness — the 02 intro's cut-scene actors lit near-white

Task 102g. The playtest (results 1013-1825, the 07000002 intro): "the models in the game during
the cutscene are extremely bright" — the port's Bond and the man at the computer render near-white
(the screenshot at `--bond -82.30,5.30,32.05`, frame 170) where PCSX2's 02 intro lights them
darkly to match the room.

## Root cause, with the game's evidence

The port ramps every character's ambient (`Sim::ramp_char_ambient` — the port of
`Lights_CalcAmbientLight` 0x1cb178 + `control_link_object_to_cel` 0x1346b8's snap) from a per-
character position. The intro's cast are script characters (`char_script` Some — the level's NIS
players), and their position came from `Sim::characters_script_pos`, which broke twice:

1. **The strict sequence filter.** `characters_script_pos` filtered candidate sequences with
   `nf_formats::anim::fits(seq, skeleton)` — the skeleton-id match. The 02 intro's actors play
   shared human tracks whose record byte is skeleton 0 on skeleton-N skins (char[79]: seq
   0x04000290, record skeleton 0; char[80]: 0x04000291) — every candidate was rejected, the
   position was `None`, the ramp's target was `None`, and `obj+0x100..0x102` stayed the
   0xff,ff,ff sentinel of `control_init_object` 0x134630: full-bright actors. The relaxed filter
   (the skeleton-id match dropped, the channel-count guard kept) already exists in
   `nf_game::anim::script_pose` — its comment records the same strictness bug on 07000004's
   seated target. [M] the game never filters by the record byte here: `Lights_CalcAmbientLight`
   0x1cb178 reads only `obj+0x20`'s cel ambient bytes — the link decides, not an anim fit.
2. **The missing script base matrix.** With the filter relaxed the position resolved to the raw
   root translation — near the world origin ([0.0, 0.038, 0.958] at frame 200), where `ambient_at`
   is `None` (no cel contains the origin). The game's anim objects ride the script's scratch
   matrix: [M] `Script_AnimStart` 0x1d8f40 creates the object at
   `control_create_object(0, 0x2d6200, 0x2d6200, 0)` (VECTOR 0x2d6200 = the script's base matrix —
   the port's `Player::live_matrix = mul(stream_pose, script.matrix)`), then `AnimGetFirstTrans`
   composes the root into obj+0x30, and [M] the same function ends with
   `if (obj+0x20 == 0) build_LinkToRoom(obj, 0, glb_world)` — the cel link by position at
   creation. The sequence roots are local offsets (char[79]'s seq: root@1 [0,0,0], root@30
   [0.90, 0, 0.10], …) — the base matrix carries them into the cut-scene set.

## The fix

`crates/nf-view/src/scene.rs`:

- `characters_script_pos` adopts `script_pose`'s relaxed sequence filter (via
  `nf_game::anim::script_pose` itself) and composes the script's scratch base
  (`Sim::script_base_matrix(src.0)` — the placed NIS player's `live_matrix`) with the root
  translation.
- `Sim::script_base_matrix(nis)` — the new lookup over `drones.scripts.players`.
- The render's script-character object matrix (frame_state's script branch) gets the same base
  (`unwrap_or(c.matrix)` keeps the old matrix when no placed player runs the hash) — the intro's
  actors draw inside the cut-scene set like the game's, not at the origin.

With both links fixed the ramp snaps the actors to the bunker cel's ambient on the first link —
char[79] `obj+0x100..0x102` = **[20, 30, 20]** (the bunker's ambient at Bond's intro spot), the
draw multiplier the ambient bytes over 255 — dark, like PCSX2's. char[80] same.

## Reproduce before fix (shown failing)

`crates/nf-view/tests/fix_cutscene_brightness_live.rs` (opt-in NF_ISO), through the real path
(`nf_view::load` + `Sim::step_bond`, 200 ticks into the intro — the playtest's frame-~170 moment):

- `the_02_intro_actors_take_the_rooms_ambient_not_full_bright` — the live script actors (char[79],
  char[80]) must carry the bunker's ambient and a sub-0.5 draw multiplier. Shown failing on the
  unfixed tree: `assertion left == right failed: intro actor char[79] carries the room's ambient
  … (got [255, 255, 255])` (and, before that, the strict filter's `pos None` panic). Green
  post-fix: `char[79] amb [20, 30, 20] pos Some([-27.06, 1.45, 33.46])` — the actor stands in the
  bunker set beside Bond ([-27.17, 1.05, 29.80]).
- Sisters: `the_drone_characters_ambient_ramp_is_untouched` (the drone-backed characters keep
  their own cel's ramp), `the_view_model_characters_keep_bonds_ambient` (the first-person weapon
  models keep riding Bond's position branch).

Regression set on the fixed tree, all green: fix_cutscenes_live 5, fix_cutscenes2_live 4,
fix_objects_live 5 (the truck's script characters — the same base composition), fix_objects2_live
6, facial2_live 2, facial3_live 3.

## Suite

Full `tools/test/suite.py` on the final merged tree (commit 03e9048): see the total line below
(slim ISO — the 3 `disc_golden` tests and `pss_movies_live` are the allowed failures).

- This tree (commit 03e9048): **975 passed, 4 failed, 31 ignored (32 min)**. The 4: the 3
  allowed `disc_golden` tests (slim ISO) and `weapon_matrix_live`'s 600 s timeout on
  `matrix_07000015` — the regression the fix-08-collision report already lists as its what's-left
  (that task's per-cel movement pass hangs that level's matrix walk; this task touches only the
  ambient ramp and the script-character pose/position — no movement code). No new failures.

## Files

- `crates/nf-view/src/scene.rs` — a task file: `characters_script_pos` (the relaxed filter + the
  base composition), `Sim::script_base_matrix` (new), the render's script-branch object matrix.
- `crates/nf-game/src/nis.rs` — read-only for this task (the Obj::Anim/Script structures the fix
  rides); no change needed.
- `crates/nf-view/tests/fix_cutscene_brightness_live.rs` — the new test file.

## Live checks for the main agent

PCSX2, 07000002's intro:

1. Spot: let the intro play to the computer-desk scene (frame ~170). Expected: Bond and the man
   at the computer are lit by the room's dim ambient — no near-white models. Pause on the same
   beat in nf-view (post-merge) and compare: the actors' brightness should now sit with the room.
2. gdbwatch (optional): watch a byte at the intro actor's `obj+0x100` during the intro — expect
   the value to snap from 0xff to the bunker cel's ambient byte (0x14 for the R channel per the
   port's parse [L candidate: the cel's params]) within a tick of the actor's creation, not stay
   0xff.
3. nf-view render spots (`--bond x,y,z,yaw,pitch`):
   - `-82.30,5.30,32.05,1.57,0` frame ~170 — the user's screenshot beat: the actors dark in the
     bunker set, no near-white models.
   - `-27.2,1.1,29.8,3.14,0` — beside the intro's Bond: the two script actors draw in the set
     (the base-matrix composition) — not at the world origin.
   - The 07000005 truck scene (the truck's script characters use the same composed base): the
     truck and its driver draw where they did before (the base composition must not move them).

## What's left

- The intro actors' render position rode `c.matrix` (identity) before this task — they drew at
  the world origin; the base composition now places them in the set. If the main agent's render
  shows any cut-scene actor at a wrong spot (e.g. the 05 truck), the base-matrix source
  (`Player::live_matrix`) is the knob to check.
- Chars 77/78 on the 02 intro (the second NIS player's actors) have no live `Obj::Anim` at frame
  200 — their streams start later in the intro; their ambient snaps with the same fix when their
  animation starts (the position resolution no longer depends on the strict filter).
