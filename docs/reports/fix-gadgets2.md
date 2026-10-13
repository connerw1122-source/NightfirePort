# fix-gadgets2 — the gadgets as the user played them: the held-fire polarity, the grapple, the sounds, the boxes

Status: 5 of the 6 reports fixed and tested; the micro-camera overlay (report 4) is not started —
the findings and the live checks are at the bottom.

## 1. The hold-to-use input was backwards — FIXED

The user: "If I click in, they start. But then when I let go, they actually release and actually do
what they're supposed to do briefly." Three game mechanisms were missing, and one was inverted.

- **The burn ran while equipped, not while fired.** The port's `laser_tick` ran every tick whenever a
  laser weapon was out. The game's burns are the SHOTS the held fire spawns: the held branch of
  `Player_WeaponFiring` (Input_Action(9,1) — mask 1 = held [M 0x1a5460:442, 0x17f208's state byte:
  bit0 held, bit2 press-edge]) decrements the refire timer each tick [M 0x1a5460:601-630] and each
  expiry spawns via `Player_WeaponInitBullet` 0x1a4bf0 → `+0x95b` → `Player_MuzzleFlash` 0x1a4298 →
  `Draw_LaserBeam` 0x1a4560 (weapdef+0x6c bit 0x100) / `Draw_TaserBeam` 0x1a46a8 (bit 0x80).
  Fixed: the burn runs only while the fire is held, and for the watch laser/taser (0x54/0x55) also
  only while AIMED [M 0x1a5460:512-514]. Release = the burn stops the same tick.
- **The fire script restarted every held tick.** `scene.rs` queued the fire script from frame 0 on
  every `fired`. The game SKIPS `Player_SetFiringAnim` while the view state is already 9 and
  `weapon_data+0x68 & 0x100400` [M 0x1a5460:664-672] — the script plays through and the release-frame
  spawn (case 9 of `Player_SetWeaponAnimObj` 0x1a2e38:416-420) happens WHILE HELD. For the Q-Laser
  (fire_anim_len 20) the old code could never reach frame 20 while held — the shot only spawned on
  the button-up, which is literally the user's sentence. Fixed with the state-9 gate.
- **The watch's per-shot sound** [M 0x1a4bf0:261-263 — `Sound_Play(100.0, 0xec, 0, 0)` on every shot
  of weapon_data[+2] == 0x54] plays per held tick now (the watch's interval 0 → the 1.0 floor fires
  every tick [D weapon_data 0x54+0x40]).

Tests: `gadget_laser_live::the_watch_laser_burns_only_while_the_fire_is_held` (aimed WITHOUT fire →
no burn — this is the fail-first form: the pre-fix port burned in exactly that state; hold → burn),
plus the pre-existing tests updated to hold the fire (and aim for the watch) per the game's gates.

## 2. The sounds kept playing after the stop — FIXED

`AnimProcessScriptCmds` op 4 sub-op 3 is `Sound_StopAllWithId(cmd[3])` [M 0x1c1b38:388] — the fire
scripts stop their own loop sounds at a scripted frame. The port played the op-1 starts and dropped
the op-4.3 stops, so every gadget loop leaked; the old restart bug made it worse (the script
restarted every tick, the stop frame never came). Fixed: the op-4.3 commands run in the same
frame-window pass as the op-1 starts. The game's safety nets (`Player_ChangeSubState` stops ids
0x12/0x3d/0xf0 [M 0x190630:83-85]) were already ported.

## 3. The grapple "doesn't work at all" — FIXED (the polarity), plus the honest range

The user's spot: 07000005 `--bond -18.28,38.19,-71.83,-4.717,0.151`, the claw selected, aimed at the
wall point. Reproduced with a hold-shaped click through the real input path
(`gadget_grapple_user_spot`, the aim sweep): the port latched on the press tick and DETACHED ~2 ticks
later while the button was still down — invisible. The game's polarity is the opposite:

