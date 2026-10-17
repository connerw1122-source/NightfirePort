# level-05-interactables — 07000005 (Mayhew A, the castle approach)

Branch `s/level-05-interactables` (off master d628f01f, merged origin/s/level-04-interactables for
the census tool + the parsemap main-map rule).

## What's done

1. **The census** (docs/levels/07000005.md §7): every main-map placement (363 [D]) by kind with the
   dispatcher `parsemap_create_dynamic_objects` 0x1d04f0's creator per kind, the placements'
   params/channel wiring, the [M] behaviour and the port status. The level has no switches, locks,
   keypads, safes, monitors, SimpleScripts or ripples at all [D]; the interactive kinds are the
   touch family (0xEA ×28 / 0xEB ×9 / 0xEC ×6 / 0xEF ×3), the exits 0xE8 ×2, doors 0xDB ×5,
   pickups 0xF0 ×6, ScriptPlayers 0xD9 ×6, copters 0x2F ×2, fuseboxes 0x31 ×4 (all origin-parked,
   no params → inert [M 0x1b0448]), searchlights 0xDE ×29 (7 heads / 2 psiLights / 7 mounts /
   6 beams / 7 coronas), hurt volumes 0xE4 ×13, spawners 0xF1 ×2, sound triggers 0xF9 ×6, third
   icons 0xFA ×9, grapple points 0x3A ×2, wire 0x3E ×1, ladder 0x22 ×1, creepwalls 0x3F ×2 +
   their DynamicObject blockers 0x3B ×2, third cams 0xE7 ×4, env snow 0xDF ×2 + rainboxes 0x32 ×18,
   plus the ambient kinds. Two census corrections to the old page: **TouchOnce #36 "Mission
   Trigger" IS the ch-0x45 objective-reveal writer** (out 69; the old "no 0xEA out-69 placement"
   note was wrong), and the searchlight parts' roles are param 0 = 1 head / 2 psiLight (the −10000
   kind-2 light record) / 0 mount / 3 beam / 4 corona.
2. **The hurt volumes ported** (crates/nf-game/src/hurt.rs, wired into drone.rs's tick): the
   class-0x27 damage volumes [M Hurt_Create 0x17e870; the damage lands through the player's object
   collision — Player_DealWithObjHit 0x194458:95-103 → Player_HandlePain(damage, Hurt_GetType, −1)
   [M]; armour cannot absorb pain types 1..7 [M 0x1902c8:84-104 — the port's pain_generic mirrors
   that switch]. The contact is the trigger-family capsule-vs-mesh (triggers::capsule_hits_tris,
   now shared); the exact BL object-collision pick is the port's approximation [S, the [M] parts
   are the class arm and the numbers]. Live test: a walked fall off the upper west ledge down the
   valley floor dies in the y-8.1 plane #329 (1000 × the 05–08 damage factor [M 0x1d3c44] against
   a ≤ 100 pool); sister stands on the ledge, alive.
3. **The searchlight spot ray fix** (searchlight.rs): the game excludes the head's own cel+object
   from the spot ray [M Collide_PickObj 0x1eae80:57-59]; the port's ray started inside the head's
   own platform geometry and hit at t≈0, so the spot stuck to the head and no head on this level
   could ever arm. The port now skips the ray's path through the head's own gfx box (capped at 4
   units). 07000006's searchlight tests stay green.
4. **One live test per KIND through the real input path** (crates/nf-view/tests/
   interact_07000005_live.rs, 11/11 green, each with a sister): the census creations + the zero
   white-fallback texture sweep, the approach plates → the OR mux, the doors (locked refuse on E —
   lock ch 200 has no disc writer, U-044), the ladder climb, the pickups (armour 50, the kind-6
   bonus token take), the searchlight sweep + the [L]-measured orphan light, the SC3 plate → copter
   #242's gate (and the gate-0 copter flying from the load), the ledge plate → sound trigger 368,
   ch 0x22 → the group-2 spawner, the grapple-point aim, the valley hurt fall.
5. **The model sweep**: the level load reports **0 unresolved** meshes, 3576 materials with **0
   white-fallback** textures (the census test now asserts it), 0 weapons without mesh or datum
   [D the load line]. 12 character meshes held back by spawners (the dev panel shows them) —
   pre-existing, not a miss.
