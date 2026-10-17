# level-0c-interactables (task 712a) — 0700000c, the power station, part 1

Branch `s/level-0c-interactables`, cut from origin/master 0c455572. Test-only change: one new
test binary (`crates/nf-view/tests/interact_0700000c_live.rs`, 9 tests) + the census
(`docs/levels/0700000c.md` §7). No `nf-game`/`nf-formats`/`nf-audio` source touched.

## What's done

1. **The census** (`docs/levels/0700000c.md` §7): all 574 main-map placements from
   `nf-extract placements-all` [D], every kind mapped to its `parsemap_create_dynamic_objects`
   creator [M 0x1d04f0, every case read], with params, channel wiring and port status per kind.
   Headline wiring [D]: the 21 Touch + 7 TouchOnce plates (objective 1 = #506/#274 → ch 0x1b;
   the hint plates #455/#459 lock ch 0x1c; the Bond Moments → ch 0x65/0x06), the 3 skylight
   Trigger_Create plates (mask 0x1b) → ch 2, the secret-door AND #560 (ch 2 AND ch 3 — the
   drone half is #558's mask-4 volume) → ch 8 → SS `Door_secret1` #544, the hinge burns →
   chs 14–17 → AND #505 → ch 18 → SS #138 (the skylight hatch), `VentLatch` #182 → ch 84 →
   SSs #180/#184, the 5 sound triggers each gated by a placement-written channel (0x55/0x43/
   0x08/0x66/0x65), the 2 music triggers (#95 gate ch 5 → event 45, #502 gate ch 0x24 →
   event 13), 5 grapple pulleys, 4 wires (3 hand-over-hand + the Zipline #512, wire_type 2),
   2 ladders, 2 doors (door2turbine + the off-map `lockeddoor` #555), 15 pickups (6 heavy
   armours, 3 sniper 300AP, the crossbow, the 007 logos, the briefcase), 12 oil-barrel
   destroyables, 2 CamSubjects (ch 0x1c), the use-activated LoadLevel gate #556 (gated on the
   photograph's ch 0x1c). **Absent kinds** [D]: no switches/buttons, no lifts, no safes,
   keypads, monitors, terminals, searchlights, turrets, breakable glass, vehicles, shoji.
2. **One live test per KIND** (`interact_0700000c_live`, 9/9 green through the real input path,
   each with a sister that fails without the operation):
   - `the_static_world_kinds_all_register` — the [D] counts (2 doors, 2 main-map ladders, 4
     wires, 5 grapple points, 5 locks, 8 SSs, 15 pickups, 12 destroys, 2 subjects, 15 icons,
     5 sound triggers, 2 music triggers, 5 dynobjs, 4 thirdcams) and the param-less fusebox
     pin.
   - `the_use_key_opens_door2turbine_and_the_sister_keeps_it_shut` — E opens [M 0x1a5170 case
     0x1C]; the sister never presses.
   - `walking_the_mission_trigger_fires_channel_43_once` — the walk across TouchOnce #551
     fires ch 0x43 [D triggers dump; M Trigger_Update 0x1b1a78 case 1]; the sister teleported
     to the yard never sees it.
   - `the_armour_pickup_grants_50_on_the_walk_over_and_the_sister_refuses` — the walk-over
     grants 50.0 [M Pickup_Handler 0x18e650 case 3; D p2=50]; the sister stops short.
   - `a_bullet_explodes_the_oil_barrel_and_the_far_barrel_stays_whole` — the shot barrel dies
     [M Destroy_CollisionHandler 0x12bd48]; the warehouse barrel stands.
   - `the_paramless_fuseboxes_take_no_use_and_the_door_takes_it` — the twelve kind-0x31s are
     param-less `object_oilbarrel01_destroyed` twins [D]: `FuseBox_Create` needs the script
     hash in param 0 [M 0x1b0448], so nothing creates, no wire group registers [M 0x1b0970],
     E writes no channel [M 0x1a5170 has no class-0x3C arm]. The control arm is the door test.
     The fusebox KIND's live behaviour (the laser wire-cut) is pinned on 07000008/14 by
     `fix_fusebox_cut_live` (green).
   - `the_zipline_grabs_on_the_action_key_and_carries_bond` — the roof spot arms the wire
     icon's context 3 [D #509 p0=3; M 0x1916b0:126-147], the action edge grabs the zipline,
     strafe carries Bond along the segment [M Player_Zipline 0x19806c]; the sister never
     presses.
   - `the_q_claw_grapple_latches_the_pulley_and_reels` — the Q-Claw is in the 0c loadout
     [M 0x1a7ef8 case 0x700000c], the aimed fire latches `graple_pulley` #500 [M 0x195da0],
     the reel advances pull [M 0x192860:79-104]; the sister doesn't fire.
   - `the_music_trigger_fires_its_event_while_bond_is_in_its_zone` — standing in the ch-0x24
     volumes (#501/#503/#504 [D]) re-arms trigger #502's 30-tick countdown [M 0x1b31a0:15-27],
     which drives `Music_Event(13, 2)` [M 0x1b31a0:23]; the sister out of the zone keeps it
     drained. (The event board itself is a per-frame mailbox the music updater consumes the
     same pass [M music.rs], so the pin is the trigger's own game state.)
   The kinds already pinned green on this level elsewhere were run again rather than
   duplicated: the SS sweep + the Lock burns + the AND mux (`fix_level_0c_live` 4/4), the
   Touch objective plate + the CamSubject photograph + the LoadLevel gate
   (`walk_0700000c_live` 4/4), the yard-ladder climb (`walkreal_0700000c_live` 1/1).
3. **Every model draws**: `textures_live::every_texture_resolves_and_every_mesh_is_drawn`
   (nf-game) is green on the final tree — every texture and mesh for every level including
   0700000c resolves, no white-fallback material. The GPU-side `NF_TEX_TRACE=1` sweep is not
   runnable headless (no wgpu adapter in this sandbox — see below).

## How it was verified

Foreground on the final tree: `interact_0700000c_live` 9/9, `fix_level_0c_live` 4/4,
`walk_0700000c_live` 4/4, `walkreal_0700000c_live` 1/1, `textures_live` 1/1. The crates the
change touches: nf-view (tests only). nf-game/nf-formats/nf-audio sources are untouched by the
branch; `textures_live` (nf-game) ran green as the model/texture sweep.

The fast runner (`tools/test/suite.py`, background) did not complete in this sandbox: the
sandbox's OOM killer crashed `load_time_probe` and `weapon_matrix_live` (exit -9) and the
run is still going at report time; its only FAILED lines are `fix_0c_pylon_live` +
`probe_pylon_render` — re-run in the foreground those fail on **`adapter: NotFound …
no_adapter_backends`**: they are wgpu GPU-render probes and this sandbox has no GPU (the same
headless limit every prior report hit). They are master tests; this branch's diff is one new
test file + docs, so they cannot be affected by it.

## What's left

- **The winch SSs' ch-0 self-latch driver** (`SS_Operate` 0x1a9df8's contact half) — 712b
  (`s/level-0c-complete`) has it WIP; unmerged. ch-0 sweeps exist (fix-0c-vent2).
- **The vent-route traversal legs** (the display-case ascent/descent) — documented stand-ins
  in `walkreal_0700000c_live` remain; mapped in `docs/reports/fix-0c-hatch.md`.
- **The zipline's walked approach**: the test reaches the turbine roof by the documented
  teleport stand-in; the real walk from the hall top is unverified (the roof approach legs are
  not in any walkreal route yet).
- **Live checks for the main agent**: (1) on a fresh-boot PCSX2 run of 0700000c, does the
  spawn fire the mission trigger #551 immediately (the port fires it from the spawn capsule)?
  gdbwatch `switch_channels` 0x26f... ch 0x43 during the first second. (2) Do the ch-0x24
  volumes' music event 13 and the #502 countdown behave on real hardware while crossing the
  south yard walkways? (3) The #95 music trigger's gate ch 5 has no writer in the trigger dump
  — a gdbwatch on ch 5 during a full play would name its writer (suspect: a level-script
  event).
- **The sniper spots**: the three 300AP pickups (#477 yard, #520/#521 the tower pair) and the
  #478 Bond Moment are placed [D]; the tower's walked approach is not in a walkreal route.

## Usage

not visible to me — this environment exposes no token, credit, context-window or cost figures
for the session.
