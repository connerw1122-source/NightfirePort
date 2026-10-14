# fix-qclaw — the Q-Claw: no hook model, a bullet impact instead of the grapple

Branch `s/fix-qclaw`. The playtest (results 1013-1758, 07000005 at
`--bond 16.5,29.6,58.6,0,0.25 --give 0x50`, beside grapple point #282 (13.75, 32.29, 57.3)):
"the grapple hook isn't there; using the hook within range makes a bullet impact".

## Root cause — the gadget ids were shifted one table over

The port's `gadget.rs` put the Q-Claw on 0x4A/0x4C and the Q-Laser on 0x50/0x51. The disc says
otherwise [L ee_start_05.bin @ weapon_data 0x2bf150 (file offset = EE address) + USATxt.dat, probed
with a scratch reader, since deleted]:

| ids | bank name (+0x38 label) | +0x70 | +0x1C range | clip |
|---|---|---|---|---|
| 0x4A/0x4B | Stunner (0x05000042) | 0x800 | 5.0 / 1.75 | 100 |
| 0x4C/0x4D | Stunner | 0x800 | 10.0 / 1.75 | 100 |
| 0x4E/0x4F | Laser (0x05000043) | 0x800 | 4.0 / 8.0 | regen |
| **0x50/0x51** | **Grapple (0x05000044) — the Q-Claw** | **0x40** | **50.0 / 100.0** | 0 |
| 0x54/0x55 | Micro-Camera (0x05000045) | — | 50.0 | regen |
| 0x56/0x57 | Decryptor (0x0500004e) | — | 2.0 / 4.0 | — |

The Q-Claw is **0x50 (base) / 0x51 (upgrade)**, confirmed by every reader in the decompile:
`Bullet_Delete` 0x12e660:33 branches on `weapon_data+0x70 & 0x40` and its claw path latches the
grapple (the hit object's hitdata +0x58 straddler must be class 'Q' = 0x51, then
`Player_SetGrapplePoint` 0x195da0 + the latch sound 0x28d); `Bullet_CollisionHandler` 0x12ce80's
`weapon_data+0x02 - 0x50 < 2` band; `Player_GrappleSetRope` 0x192728's `BLData+0xdc+0x62 - 0x50 < 2`
rope gate; `Player_InitGrapple` 0x195d10 creates the rope mesh 0x2000138 at BLData+0x158; and the
level loadouts grant 0x50 (07000005's start, `Inventory::level_start`).

The playtest's symptom chain was two bugs stacked:
1. `--give 0x50` printed "refused" — the claw is **already owned** at the 07000005 start and the
   game's own equip declines the re-equip (`Player_EquipWeapon` 0x1a8a58's owned branch: the claw's
   ammo type 0 has cap 0 [L +0x90/+0x92]), so the give never set the switch request and Bond stayed
   on the P2K: firing gave a **real bullet** and a real bullet impact.
2. `is_laser_weapon(0x50)` was true, so with the claw out the port also ran the laser beam every
   held-fire tick — the beam's endpoint rendering as the "impact".
3. The grapple fire gate checked `0x4a | 0x4c` (the Stunner), so even a drawn claw never latched;
   and `grapple_range` returned 5.0/10.0 — the Stunner's ranges — which a test had baked in as
   "the user's spot is out of range" (15.4 units; inside the claw's real 50.0).

## What's done

- `gadget.rs`: `is_laser_weapon` → {0x4E, 0x4F, 0x54, 0x55} (the Laser records carry the only
  +0x68 laser-pointer flag 0x2 that `Player_LaserPointer` 0x1a65d8 gates on; the watch 0x54/0x55
  keep the aim-gated beam [M 0x1a5460:512-514]); new `is_grapple` = {0x50, 0x51}; `grapple_range`
  reads the weapon's own `weapon_data+0x1c` [M Bullet_update 0x12e7b0:158] instead of hardcoded
  5/10; the module header documents the settled id table; the Lock_Update name comments corrected
  (0x4E/0x4F "Laser", 0x56/0x57 "Decryptor" — the numeric burn bands unchanged).
- `scene.rs` (shared-file hook, listed below): the grapple fire gate uses `is_grapple` and the
  record's own range.
- `lib.rs` `--give` (shared-file hook): an id the game's equip refuses **because it is already
  owned** still gets its switch request (the give's contract is to draw it; the game's own
  `Player_WeaponSelect` path). Without this the playtest's `--give 0x50` did nothing at all.
- `gun.rs` / `player.rs`: untouched — the gadget cycle band (0x4A..0x5E) and the SS_GRAPPLE
  substate (`Player_SSGrapple` 0x192860's reel/hang/release polarity) were already correct.

## Tests (all through the real input path, `nf_view::load` + `Sim::step_bond`)

New `crates/nf-view/tests/qclaw_live.rs`:
- `the_claw_draws_and_the_held_fire_latches_reels_and_hangs_with_no_beam_and_no_decals` — the
  playtest flow: `--give 0x50` draws the claw (view model = the claw's own skin 0x05000044 with its
  rigid parts; D-003's cycle gate lists it), the held fire at #282 from the playtest spot latches,
  reels Bond in and hangs him (< 2.5), **`laser_beam` stays None every tick** (the "bullet impact"
  regression) and **`effects.decals` stays empty** (the game's claw bolt spawns no decal: the claw's
  +0x70 = 0x40 has no effect bit, `Effect_Create` 0x1ce688's kind-0 dispatch runs nothing [M]);
  the release detaches back to the walk substate.
- `the_stunner_does_not_grapple_and_the_claw_does_not_beam` — the sister: the id sets + the ranges
  (50.0/100.0 off the merged table) + through the sim the Stunner 0x4A fires at #282 and never
  latches or beams.
- `the_users_report_spot_latches_within_the_real_range` — the user's other spot
  (`-18.28,38.19,-71.83`): the nearest point 15.4 out, inside the claw's real 50.0, the bolt sweep
  accepts it (the old test asserted the opposite from the wrong 5.0).

`gadget_grapple_live.rs` + `gadget_grapple_user_spot.rs` re-pointed at 0x50 (they equipped 0x4A —
the Stunner — and grappled through the old wrong gate); `gadget_select_live.rs` comment names fixed.

Failing-first: a scratch probe (`probe_qclaw_symptom.rs`, deleted) ran the playtest flow on the
pre-fix tree and failed at `--give 0x50 equips` — the refusal the fix removes; the behavioural
assertions (no latch, beam on) are covered by the new suite's pre-fix state of the same lines.

## Verification

- The three grapple/qclaw test binaries: 6/6 green; nf-game unit tests 140/140; gadget/laser/dev
  panel neighbours green (gadget_select 6, gadget_laser 4, fix_dev_panel 2).
- Full suite on the final tree (merged with origin/master 810c109), run as 214 fresh binaries
  serially (`--test-threads=1`; suite.py's nohup form cannot survive this sandbox — the same
  fallback the previous session recorded): **nf-game + nf-formats + nf-audio: 43 binaries, 42 ok,
  the only failure disc_golden (11 passed / 3 failed — exactly the 3 allowed slim-ISO tests);
  nf-view: 171 binaries, all ok.** After the merge, the touched + master's new binaries re-ran
  green (qclaw_live, gadget_grapple_*, fix_objective_messages_live, fix_pause_audio_live,
  devwin_flow).

## Shared-file hooks (the driver's list: drone.rs, scene.rs, player.rs, lib.rs)

- `scene.rs`: the grapple fire block's gate line + range argument (2 lines) and its comment.
- `lib.rs`: the `--give` already-owned branch (8 lines + comment).
- `drone.rs` / `player.rs` / `gun.rs` / `nf-formats/*`: untouched.

## What's left / notes

- **The rope and the flying claw are not rendered** (the grapple works, the visuals don't exist).
  The game draws rope mesh 0x2000138 hand→point with `Mat_Align2Dir` and the stretch in +0xe8
  [M 0x192728] while `weapon − 0x50 < 2`; `Grapple_Delete`/the phase-4 detach clears it. The port
  has no rope draw; the next visual task should port `Player_GrappleSetRope` 0x192728 into the
  renderer (a line/stretched mesh from the view model's hand bone to the point).
- The claw fp skin's skinned main mesh **0x02100717 resolves on no archive of the disc** (probed
  every FILES.BIN archive; only 23 skin references exist, no object). The port's claw view model
  draws the sleeve + the skin's rigid parts (bones 38..42) and skips the unresolvable main —
  whether the real game shows more than that is a live check below.
- `drone.rs` ~1948's comment calls 0x4A/0x4C "the Q-Claw's grapple state" in the msg-7 taser arm —
  the ids are right for the taser (the Stunner) but the name is the old table's; not touched
  (shared file, comment-only). Same for `tests/common/tower2.rs`'s "the Q-Decryptor is held" (0x4E
  is the Laser per the bank; the id assertion is correct and passes).
- The claw's flying bolt in the port: the latch swallows the trigger on the latch tick (no shot);
  a no-latch fire still spawns a real PlayerBullet whose world impact spawns no decal (the +0x70
  bit-2 gate already models `Effect_Create`'s kind 0) — the game's bolt behaves the same but is a
  slow visible claw object (speed 0.667/1.333 [L]), which the port does not draw.

## Live checks for the main agent

PCSX2 (07000005, spot 16.5, 29.6, 58.6, the claw out — `Player_WeaponChange`'s gadget cycle or the
loadout):
1. Fire held at #282: the latch sound (id 0x28d [M Bullet_Delete 0x12e660:53]) plays, the rope
   draws from the hand to the point, Bond reels in and hangs; releasing the button drops him.
2. Fire the claw at a plain wall: expect NO decal and NO debris (Effect_Create's kind-0 path
   [M 0x1ce688]) — the port matches this.
3. The in-hand claw look: the port shows the sleeve + the skin's rigid claw parts (bones 38..42);
   confirm what the real game shows for the same pose (the skinned main 0x02100717 resolves
   nowhere on the disc — if the real game draws a full arm, the mesh comes from a lane we haven't
   found; if it shows the same parts, the port is already right).

nf-view render checks (after the rope task, or now for the grapple itself):
- `--bond 16.5,29.6,58.6,0,0.25 --give 0x50` — the claw view model in hand, the HUD's ammo pane on
  the claw record; fire held → Bond travels to #282 (13.75, 32.29, 57.3) and hangs.
- `--bond -18.28,38.19,-71.83,-4.717,0.151 --give 0x50` — the user's report spot: the held fire
  latches the 15.4-unit point (the real 50.0 range).
- `--bond 16.5,29.6,58.6,0,0.25` + the Stunner (0x4A, the cycle's first gadget) — fire at #282:
  no grapple (the sister).
