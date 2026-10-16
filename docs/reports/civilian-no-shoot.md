# civilian-no-shoot — the crosshair rule, and what the game really refuses

Branch `s/civilian-no-shoot` (master ac1fba78 + merge; scene.rs + one test file). The task's
premise — "the game shows the red no-shoot crosshair over civilians AND refuses to fire" — is
**contradicted by the disc**. The port matched the game's fire behavior already; the real gap was
the icon's own rule, which is now ported, and the tests pin the game's actual behavior on
07000002.

## The premise, and the disc evidence against it

The repo's own PCSX2 trace of this exact play (tests/traces/02-civilian.trace, 2026-10-11, the
user's run on 07000002 with the red mark up) shows the P2K's clip falling on EVERY trigger pull:
7→6, 6→5, reload, 7→5→4→3→2→0, reload, 7→6 [L]. The trigger is not blocked. The decompile
agrees at every stage:

- `Check_Target` 0x1a7810 writes the mark (BLData+0x132 = 1, +0x133 = 3) and the aimed object
  (+0x880) — its only readers are the HUD draw [M `HUD_UpdateCrossHair` 0x19e6d0:69 reads
  +0x133] and the gadget/use weapon arms [M `Player_WeaponFiring` 0x1a5460:531-581 — types
  0x56/0x57/0x58 only, i.e. the lockpick/use gadgets, not guns].
- The fire pipeline `Player_Update` 0x193600 → `Player_Weapon` 0x1a24c8 → `Player_WeaponFiring`
  0x1a5460 (the trigger is `Input_Action(player, 9, …)` read inside; the single call site at
  0x194070 passes a1 = 0 [M disassembly]) → `Player_RoundToFire` 0x1a18e0 (ammo bookkeeping) →
  `Player_WeaponInitBullet` 0x1a4bf0 → `Bullet_init` 0x12c5a0 — no target-class consult
  anywhere.
- The damage path has no team gate either: `NDrone2_DealWithObjHit` 0x13fff0 case 5's team
  check applies only to NPC shooters; a class-3 (player) owner proceeds for any team [M
  disassembly 0x1402fc-0x140314: `beq $v1, 3` → the hit proceeds].
- A subagent audit confirmed there is no fire- or damage-refusal site: the action table
  (0x2a39cc/0x2a38dc) is written only by the Input module, and the only non-Input
  `Input_ClearAction` is the reload's press swallow [M `Player_SetWeaponAnimObj` 0x1a2e38:386].

So "aim at a civilian and fire — no shot, no ammo used" is not the game's behavior, and the port
already fired like the game. What the game DOES gate (all three now verified in the port):

1. **The icon itself is keyed on the held weapon** [M `Check_Target` 0x1a7810:152-183]: the
   weapon-class switch on the held weapon id (animData+0x62) marks the no-shoot crosshair
   (+0x132 = 1, +0x133 = 3) EXCEPT for ids {1, 0x35, 0x36, 0x43, 0x44} — the fists and the
   throw family (grenades 0x35/0x36; 0x43/0x44 the other throwables), which keep the NORMAL
   crosshair over the same civilian (you can punch or blast him; the damage path agrees) — and
   levels 0x7000005..0x7000008 never mark at all (`0x7000008 < level || level < 0x7000005`
   guards the whole arm; 07000002 marks).
2. **The auto-aim lock skips non-enemies** [M `Check_AutoAim` 0x1a73f8:83-87 — SP, a class-2
   target with animData+0x44 != 1 returns before the lock]. Already ported (`drone.rs`
   `auto_aim`'s `dr.team != 1` gate); now pinned on 07000002's data.
3. **Killing the protected non-enemy fails the mission** [M `DroneFunc_OnInitDeath` 0x145bd0 →
   `DroneFunc_SetMissionFailReason` 0x13e560 reason 1/3 → `Mission_SetFailLabel(0x4000034)`
   "Killed Civilians" + the fail channel 0x60]. Already ported (fix_actors); now pinned on
   07000002.

## What landed

- `crates/nf-view/src/scene.rs` — the crosshair block's state rule gains the game's weapon-class
  + level key: a non-enemy under the crosshair marks state 3 only when the held weapon id is NOT
  in {1, 0x35, 0x36, 0x43, 0x44} and the level is not 0x7000005..0x7000008 (pre-fix the port
  marked every weapon, e.g. the red icon over a civilian with the fists out).
- `crates/nf-view/tests/civilian_no_shoot_live.rs` (3 tests, green, NF_ISO):
  - `the_no_shoot_crosshair_keys_on_the_held_weapon` — 07000002 after the intro skip: the
    civilian (kind 9, team 2) beside Bond's start shows the mark with the start gun, the normal
    crosshair with the fists (switched through the real number-key path; this arm failed pre-fix),
    and aiming away returns to normal.
  - `the_trigger_fires_under_the_mark_and_the_civilian_takes_the_hit` — firing under the mark
    spends ammo, the civilian's health drops, the P2K kills him and the mission fails with
    "Killed Civilians" (0x0400_0034); aiming off clears the mark and the next press fires.
  - `the_guard_beside_the_civilian_is_shootable_and_the_lock_skips_the_civilian` — the sister:
    a team-1 guard beside the civilian is a normal target (state 2), takes the shot (health
    drops, ammo spent), and the auto-aim locks the guard while never locking the civilian
    [M Check_AutoAim's team gate].
- The held-trigger question from the task, answered by the game's own shape: a held trigger
  fires the semi once per press (the fire action's edge), and nothing latches — the mark is
  per-tick from `Check_Target`'s every-6th-frame ray, so the shot follows the aim the moment it
  leaves the civilian.

## What's left

- The 0x43/0x44 weapon ids' identity (the other throwables) is inferred from the class table
  alone — worth a live check only if a render ever shows the wrong icon with them out.
- The MP teammate arm of `Check_Target` (class 3 / `MP_isObjectOnTeam`) is not modelled (the
  port is SP) — as before.

## Live checks for the main agent

1. PCSX2 07000002, the level start: with the P2K out, the civilian by the start shows the red
   no-shoot crosshair; switch to the fists (□) — the crosshair over the same civilian is the
   NORMAL one. (The port now does this; the pre-fix build showed the red mark with the fists.)
2. The same spot with a grenade equipped: the normal crosshair (the throw family escapes the
   mark).
3. Optional gdbwatch: BLData+0x133 during the fists-out aim — expect 2, and 3 with the gun.

## Render spots

- `07000002` level start (`--bond -27.17,1.05,29.80` yaw ≈ 0.93): the civilian ~3.5 units
  ahead — the red no-shoot mark with the gun; the normal crosshair with the fists.

## What I ran

- `cargo test --offline --release -p nf-view --test civilian_no_shoot_live` — 3/3.
- Blast radius on the changed crosshair rule: fix_player2_live 5/5, fix_player3_live 6/6.
- Full suite (`tools/test/suite.py`, the final tree merged with origin/master) — chunked over
  the tool-call limit; the total line below.

## ntfy

(pending)