- Phase 1 (the reel) reads NO fire input at all [M `Player_SSGrapple` 0x192860:79-121]; the pull is
  0.05/tick up to **1.5** single-player [M 0x192860:96-104 — the port had 0.5], arrival < 2.0 hangs.
- Phase 2 (the hang): `Input_Action(9,1)` HELD commits the velocity and STAYS; the RELEASE moves to
  phase 4 and detaches [M 0x192860:56-66]. The pre-fix port released on fire — backwards.
- The acceptance test is a physical one: the claw's bolt (object radius 1.0 [M Bullet_init
  0x12c5a0:92]) flies and its death check is the travelled distance vs `weapon_data+0x1c` [M
  Bullet_update 0x12e7b0:158] — 5.0 for the 0x4a claw, 10.0 for the 0x4c [D weapon_data_live.bin;
  the scale confirmed by the fists 1.75 / P2K 50 / sniper 1200]. The port used a bare aim-ray with a
  100.0 range. Fixed: the point's sphere grown by the bolt's radius, within the weapon's range.
- **The user's screenshot spot is 15.4 units from the nearest point — out of the claw's 5.0-unit
  bolt range, so the real game cannot attach from there either.** The test
  `the_users_spot_is_out_of_the_claws_bolt_range` pins that; `the_q_claw_holds_while_held_and_
  releases_on_the_button_up` drives the full loop from inside the range: hold → latch → reel → hang
  → stays while held → the button-up detaches. The game's SP `+0x95f == 2` ThirdIcon gate for the
  Q-Laser-family fire [M 0x1a5460:523, `ThirdIcon_Update` 0x196e80:52] is NOT ported — the port is
  more permissive than the game there (it has no ThirdIcon system); listed below.

## 4. The micro-camera zooms through a sniper scope — NOT DONE

Not started; the session went to the five fixes above. What the research established for the next
run: the scope overlay the port draws (`SCOPE_TEX` 0x03000034, hud.rs `zoom_target > 1.0`) is the
sniper's SightPane texture from the 07000005 archive [D entry 71]; the camera's own pane exists as
`HUD_UpdateCameraPane` 0x1a12d8 (it reads `BLData+0x95b` [M 0x1a12d8:18]) — the port has no such
pane. The port needs: the camera-zoom HUD selection by weapon (0x52), the 0700000c archive's overlay
texture ids, and the viewfinder geometry from the main agent's screenshot (rounded-rectangle
viewfinder, black outside; left/right level bars ending in curved hooks with 5+5 tick columns; a
half-filled centre circle; green corner brackets top-left; the "Micro-Camera" label bottom right; no
reticle).

## 5. "I have the Q-worm on a level that doesn't need Q-worm" (0700000c) — the port MATCHES the game

The port's 0c case is the game's case, verbatim: `Player_InitWeapon` case 0x700000c runs
`Player_InitAmmoWeapons`, equips the handgun (2) at 48, `Upgrade_Weapon(0x1e)` (the sniper) and
`Upgrade_Weapon(0x4a)` (the Q-Claw) at 999, and **0x50/0x54/0x58 at 0**, selecting the sniper [M
0x1a7ef8 case 0x700000c]. 0x58 IS the Q-Worm (`PCQWorm_Update` 0x1aee20; its fire needs a class-'I'
worm target within 2.0 [M 0x1a5460:530-547] — of which 0c has none, so it is inert there). The game's
own cold 0c start carries the Q-Worm in the cycle. `--dev` adds nothing: it only tops up OWNED
weapons' ammo (dev.rs), it grants none. No code change; the existing loadout test pins the case.

## 6. The red and yellow boxes — FIXED (the sweep stopped drawing two kinds)

