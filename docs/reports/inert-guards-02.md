# inert-guards-02 — report

Task 694, branch `s/inert-guards-02`. Playtests 1010-2055 / 1011-1128 (07000002): "there are npcs
that shouldnt be able to be shot and some of the enemies are invincible with no AI"; "one specific
guard … standing next to a civilian that won't move, no AI, doesn't do anything".

## The census (the deliverable the task asked for)

25 drones on 07000002 (`crates/nf-view/tests/inert_guards_02_live.rs` holds the dump): 21 guards
(kind 1 — skins 0x05000036/0x38/0x39/0x3a, weapons 0x0e/0x12/0x1c/0x1e, health 10–20), 3 kind-8
civilians (skin 0x05000034) and 1 kind-9 civilian (skin 0x0500004f, inside the bunker). At start:
all present, enabled, damageable, state 0. After 600 real ticks: the kind-9 civilian runs (state
0x20, moving); every zone-gated guard sits in **WAIT_SWITCH (0x01) with the AI off** — which is
the game's own gate, not a missing handler:

- The switch channel is placement parameter 3 (DCVars+0x134 [M `NDrone2_DefaultInit` 0x14b300 /
  the port's `Drone::new`]). A drone whose channel is off enters WAIT_SWITCH with vision and AI
  off [M the port's WAIT_SWITCH arm; `NDrone2_WaitSwitch`].
- The level's intro cut-scene freezes everyone (`Drone_EnableAll(0)` [M 0x1383e8] — the port's
  `DroneEnable{on:false}`) and the hand-back re-enables everyone EXCEPT WaitSwitch drones [M
  0x1383e8:14] — they stay disabled until their own channel wakes them [M 0x15b960:21-37].
- Each guard's waking channel is set by its zone's Touch trigger ([D] the census dump of all 36
  triggers): the bunker-exit pads (placements 341/383) set ch 21 → d00-d04 (placements 442-446);
  the terrace pads (342/343/424 + the four 22-pads 467-470/344/345) set ch 22/23; the terrace-east
  pads (347/348) set ch 24; the back-garden pad (349) sets ch 25.

## The locks (tests, through the real viewer path)

- `the_guards_wake_from_their_zone_pads_and_take_damage` — the start census (25 present,
  damageable, alive); 600 real ticks leave every gated guard in WAIT_SWITCH with the AI off and
  nobody invincible/missing; the bunker-exit pad teleport sets ch 21 and d00-d04 wake — d00
  (the guard beside the bunker exit) enters combat at alert 1.0; the terrace pad sets ch 23 and
  wakes the playtest's quoted pair — **d05 (the guard) + d23 (the kind-8 civilian, 1.3 units
  apart)**; a woken guard then takes P2K bullet damage through the real `player_bullets` path
  (health drops). The other zones' guards correctly stay gated.
- `the_far_garden_pair_is_legitimately_dormant_on_channel_8` — placements 461/462 (the far
  garden) wait on ch 8 and hold WAIT_SWITCH through the walk.

## Why nobody is invincible and the NPCs are shootable — the game's own rule

`damageable` (DCVars+0x14) is skin-keyed in `NDrone2_DefaultInit` 0x14b300 [M]: only skins
0x0500003b/0x0500003c (the Kimiko scene guests) and 0x05000053 start with +0x14 = 0; everything
else defaults to +0x14 = 1 (line 53). NONE of 07000002's skins is on the exempt list, so every
guard AND civilian on this level is shootable — the port matches the game (`Drone::new`'s
`!matches!(skin, 0x3b|0x3c|0x53)`). The test asserts the census holds and a woken guard bleeds.
The player-facing "invincible, no AI" experience is the WAIT_SWITCH gating above: the guards are
legitimately frozen until you cross their zone pad, and the pads are small (1.4-unit door pads
[D]) — walking near a guard without crossing his pad leaves him frozen, exactly as the game does.

## What's left / live checks for the main agent

1. **The far-garden pair (placements 461/462, ch 8).** No 07000002 trigger, door (`ch_open`),
   inverter, or 0xE1 Destroy placement writes channel 8 [D the census dumps — the three Destroy
   objects write 72/0/130]. The game's remaining writer is the script event command 0xd [M
   `Script_EventHandler` 0x1d94c0:166 `switch_channels[ch] = value`]. **Check**: in PCSX2 on
   07000002, watch `switch_channels[8]` (gdbwatch, write accesses) during a normal playthrough —
   the expected writer is a script event (the level's story scripts); if the game NEVER sets it,
   the pair is dormant for the whole level in the game too and the port is correct.
2. **The pads' in-game reach** — the port's touch volumes are the placement-derived triangles
   [M Trigger_Create 0x1b1720]; the pads are 1.4 units wide. If the main agent's playthrough
   crosses a pad (e.g. the bunker exit at (−25.0, 5.4, 42.5)) and the guards still don't wake in
   the port build, the contact classification (`Trigger_Hit` 0x1b1fc8's contact-list port) needs
   the live check.

## What I ran

- `cargo test --offline --release -p nf-view --test inert_guards_02_live` — 2/2 passed.
- The fast runner (`tools/test/suite.py`) started 14:16 UTC on this branch (the task changed NO
  library code — the port's mechanisms were already the game's); the total line is appended to
  the WORKLOG entry below.
