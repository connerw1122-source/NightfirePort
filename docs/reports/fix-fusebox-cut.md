# fix-fusebox-cut — the fusebox wires are cut by hits (the Laser), not by pressing E

## What the game does (all cited)

- `FuseBox_Update` 0x1adfe8 walks the box's two wire groups (10 slots each, `FuseBox_Register`
  0x1b0970) and per wire filters its hit list (`obj+0xD0`) with `Collide_FilterBullets(list,
  0xfffd)` [M 0x1adfe8:88]. The filter DROPS class-5 (bullet) hitters whose `weapon_definition+0x10`
  category word ANDs with 0xfffd non-zero, keeps the rest [M 0x1ece30:24-31; the same drop polarity
  `Break_Update` 0x12bfd8:15 (`ext+4|0x106`) and MP demolition (mask 1) confirm]. A wire whose list
  still holds an entry after the filter DIES (`Control_NextLOD` + `Effect_RemoveSObjs`, :93-96);
  a group-1 death hurts Bond 5.0 iff +0x56 == 0 and zeroes +0x5C (:99-102); an emptied group with a
  non-zero registered count writes both channel words and zeroes them (one-shot, :110-123 — group g's
  words are +0x54+2g and +0x5C+2g).
- **Which projectiles cut**: the disc's category words [L weapon_data dump over the disc's own
  ACTION.ELF initialiser + USATxt.dat name labels]: every gun is 0x0008 → dropped → **ordinary
  gunfire never cuts a wire**. The weapon named **"Laser"** on the disc (0x4E/0x4F — the upgrade
  table's `Lasers`) carries 0x0002 → kept → **it cuts**. The Stunner (0x0004), Grapple,
  Micro-Camera, Decryptor, Q-Worm (0x0100) are dropped. Grenades/Ronin/Shaver (0x0000) would be
  kept. The task's guess "the watch's bullets" is corrected by the dump: the watch (0x54) and the
  beams with ids 0x56/0x57 carry 0x0100 — they do NOT cut; the cutting weapon is the disc's
  "Laser" (0x4E/0x4F, range 4.0, speed 33.3, damage 0 [L record]).
- **E can never reach a fusebox**: `Player_Activate` 0x1a5170:95-160 dispatches SP (class 0 +
  obj+0xE4), 0x1C door, 0x1D trigger, 0x25 SS, 0x28 car, 0x36 GT, 0x38 monitor, 0x3A lock, 0x47
  GunImp, 0x49 PCQWorm — **no class-0x3C arm**. The old two-E-presses stand-in was wrong in kind.
- **The wires register** through the box script's event 0xF: `Script_EventHandler` 0x1d94c0:172-175
  calls the SP create callback with the object of stream **i−1** (the handler's record base +0x80
  sits one 0x28 record below `Script_Run`'s +0xa8 — the same previous-stream pattern as events 6
  and 0xB) and the event's param 0 as the group [L NF_SCRIPTLOG: 07000008's 0x060008AD stream 2 →
  group 0 (one wire); 07000014's 0x06000A1B streams 2/6 → groups 0/1 (two wires per box)].

## What changed

- `crates/nf-game/src/fusebox.rs`: real wire groups (10 slots, counts), `register_wire`
  (`FuseBox_Register`), per-tick hit marks, `update` = `FuseBox_Update`'s filter/death/channel
  logic (channel pairing `ch[g]`/`ch[g+2]`, ext zeroing = one-shot, the fixed +0x56 polarity and
  +0x5C cancel kept). The E stand-in (`player_use`, `opened`, `open_t`, `wires`, `gone`) removed.
- `crates/nf-game/src/nis.rs` (additive): `Request::WireRegister { player, obj, group }` + the
  event-0xF arm resolving the previous stream's object (kind 1/2).
- `crates/nf-game/src/drone.rs` (additive hooks): the request-drain arm registers wires into the
  box by placement; `update_player_bullets` clears the per-tick wire hits (Collide_Update 0x1ea398's
  head-of-frame clear) and marks kept-kind contacts on the wires' spheres (segment-vs-sphere r 0.5
  [S — unmeasured] at the script object's world position, entities and anims both); the per-tick
  `FuseBox_Update` consumes the marks after the script requests drain (dying wires lose their
  entity = `Effect_RemoveSObjs`; the group-1 hurt rides `pain_generic` like the explosions).
  REMOVED: the fusebox loop in `player_use_doors`.
- Tests: `crates/nf-view/tests/fix_fusebox_cut_live.rs` (4 tests) + re-driven
  `fix_party_mission_live::the_07000008_safe_chain_works_in_play` (the laser burn replaces the two
  E presses; the whole chain — bolt → safe → briefcase → gondola — passes) and
  `walk_07000014_live::the_facility_chain_runs_to_the_delta_sector_exit` stage 3 (both generator-room
  boxes cut with the laser; the sabotage objective completes). `fields_live.rs` and
  `walk_tower2_probe.rs` updated to the new model.
- Docs: `docs/fields/fusebox.md` mismatch #5 marked FIXED with the kind-word census.

## Verification

- Fail-first: the new tests need the new API (`FuseBox::register_wire`/`mark_hit`/`update`,
  `Request::WireRegister`, `Drones::script_obj_pos`) — they do not compile against the pre-fix
  tree, and the pre-fix behaviour they contradict (E cuts, no wire registration, no hit path) is
  in HEAD~1 (commit `20fe32b`..`d0650b9`). The nf-game unit tests additionally pin the filter
  polarity (Laser 0x0002 cuts / guns 0x0008 and the 0x0100 family do not), the one-shot channel
  tail, and the group-1 hurt/cancel polarity.
- All four `fix_fusebox_cut_live` tests green (the laser cuts 07000008's box #219 → ch 137 → the
  doorway bolt SS opens; E presses do nothing; 120 ticks of P2K fire cut nothing; the 07000014
  sabotage completes with no live-wire hurt — the boxes carry param 2 non-zero [D]).
- Full suite `cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view
  -- --test-threads=1`: green except the 3 allowed `disc_golden` failures (slim ISO).

## What's left / live checks for the main agent

1. The wire's world sphere radius is [S] 0.5 — the game collides the bullet's HITTEST against the
   wire's real geometry. A `gdbwatch.py` pass on 07000008 while burning the box: watch the wire
   object's hit list head (obj+0xD0) the frame the laser touches — confirm the wire dies on the
   FIRST contact tick and that the visual removal matches `Effect_RemoveSObjs`.
2. The Laser's exact weapon identity in play: the disc name is "Laser" (0x4E/0x4F). A render at
   the guard-room box (`--bond 156.3,43.0,-119.3` facing the box) should show the beam reaching
   the wire and the wire mesh disappearing on the cut.
3. The 07000008 box's registration is group 0 only — if the game also cuts a second (live) wire
   there, the SP's later event-0xF traffic was missed; a script-log watch of hash 0x060008AD in
   PCSX2 would settle it.