The user's "transitions for the wall shimmy … and even one for the grapple" are the creep-wall
(0x3F) and grapple-point (0x3A) marker meshes that models-misc's `misc_static_sweep` drew. Neither
kind is ever drawn by the game: `Grapple_Create` 0x198298 builds an instance-size-0 object with
**zero** `hashtable_set_object_to_entity_gfx` calls (it reads no placement params at all — only
class 0x51, the glist pointer, LinkToRoom, StraddleTest), and `CreepWall_Create` 0x196fc8 sets class
0x2B with no gfx attach. `View_DrawObjList`'s empty-glist no-op draws nothing for either. Both kinds
are out of `MISC_KINDS`; the census test (`models_misc_live`) drops them and adds must-not-draw
teeth on 0700000c (5 grapple placements [D]) and the creep-wall levels. The other 15 kinds keep
their per-kind proofs in the sweep comment.

## Shared files touched (small, additive)

- `scene.rs`: the burn gate (held fire + the watch's aim), the state-9 fire-script restart gate, the
  op-4.3 stop pass, the grapple range from weapon_data, `MISC_KINDS` minus 0x3a/0x3f.
- `gadget.rs`: the grapple polarity/pull cap, the bolt-sweep pick + `grapple_range`, the tests.
- `player.rs`: untouched. `drone.rs`: untouched. `lib.rs`: untouched.
- Docs: this report, the WORKLOG entry, `docs/port-map/gadgets.md` (the status table + the grapple
  polarity correction).

## Verification

- New/updated: `gadget_grapple_user_spot` (2), `gadget_grapple_live` (updated), `gadget_laser_live`
  (the hold-gate form + the new burn-follows-the-button test), `gadget_select_live`, the
  `gadget.rs` unit tests (the polarity + the bolt sweep + the range), `models_misc_live` (the
  census minus 0x3a/0x3f with teeth). All green.
- The full suite: see the WORKLOG entry for the run record.

## Live checks for the main agent (renders)

- `--bond 16.5,29.6,58.6` on 07000005, the claw equipped, aimed at point #282 (13.75, 32.29, 57.30),
  fire HELD: the rope (0x2000138) should draw hand → point while the reel runs and during the hang;
  releasing the button drops Bond. (The rope is still not drawn by the renderer — parked on
  `Sim::laser_beam`'s pattern; fix-gadgets's leftover.)
- `--bond 160.3,44.6,-115.4` on 07000008, the watch equipped, AIMED at the top bolt, fire HELD: the
  beam (0x2000131/0x2000133) draws while held and disappears the tick the fire releases; the burn
  effect on the bolt; the burn stops with the button.
- 0700000c roof `--bond 5.61,12.49,-154.44,6.286,-0.059`: the red/yellow marker boxes near the
  shimmy walls and the grapple point should be GONE; the grapple point itself draws nothing.
- 0700000c zoomed micro-camera: still shows the sniper scope (bug 4, not done — do not merge as
  fixed).

## Leftovers for the next run

- The micro-camera overlay (report 4) — the plan is in section 4.
- The Q-Laser 0x50/0x51's SP `+0x95f == 2` ThirdIcon zone gate [M 0x1a5460:523] and the ThirdIcon
  placements (type 0xFA: id at +0x2c, channel A +0x30, channel B +0x34 [M 0x197070:21-31]): the port
  lets the trip-laser-family fire anywhere; the game restricts it to icon zones.
- The lock-id split the decompile shows: case-0 burn locks (data[0] == 1, the safe bolts) accept ONLY
  the decryptor's 0x4E/0x4F contacts [M 0x1ae3d8:57-66]; the 0x56/0x57 laser-bolt ids feed the KEYPAD
  decode branch (data[0] 0/2 and case 5) [M 0x1ae3d8:162, 305-314]. The port burns any lock from any
  beam weapon — the watch burns the 08 safe bolts in the current tests, which the game reserves for
  the decryptor. Needs the lock data[0] field modelled before anyone tightens it.
- The grapple's swing momentum at release [S], the rope-fire-anim miss path (+0x60 & 8 with the
  15-tick timer [M 0x192860:89-90]), and the claw-cam substate 10 (the flying bolt's camera [M
  0x12c5a0:248-253]) are still not modelled.
