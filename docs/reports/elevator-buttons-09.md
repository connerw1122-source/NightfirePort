# elevator-buttons-09 — the 07000009 elevator buttons and the lift

Task: "09 level elevator buttons dont work" — find every elevator-related interactable on
07000009, make it work through the real input path, prove it with a live walk test.

## What was done

1. **Census** (docs/levels/07000009.md, docs/systems/elevator-button.md): the elevator activation
   is `SCRIPTPLAYER_Elevators` #70 (0xD9, script 0x0600059B, mode 64, stay 1, mask 4, out ch 57)
   whose art is baked absolute at the lobby panel (gfx bounds −18.38..−17.78 × −368.53..−367.92 ×
   24.83..25.20 [D placements dump]); the two LIFTDOORS SS #3/#2 (params (3,57), spline, one-shot,
   no sound [D]); the cab ScriptPlayers #21/#22 (mask 4, out 0 — E writes nothing [M 0x1dbda8:221]);
   TOUCH_ONCE #67 + LOAD_LEVEL #63 (gate 0x32) [D]. The doc's old "open question" section is
   replaced with the solved mechanism and its "with sound" wording corrected (param 4 = 65535 =
   none [D param map]).
2. **The root cause, [M]**: `Player_Activate` 0x1a5170:121-122 reaches `SP_Activate` only on a hit
   object with `obj+0xE4 != 0 && class == 0` — a script ENTITY (`Script_EntityStart` 0x1d8d78:108-110
   stores the owner SP there); the class-0x1B SP body is inert to the pick. The port E-activated
   mask-4 SPs only at their placement matrix — #70's placement is the world origin, so the button
   could never be reached. The mode-64 preload runs the t=0 Entity op (play+update+stop + one more
   update [M 0x1dbc08:41-56]; [L ee_start_05.bin SI+0xA0 = 2.0]), so the panel entity
   (gfx 0x020005EA) exists from level load with its world-baked sphere.
3. **The fix** (crates/nf-game): `nis::Player::entity_use_targets` (the current SCRIPTINFO's live
   entity world spheres via `World::glist_sphere` — the same math `held_hit_targets` uses);
   `useobj::UseObjects::player_activate` gained the world and per-SP tests the entity spheres
   first, then the SP's own model sphere (`Player.model`, the placement hashcode) and only then
   the old placement-matrix stand-in; plus `SP_Activate` 0x1dc338:84-86's second write —
   `SCRIPTINFO+0x9C = 0` — without which 0x0600059B's stream-1 `Event{kind 3, p0 = 0}` hold
   (the port's `delay = −1` [M 0x1d94c0:98-117]) kept the activation FX from ever playing.
4. **The live test** (crates/nf-view/tests/elevator_buttons_09_live.rs, real input path only):
   - `panel_spheres_resolve_at_the_lobby_panel`: #70's model (0x020005E2) and both FX entities
     (0x020005EA/EB) resolve to r 0.434 spheres at the panel [D]; sister — an unused hash does
     not resolve.
   - `walk_operate_the_button_and_ride_to_0700000a` — walks the level start → car park →
     stairwell door #107 (E) → the two-flight switchback → lobby door #97 (E) → the lobby → the
     panel; aims the view at the knee-high panel (pitch down — `Bond::look`'s RotMatrixX is
     positive-up) and presses E; asserts ch 0x39, ch 0x46 (objective 2), the SP playing/deleted,
     entity B created, both door SSs disarmed at state 0 and posed open (matrix moved > 0.3);
     then walks into the cab → ch 0x32 → `level_exit == 0700000a` with objective 2 live-complete
     and no alarm. 4988 ticks, no falls.
   - Sister `without_the_button_the_doors_stay_shut`: the same walk with no press — ch
     0x39/0x46/0x32 all off, #70 still primed (state 1), the doors still resting (state 0x40),
     and pushing at the shaft mouth cannot reach the touch box (the closed SS doors are solid;
     Bond stops at z > 12.9).
   - Unit tests (useobj.rs): the model-sphere fallback replaces the origin matrix and falls back
     to the placement matrix when the model does not resolve; the existing gate tests updated for
     the new signature.

## How it was verified

- The three new tests above through `nf_view::load` + `Sim::step_bond` (the real path; no
  `set_channel` stand-in anywhere in the new tests).
- `cargo test --release -p nf-game --lib useobj` → 6/6.
- Full suite (`tools/test/suite.py`, the branch's tree as of 11:02 — before the fix's code commit,
  i.e. the master baseline for everything but the docs): **2068 passed, 6 failed, 74 ignored
  (55 min)** — the 6: the 3 `disc_golden` tests (allowed on the slim ISO) + `fix_0c_pylon_live` ×2
  and `probe_pylon_render` ×1 (render probes on 0700000c, pre-existing, untouched by this task).
- Final tree (the fix + tests committed): `cargo test --release -- --test-threads=1`
  - libs `-p nf-game -p nf-formats -p nf-audio --lib`: 262 + 79 + 11 passed, 0 failed;
  - nf-view integration: elevator_buttons_09_live (3), simplescript_live (5), abseil_elevator_live
    (3), walk_07000009_live (3), walk_0700000a_live (3), walk_0700000b_live (2),
    fix_script_objects_live (2), solar_panels_live (3), fix_blast_scripts_live (3),
    campaign_flow_live (4), wire_live (2) — all passed;
  - nf-game integration: doors_live, trig_live, fields_live, objective_state, level_live — all
    passed.
  The stale full-suite rerun on the final tree did not fit the sandbox window; the baseline run
  plus the final-tree targeted batch above covers the touched crates and the SP/SS/door consumers.

## What's left

- The door-open ANIMATION and the panel FX rendering are draw-side (the main agent renders; the
  posed matrices and the entity list are already live — render spots: the panel at
  `--bond -17.2,-368.4,25.3` yaw facing −z should show the lobby panel; after the E the two door
  SS matrices sit posed open at the shaft mouth (−4.5..−2.9, z ≈ 13.2)).
- The Monitor camera view and the switch button animations remain presentation-only (pre-existing,
  unchanged).

## Live checks for the main agent

- On a fresh-boot 07000009: walk to the lobby panel (by the BodyArmourHeavy at (−22.6, −368.5,
  24.9)), face the panel and press E — expect the activation FX (sound 0x4A1 at +16 ticks) and
  the lift doors opening; `switch_channels[57]` (0x26fc90+57) should flip to 1 on the press tick.
- gdbwatch on the panel entity object (the class-0 object with obj+0xE4 = SP#70) during the E
  press: `Player_Activate` 0x1a5170:121 reads +0xE4 ≠ 0 → `SP_Activate` 0x1dc338.

## Usage

not visible to me (no token/credit/context figures are shown in this environment).
