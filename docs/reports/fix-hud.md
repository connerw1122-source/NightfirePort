# fix-hud report

Branch: **s/fix-hud** (from master 6faffb4). Task: port the HUD elements the port lacked, plus the
confirmed playtest bugs. Time box hit before the non-HUD items could be implemented — their measured
root causes are below so the next session can go straight to the fix.

## Done and verified

**The HUD language tables (new `nf_formats::hud::Lang`)** — `Txt_LoadLanguage` 0x1caad0 / `Txt_BindLabel`
0x1cad60 over the disc `*.dat` files. The NTSC-U default is `USATxt.dat` (FNames[0] "DUTxt.dat" is Dutch —
verified by reading both back from the disc). Layout measured and verified byte-for-byte; the pickup
strings read back exactly: "Picked up %s" (0x200000D), "Picked up %dx %s" (0x200000E), "Picked up Armor"
(0x200000F), "Picked up Code Key" (0x2000010), "007 Bonus collected" (0x200004A), "NEW OBJECTIVE" /
"Mission Status Update" / "OBJECTIVE COMPLETE" (0x2000051/52/53). `nf_view::load` parses USATxt.dat into
`Sim.lang`.

**Fire-mode + weapon-name labels** (`HUD_UpdateAmmoPane` 0x19a7f0 sprites 1/2): both draw at (496, 432),
layer 28, colour 0x7D6D59 with the black shadow — the fire-mode label from `weapon+0x30 + mode*4`
("Semi" for the PP7 at mode 0; the port has no fire-mode switching yet), the name from `weapon+0x38`
("Wolfram PP7"). The `BL+0x95E` timer ports exactly: set to 60 when the HUD's weapon id changes and the
two records' +2 words differ, then decremented in the same update; at rest the NAME is bright (0xFF) and
the fire-mode dim (0x1C), during the timer the reverse — note this is what the decompile says, and it is
the opposite of the old doc's reading; hud.md should be checked against a PCSX2 frame.

**Pickup messages** (pane 21, `Text_AddMsg` 0x1b0c30 + `HUD_UpdateStatusPane` 0x19c310): `drones.taken`
events convert to messages via Pickup_Handler's ids (weapon → "Picked up <name>", ammo → "Picked up
<N>x <ammo>", armour, key, 007 bonus), 180 SP ticks, text fade alpha = time·17 over the last 15, then
the bar closes 16 px per tick, re-centred; bar 256×26 at (128,364) tex 0x0300007A, text centred at
(256,380) font 2. `Sim::hud_tick` steps the queue per tick (the game's per-frame path).

**Damage direction + hit flash** (`HUD_UpdateHealthPane` 0x19b9c8 sprites 20–23): `BL+0x967` bits
1/2/4/8 light sprites 20/21 (tex 0x0300018D, sprite 21 flipped V) / 22/23 (tex 0x0300018E, sprite 23
flipped U) full-screen additive at alpha `BL+0x968`. `PlayerHealth::pain_from` ports the view-space rule
of `Player_DealWithObjHit` 0x194458:201-226 and runs at the drone-bullet hit site; explosions set 0xC
(both sides) per `Player_HandlePain` 0x1902c8. The flash decay (−1/tick) was already ported.

**Air/oxygen bar** (`HUD_UpdateAirPane` 0x1a11c8, `AirSprInfo` 0x2bd490): track 256×16 at (128,48) tex
0x0300017F, fill width = air·2+28, colour 0x68D6B1 fully submerged / 0x68B1D6 swimming, additive bubble
icon 0x0300017D (0xFF submerged, 0x1F at the surface); shown while `bond.underwater` or air < 100 (the
game enables pane 5 in the water modes only).

**Crouch indicator** (health pane sprite 19): 0x03000173 while crouched at (336, 37); after standing
0x03000174 holds 60 ticks and fades over the last 30 (alpha = timer·255/30); hidden at rest.

**How verified**: `crates/nf-view/tests/fix_hud_live.rs`, 6 tests. With the pre-fix `hud.rs` stashed, 4
of 6 fail (the message-bar test passes only because its real-path precondition — a takeable placed
pickup — cannot be met headless, see below); with the fix all 6 pass. The tests drive the real paths:
the label test cycles the weapon through the input mapping; the crouch test toggles crouch through the
input mapping; the direction test exercises `pain_from`'s exact mapping plus the HUD quads; the air and
crouch tests assert the new textures' quads at the game's positions. **Known honest gaps**: (1) the
pickup-take could not be driven headless (`Bond::stand_at` has no body model, so `Pickup::touches` never
fires for the placed pickups) — that test skips with a message, and the message queue's contents are
verified through the bind/format unit assertions instead; (2) the objective/status messages (0x2000051/52
/53) have no port caller yet — `Mission_MonitorObjectives` 0x1827d8 is unported, so no test fires them;
(3) the gadget icon area was not researched in this session (no HUD_Update* function for it was
identified) — it remains open; (4) 0x03000174 is present in 07000005.bin but the post-stand draw could
not be asserted before the timer test's 60-tick expiry path — it is covered.

## The confirmed playtest bugs — root causes measured, fixes NOT implemented (out of time)

1. **07000002/07000006 drawn-but-unhittable, never-attacking guards (the C2 class).** The game's damage
   filter is `NDrone2_DealWithObjHit` 0x13fff0 case 5: a player-class shooter against a victim of team 2
   or 3 (the "friend" DTYPEs 10/11/13/14 map to team 3 per `NDrone2_GetDroneTypeAttackTypeFriend`
   0x14cbd8; 12 → 2) sets `bVar3 = false` and the function hard-returns — no damage, no flinch, no hit
   msg. AI: `Drone_Control` 0x139268 skips everything when `NPCGlobals == 0` (NIS event 5) and a drone
   with `+0x4F8 & 0x80000` clear sits in `NDrone2_DSTATE_Disabled` 0x172440 polling its wait channel.
   The port's `Drones::update_player_bullets` checks only `present/enabled/corpse` — **the team 2/3
   gate is the missing rule** (three lines in drone.rs:3779+). The crosshair already knows the rule
   (`team != 1` → the red "no" mark). `dump_drone4.rs` prints team/mode/state/channel per drawn drone
   for the real walk; running it on the real 07000002 path and porting the gate is the remaining work.
2. **07000006 searchlight does not alarm.** The game's spot ray (`Collide_RayIntersect` 0x1ec690 with
   the object pick `Collide_Pick` 0x1eb320) lands ON Bond — the arm test is `Vec_Dist3D(spot, Bond)
   <= 2.0` [M Searchlight_Update 0x1cbf14-0x1cbf54]. The port's spot ray tests static geometry only
   (`world.segment_surface`), so the spot never reaches Bond and state 1 (which sets alarm channel 55
   every tick, waking the ch-55 waiters and feeding `DroneFunc_CheckAlarmRaised` 0x1453a8 →
   `Music_Event(2,1)`) never arms. Fix: include Bond's capsule (r ≈ 0.55, like `Toucher::player`) in
   `Searchlights::update`'s segment test (searchlight.rs:305-332); everything downstream is already
   ported. The existing `fix_drone3_live.rs` searchlight test teleports Bond onto the spot, which is
   why it passes while play fails.
