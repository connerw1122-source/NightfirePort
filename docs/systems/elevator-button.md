# The elevator button (07000009) — the class-0 entity use path

Task elevator-buttons-09. The user: "09 level elevator buttons dont work". Root cause and fix, all
addresses from the ACTION.ELF decompile in `work/decomp/ACTION.ELF/`.

## The symptom in the port

`UseObjects::player_activate` (crates/nf-game/src/useobj.rs) E-activated a mask-4 ScriptPlayer only
at **its placement matrix** — and `SCRIPTPLAYER_Elevators` (#70 on 07000009) sits at the world origin
[D placements dump: `t [0.0, 0.0, -0.0]`], its art being baked absolute in the model. Nothing is ever
near (0,0,0), so the use never landed, ch 0x39 (57) never fired, objective 2 (ch 0x46 = AND(ch 0x39)
[D mux #274]) stayed incomplete and the two LIFTDOORS SS (waiting on ch 57 [D params (3,57)]) never
opened. The old test stood in with `set_channel(0x39)` — the stand-in is what this task replaces.

## The game's real chain [M unless noted]

1. **The SP's use target is a script ENTITY, not the SP object.** `Player_Activate` 0x1a5170's
   sphere pick (radius 1.0 at head+view, `Collide_SphereIntersect`) dispatches to `SP_Activate` only
   for a hit object with `obj+0xE4 != 0 && obj+0xFF == 0` (class 0) — 0x1a5170:121-122.
   `obj+0xE4` is the owner-SP handle `Script_EntityStart` 0x1d8d78:108-110 stores on every
   script-created entity (`*(u32*)(obj+0xe4) = *(SCRIPTINFO+0x80)`; docs/fields/script-player.md
   "The obj+0xE4 handle"). The class-0x1B SP object itself is inert to the use pick (no case in the
   0x1a5170 switch; obj+0xE4 = 0 on placement SPs [L ee_start_05.bin ×8]).
2. **#70's entity exists from the level load.** Placement #70 (mode 64 = preload-then-prime, mask 4,
   out ch 57, stay 1, script 0x0600059B [D params (0,100664731),(2,64),(4,1),(5,4),(6,57)]) primes at
   create: `SP_LoadScript` 0x1dbc08's mode-64 arm is play+update+stop plus one more `Script_Update`
   — two ticks, `SCRIPTINFO+0xA0 = 2.0` at level start [L ee_start_05.bin, the EndLevel SP]. Script
   0x0600059B stream 0's first op pair is `Wait(0), Entity{hash 0x020005EA, obj_type 0x1F}` [D nis
   dump] — op 7 → `Script_EntityStart` runs inside the preload tick, so **entity A exists (class 0,
   obj+0xE4 = SP#70) before any input**. `Script_EntityStart` then calls
   `Control_BuildWorldSph(entity)` (0x1d8d78:113) — the entity's world sphere comes from its model's
   bounds, which for the "SCRIPT_" art are baked in world space (placement #70's own gfx bounds
   [−18.382, −368.534, 24.832]..[−17.775, −367.921, 25.198] [D placements dump] — a 0.6×0.6×0.37
   panel in the lobby, by the BodyArmourHeavy at (−22.6, −368.5, 24.9) [D]).
3. **The activation.** Bond faces the panel and presses E: the 1.0-radius pick meets entity A's
   sphere → `SP_Activate(SP#70)` 0x1dc338: mask & 4 → no out-channel gate (mask bit 1 clear) →
   `+0x2C` latch = 1 and the current SCRIPTINFO restarts (`+0x9C = 0`, 0x1dc338:72-86).
   `SP_Update` 0x1dbda8 state 1 consumes the latch: state 2, countdown 5, `Script_Play`, and the
   out-channel write `switch_channels[57] = 1` (0x1dbda8:107-123, 221-229).
4. **What ch 0x39 drives** [D placements/triggers, docs/levels/07000009.md]:
   - AND mux #274 → ch 0x46 → objective 2 "Get to the lobby and activate the main elevator system".
   - LIFTDOORS_LEFT/RIGHT_ENDLEVELDOORS (#3/#2, 0xE2 SS, params (0,64),(1,1),(2,1),(3,57),(4,65535),
     (6,1)): state 0x40 waits for `SS_Operate()` = 1 which the channel half provides once
     `switch_channels[57]` is on (`SS_Operate` 0x1a9df8:47-51; mask bit 0 set → no write-back), then
     state 1 plays the open **spline** (param 6 = 1) and the one-shot selector (param 1 = 1) leaves
     them disarmed at state 0. Sound param 4 = 65535 → no door sound (the level doc's "with sound"
     was wrong; corrected).
   - Script t=16 (0x10): entity B (gfx 0x020005EB) + `Sound 0x4A1` (stream 2) — the activation FX;
     t=25 `EntityEnd` + script end; stay=1 → state 5 → the SP deletes itself (0x1dbda8:142-159,
     190-192). One-shot button.
5. **The exit**: the opened shaft exposes TOUCH_ONCE #67 (out ch 0x32, box
   [−5.01, −369.43, 10.33]..[−2.92, −368.04, 12.49] [D]); LOAD_LEVEL #63 (next 0x0700000a, gate ch
   0x32, key gate 0) fires into 0700000a with objective 2 live-complete [D; M 0x1b2318].

## The port fix

- `nis::Player::entity_use_targets(objects, w)` — the current SCRIPTINFO's kind-1 streams with an
  entity object, each as (world centre, radius) = `World::glist_sphere(hash)` transformed by the
  stream matrix (the same math `Scripts::held_hit_targets` already uses for the bullet pass).
- `useobj::UseObjects::player_activate` takes the world and, per mask-4 SP, tests the use sphere
  against the entity targets; with **no resolvable entity** it falls back to the SP's own model
  sphere (the placement gfx, also the panel here) and then to the placement matrix (the old
  stand-in, kept for SPs whose scripts have created nothing yet — the existing unit tests). The
  entity path is the [M] path; the fallbacks are marked [S] in code.
- The rest of the chain already existed: the SP latch → out-channel write (nis.rs update_player),
  the SS state machine (ss.rs), the touch → load (triggers.rs).

## Evidence base

| claim | tag |
|---|---|
| SP_Activate reached only through a class-0 obj+0xE4 object | [M 0x1a5170:121-122] |
| obj+0xE4 = owner SP, written by Script_EntityStart | [M 0x1d8d78:108-110] |
| SP objects carry obj+0xE4 = 0 | [L ee_start_05.bin, 8/8] |
| mode-64 preload runs two update ticks | [M 0x1dbc08:41-56] [L ee_start_05.bin SI+0xA0 = 2.0] |
| script 0x0600059B ops (Entity 0x020005EA @t0, 0x020005EB @t16, Sound 0x4A1 @t16, EntityEnd @t25) | [D nis dump] |
| #70 params: hash 0x0600059B, mode 64, stay 1, mask 4, out 57 | [D placements] |
| #70 art baked at (−18.1, −368.2, 25.0) | [D placements gfx bounds] |
| state-1 trigger → out = 1 | [M 0x1dbda8:107-123, 221-229] |
| LIFTDOORS SS params + state 0x40 channel gate | [D placements] [M 0x1a9948:80-129, 0x1a9df8:47-51] |
| door "with sound" claim wrong (param 4 = 65535 = none) | [D param map, docs/fields/ss.md +0x14] |
