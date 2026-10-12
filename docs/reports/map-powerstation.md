# map-powerstation — the power station missions 0700000c/0700000d: maps, objectives, walkthroughs

Branch: **s/map-powerstation** (from origin/master d91f5bd, rebased onto 1276eb0 — fix-mission's
mission layer, map-castle and map-mayhew landed mid-task and my work now builds on their
`Mission::init`/`update`/`monitor` and the `c0700000c`/`c0700000d` records, whose [L] data matches my
own dump of `ee_start_05.bin` @0x2a4080/0x2a40c8 byte for byte). Docs: `docs/levels/0700000c.md`,
`docs/levels/0700000d.md`. Code: `crates/nf-game/src/props.rs` (new) + additive hooks in
`crates/nf-game/src/drone.rs` (the `Drones::props` field, the create in `attach_level`, the bullet
check in `update_player_bullets`), `crates/nf-game/src/lib.rs` (`pub mod props;`),
`crates/nf-game/src/player.rs` (the 0c/0d loadout cases), `crates/nf-formats/src/nis.rs` (the
light/sub-script ops decode), `crates/nf-view/src/scene.rs` (the script-less weapon fire gate — one
behavioural change, kept to 4 lines + a comment), `crates/nf-view/tests/portal_live.rs` (the sweep's
0c/0d arms). Tests: `crates/nf-view/tests/walk_0700000c_live.rs` (4), `walk_0700000d_live.rs` (3),
`crates/nf-game/src/props.rs` unit tests (2).

## Per-level verdicts