6. Repairs along the way: nf-extract's `ai_sim` still called the old `attach_level(&level)` (the
   sibling branch's signature change) — fixed; 19 nf-game test files did the same and never built —
   fixed with each test's own level id; the census doc's script-hash typos (0x06000046/47 for the
   copter, 0x06000074 for SP #71) corrected — the tests caught both.

## How it was verified

- Every test cites [D] disc parses (nf-extract placements-all / level-summary over
  `07000005.bin`), [M] decompile addresses, or the [L] values the earlier merges pinned. The
  sisters fail without the operation (fresh loads assert the pre-operation state).
- Run before the final push: **nf-game 61/61 test binaries green** (single-threaded), nf-formats
  + nf-audio green (79+11+…), and the nf-view neighbourhood the touched code feeds: interact_
  07000005_live 11/11, walk_07000005_live, walkreal_07000005_live (the truck route), mayhew_
  handover_live, level_carry_live 8, fix_truck_live 2, wire_live, fix_wire2_live, searchlight_
  flip_07000006_live, fix_searchlights_guards_live, fix_searchlight_alarm_live, worldobj_live,
  gadget_grapple_live, copter_patrol_latch_live, fix_ladder_live/2/water, fix_wallhug_prompt/camera,
  fix_fusebox_cut, gadget_carry, ai_traces_live, cover_nodes_live, fix_ai_remainder_live,
  anim_events_live, fix_objects2_live, fix_script_objects_live, world_fx_live, simplescript_live,
  music_triggers_live, plus the walk slices for 01/02/03/04/06. **Not run: the full remaining
  nf-view suite** (≈90 binaries) — the sandbox kills background jobs between tool calls, so
  tools/test/suite.py cannot run here; the foreground budget went to the touched crates. The
  next message should run the remaining chunks (tools/test/suite.py or chunked foreground).

## What's left

- **The walkreal west/cable route** (still #[ignore]d): the anchors predate the ledge's real walk
  heights — the cable-mouth anchor (−20, 34.5, 8) actually stands at y 28.0, and the cable grab
  from there needs the jump-grab the walker does not wire for wires (wire_live's own walk-in grab
  tests are ignored for the same reason). 020-fix-wire's item.
- **The window-ledge plunge catch**: the wall-hug window plates (#311-#313, the ch-105 route) sit
  on a ledge whose walked surface (y ≈ 35.3) is inside plunge shell #6's y band (26.8..35.5) — the
  port's shell mesh catches the ledge walk and fires the death NIS. The game's wall-hug route runs
  the same ledge safely; either the port's ledge floor sits low or the shell mesh is coarse.
- **The #36 reveal plate** (ch 0x45) sits at feet+0.47 — below the port's documented toucher
  stand-in reach (drone.rs's toucher comment); the reveal fires only through the class-5 object
  contact path the stand-in does not model. Revisit with the object touchers.
- The copter #241 flies its script from the load in the port; the main agent should eyeball the
  fly-by against PCSX2 (the visual ride is the script-player machinery).

## Live checks for the main agent

1. 07000005, fresh boot, stand Bond at (60.1, 28.4, 96.4): the ch-0x45 reveal plate under his feet
   (placement #36 at (105.94, 24.41, 108.14) is the writer — but the HUD reveal should already be
   up from the spawn plate; gdbwatch `switch_channels[0x45]` at 0x2bf0f8+0x45 from tick 0 — expect
   1 within the first second if the game's own contact reaches it).
2. The window ledge walk (the wall-hug route past (−35, 37, −28..−37)): does the real game fire
   the plunge (ch 0xBE=190) there? gdbwatch 0x2bf0f8+0xBE during the ledge walk — expected 0 (the
   route is safe); if 1, the port's shell mesh is right and the game's route differs.
3. The copter #241 fly-by at the load: fieldprobe a screenshot at tick ~600 from the cold start —
   the helicopter should be airborne near (95.9, 65.3, −50.2) following script 0x06000046.

## Usage

not visible to me — the environment shows no token, credit or context-window figures for this
session.