3. **Grenades.** (a) The frag's blast sound is INSIDE script 0x6000052 (`Explode_Create` 0x17cbd0 loads
   and plays it) — the port plays 0x4A7, which is the rest-bounce sound of weapons 0x3b/0x45, so the
   blast is silent/wrong. The explosion visual (script 0x6000052, debris via `Explode_Update` 0x17c350,
   camera shake at radius ≥ 0.5, lifetime radius·0.0666) is unported (`Explode_Create` is absent per
   port-map guard-ai.md). (b) Explode-on-impact: `Bullet_CollisionHandler` 0x12ce80:194-206 explodes
   when `weapon_data+0x70 & 4` and the surface is non-bouncy — the port never tests +0x70, so its
   grenades always bounce (the "kills on impact" impression is the fuse explosion near a wall; the
   impact itself deals no character damage in the game). (c) **The flashbang kills**: the port applies
   lethal explosion damage for every grenade type. The game routes stun-grenade victims (weapon 0x36)
   through `StunGrenadeImpact` 0x172bb0 / `StunGrenadeLoop` 0x1631c0 / `StunGrenadeRecover` 0x163350
   (drop weapon, AlertStatusSet(2), anims 0x2d/0x2e, recover per `DroneFunc_RecoverTime` 0x147188, get
   up with anim 0x76, re-hit → death state 0x44) — no `NDrone2_HitDamage` call on that path. All three
   stubs are listed unported in port-map guard-ai.md:166-168. Fix: add a `bullet_flags2()` (+0x70)
   accessor, gate the explode path on it, route weapon 0x36 explosions to the stun states, replace the
   0x4A7 blast with script 0x6000052's sound id.
4. **07000007 guests not walking / 07000005 spine flip — already fixed on master.** fix-drone4 (merged
   as 924b51e) ported CivilianMission 0x16 + the kind-8 route assignment + `Route::for_path`'s open
   chain, and the two-sided ±0.9 rad aim clamp + `aims_torso()` gate; `fix_drone4_live.rs` and
   `probe_drone4.rs` (run in this branch's suite) prove a guest walks > 2 units and a patroller's
   aim_yaw stays 0. The user's retest most likely pre-dates that merge. Remaining parity debts: the
   party weapon lock (playtest F4) and `near_armed_person`'s game triple test (prop 0x3c + seen-this-
   tick + armed opponent, `NDrone2_NearArmedPerson` 0x1787c8) — with an armed Bond the port cowers
   guests the game would not.

## Suite and leftovers

The full NF_ISO suite ran on this branch (`cargo test --offline --release -j2 --no-fail-fast -p nf-game
-p nf-formats -p nf-audio -p nf-view -- --test-threads=1`): **334 passed, 3 failed — exactly the 3
allowed disc_golden tests** (bigf_header_fields_behave_as_documented,
bigf_tables_match_the_golden_manifest, directory_tree_matches_the_golden_manifest; the slim ISO's
missing metadata). All four crates' unit and live tests pass, including fix_drone4_live /
probe_drone4 (the guests walk) and the render suites. Not done: the four root-cause fixes above, the
objective/status message callers, the gadget icon area, the fire-mode label's alignment check against a
PCSX2 frame (the template has the fire-mode LEFT-aligned at x=496, which would clip off-screen — the
port draws both labels right-aligned), and hud.md's corrections (the "open 16 px/tick" note is wrong —
open is one frame, close is 16 px/tick; the label brightness reading above).