| Level | Completable (headless walkthrough) | The chain |
|---|---|---|
| 0700000c | **YES** | the loadout arms the sniper + the camera gadget [M 0x1a7ef8 case 0x700000c] → the ST-1 lower-walkway plate (#506, ch 0x1b) completes "Investigate building ST-1" → the camera gadget (0x54) fired into the jetpack display's `CamSubject` volume (#414/#454) writes ch 0x1c → "Photograph the Jetpack Prototype" → E at the gate #556 (USE-activated [M Player_Activate 0x1a5170 case 0x1d → Trigger_Activate 0x1b2318], key gate ch 0x1c) → **level_exit 0x0700000d** |
| 0700000d | **Up to objective 5 and the C5 corridor; NO for the last step** | the arrival start trigger (#436, ch 0x23) completes "Advance through Warehouse TR-2" → the ST-3 yard plate (#378, ch 0x1d) → the camera photograph of the laser display (#494/#495) → ch 0x1e → the C5 double door `bigdoor01/02` (0xE1 `Destroy`, ch 0x1f, health 1000/10000) must die to write ch 0x1f → the corridor plate (#433, ch 0x28) → the AND #434 [0x1e & 0x1f & 0x28] → ch 100 → 0x730000d. **The blocker: nothing on the disc's 0700000d data the port can reach writes ch 0x1f except the `Destroy` death itself** — no 0x38 `SpaceLaser` placement, no 0x67-weapon drone, and no 0x0D channel-write event in any of the level's 12 scripts or their sub-scripts (scanned after decoding past the light/sub-script ops). The mounted laser flies inside the crane cut-scene's scripted timeline (`shipping_handle` SS → `ScriptPlayer_ShippingCrane`), which the port does not simulate. |

## What was ported (each with its game address)

- **The photograph mechanic** (`crates/nf-game/src/props.rs`): `CamSubject_Create` 0x1bc5d8 /
  `CamSubject_Update` 0x1bc668 (a class-5 bullet with the 0x54 weapon byte entering the subject's
  volume writes its channel) and `Destroy_Create` 0x12c318 / `Destroy_CollisionHandler` 0x12bd48 /
  `Destroy_Update` 0x12c4f8 (bullets subtract their damage; death at `health < 0.0` writes the
  channel). The game's contact is the collision pass's list of objects overlapping the prop's glist;
  the port checks each bullet's flight segment against the prop's placed bounds box (grown 0.1 — the
  world surface often coincides with the box's near face).
- **The script-less fire gate** (`scene.rs`): the port's `has_fire_script` gate blocked the camera;
  the game has no such gate — `Player_RoundToFire` 0x1a18e0 rounds to fire unconditionally for
  ammo_type 0 [M], the press-tick spawn for `fire_anim_len == 0` is [M 0x1a5460:688-693]. The gate
  now stands in only for the hands class 0x47 (fix-ladder2's concern). Weapon 0x54's disc data
  backs this: no fire scripts (+0xB0/+0xB4 = 0), idle 0x060005A6, ammo_type 0 [D probe].
- **The level loadouts** (`player.rs`): case 0x700000c [M 0x1a7ef8:210-220] and case 0x700000d's
  cold fallthrough into it [M 0x1a7ef8:221-224] — the sniper selected, the camera gadget owned.
- **The NIS decoder** (`nf-formats/src/nis.rs`): ops 0x13-0x1e decode past (the `Script_Run`
  0x1d8548 byte lengths) and op 0x1b = `SubScript` (`Script_SubScriptStart` 0x1db050) — the level
  scripts' streams were halting at the first light/sub-script op.

## What was NOT ported (with the game function behind each)

| Item | Game function | Note |
|---|---|---|
| **0700000d: the C5 mounted-laser cut-scene** | the crane SimpleScripts (`SS_Create` 0x1a9c58, `SS_Activate` 0x1a9ea8, `SS_Update` 0x1a9948, `SS_Operate` 0x1a9df8), the script players #115/#492 (`SP_Update` 0x1dbda8), the laser's weapon-0x67 bullets | Blocks 0700000d's objective 6 and so the mission end (the AND needs ch 0x1f). The mission-complete path itself is verified at the mission layer's unit tests (fix-mission's + the mayhew chain pattern). |
| 0700000c: the vent/pulley route | the same SS family + `Lock_Create` 0x1ae1f8 / `Lock_Update` 0x1ae3d8 | Does not block: the walkthrough reaches the jetpack display from the ground floor. |
| 0700000d: `bigdoor01`'s hittable volume | — | The #93 object carries no gfx bounds and an empty collision block [D]; its leaf's box is unbuildable. `bigdoor02` (#96) parses fully. |
| The 0xE2 objects' playback on both levels | `SS_Update` 0x1a9948 | Shared with `120-fix-explosions` (the task's coordination note). |

## Spots the user should check in play (with `--bond x,y,z,yaw`)

- 0700000c start → the lower walkway: `--bond 5.8,13.2,-154.5,3.14` — walk −Z; the ST-1 plate at
  z −155..−153 completes objective 1. The stairs from the start walkway down to the lower walkway
  were not walked in the headless test (Bond teleports) — the descent's geometry is worth an eye.
- 0700000c the jetpack display: `--bond 5.4,7.3,-113.0,0` — equip the camera (T / the gadget wheel),
  aim at the display, fire: the objective should complete and the gate unlock.
- 0700000c the exit gate: `--bond 95.0,11.4,-209.7,1.57` — walk +X to the gate plane, press E.
- 0700000d arrival: `--bond 179.5,12.2,-205.4,2.86` — the start trigger should show "Advance through
  Warehouse TR-2" completing as the level opens.
- 0700000d the laser display: `--bond 137.0,5.1,-321.0,3.14` (pitch up ~16°) — the photograph.
- 0700000d the C5 yard: `--bond 133.0,8.5,-258.0,3.02` — in the real game the crane sequence cuts
  the door; in this build the door will not die. Everything past the door (the corridor plate, the
  AND, the mission end) is data-verified but not end-to-end.
- The SimpleScript props on both levels (the vent grates, the pulleys, `Door_secret1`,
  `shipping_handle`) draw but do not move — expected until `120-fix-explosions` lands the SS playback.

## Verification

- `walk_0700000c_live` / `walk_0700000d_live` drive the real input path (`nf_view::load[_with_carry]`
  + `Sim::step_bond` with the viewer's input mapping; E presses, the weapon switch, the aim, the
  trigger) and assert each objective stage in order, the level exit, and the sisters (the locked gate
  without the photograph; the record-12 channels cleared on the warm load; the AND open without the
  C5 death).
- Full suite: `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1` — green except the 3 allowed `disc_golden` failures
  (the slim ISO). `wire_live` and `portal_live` (the pre-existing suites my shared-file edits could
  have broken) are green; portal_live gained the game's 0c/0d cold-loadout arms.
- Two trigger-containment variants I tried for the corridor plate (the AABB-span overlap, then the
  tris' bounding sphere) fired 07000005's wire-area plate #291 under the fix-wire crossing rider and
  froze it — wire_live caught both; master's landed containment (with the wire-rider exclusion,
  c81361f) is the surviving approach and my tests pass on it.
