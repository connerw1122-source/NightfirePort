# Doors — the collide path (fix-door-react-path, G-035)

The door state machine, activation and groups live in `docs/atlas/door.md` (authoritative).
This doc covers what G-035 asked for: who calls `Door_ReactToHit`, with what object pairs, and
when in the frame — plus the port's mapping.

## The game's call structure [M]

1. **Frame position.** `Collide_Update` 0x1ea398 runs after the control walk and the movement
   pass (docs/atlas/object.md steps 6→7): every object has moved and `Control_BuildWorldSph`
   0x133070 has rebuilt `obj+0x80`/`+0x8C` before the pick reads them.
2. **The contact build.** `Collide_Update` first clears every object's `obj+0xD0` list back to
   the pool (0x1ea398:25-53). For each object with a HITTEST (`obj+0xD4`) whose `+0x94` mode is
   set, `Collide_Pick` 0x1eb320 runs: straddled cels (`Collide_StraddleCels` 0x1eac98), then the
   **global `ForcedList`** 0x270590 (0x1eb320:369-375, unless `HITTEST+0x98` bit 0x10) — the
   objects `build_LinkToRoom` 0x1c9680:63-66 put there for carrying `obj+0xF0 & 0x20000000`
   (and anything straddling ≥2 cels, `Controls_StraddleTest` 0x134778:48-52). Doors are picked
   through this list: the player's pick mask `HITTEST+0x98 = 0x1c0` (written by
   `Player_Collision` 0x194df0's arms, e.g. 0x194df0:382) skips the per-cel branch (0x80) and
   the cel world-node (0x40) and excludes classes 2/0x11 (0x100) — the ForcedList is the only
   object path left, which is what makes every door pickable everywhere [M + the design
   inference; the 0x20000000 bit on a live door obj+0xF0 is live check #1].
3. **The pick test** — `Collide_PickObj` 0x1eae80, mode word `HITTEST+0x94`. The player's is
   **0x800** (`*(undefined2 *)(iVar2 + 0x7d4) = 0x800`, 0x194df0:204/229/257/315 — the walk,
   swim, grapple and default arms): the segment-vs-sphere sweep [M 0x1eae80:152-168] —
   `sqdist(obj+0x80 centre, closest point on the pick segment `HITTEST+0x20..+0x30`, t clamped
   to [0,1]) < (1.5·r_obj + r_pick)²`, the ×1.5 at 0x1eae80:160, `r_pick = HITTEST+0x8C`
   (0x7cc: 0.275 walk / 0.55 the other arms). The pick segment is the per-substate vertical
   capsule at the root (walk 0/6: A = pos+up·0.55, B = pos+up·(0.275−cc); swim 3:
   0.1−(cc−0.55) / 0.45−cc; default: 0.275 / 0.55−cc — the port's `Ladder::pick_hit` arms).
4. **The forward.** Each hit node on the picker's `obj+0xD0` (entry `+0x58` = the hit object)
   is copied onto the HIT OBJECT's `obj+0xD0` with the copy's `+0x58` = the picker [M
   0x1ea398:160-233]. That is the proxy-list path U-041 asked about: the door never picks; it
   is picked.
5. **The dispatch.** Still inside `Collide_Update`, `control_funcs[class·0xC+4](obj)` runs per
   object (0x1ea398:59-64). Class 0x1C's collide slot is `Door_ReactToHit` 0x135980
   (`control_funcs[0x1C]` = {0x1363c8, 0x135980, 0} @0x26f9e0 [ELF, trigger-port-check #3]).
6. **`Door_ReactToHit`** (full semantics: docs/atlas/door.md "Door_ReactToHit"): gate
   `ext+0x6C == 0`; selection walks the contacts in order — class 3 (the player) qualifies
   outright, class 2 (a drone) only through `(ext+0x4F8 & 0x600) == 0 && (ext+0x4F8 & 0x100) !=
   0 && ext+0xAC > 0 && !(obj+0xFE & 1)` (0x135980:47-72; the same combo `HUD_RadarUpdate`
   0x19f6e8:189-195 reads; `ext+0xAC` is health — subtracted by `NDrone2_DealWithObjHit`
   0x13fff0:316-333); kind-0 reversal by the swing plane (`f = local.z·ext+0x5C`, negated past
   half travel, ±0.0002 thresholds), kind-2 reversal while CLOSING, kind 1 nothing; tail
   `Door_Interp(…,0)`, `ext+0x6C = 2`, busy (0x135980:109-111).
7. **Busy/timer timing.** `Doors_Calc` 0x135fe0 runs at the HEAD of the next frame's movement
   handler (0x1336f8:23) and clears every busy bit, so a reversal's busy never gates a
   `Door_Update`; the 2-tick timer `ext+0x6C` (decremented by `Door_Update` 0x1363c8:22-23) is
   what spaces reversals out.

## The port's mapping (fix-door-react-path)

- `Doors::collide_step` (door.rs) = steps 2-6: it clears the per-door contact lists, tests
  every mover × every door with the 0x800 segment-vs-sphere×1.5 test against the LIVE sphere
  (`transform(matrix, sph_centre)`, `sph.1`), then runs `react_to_hit` (the 0x135980 selection,
  reversal and tail) per door with contacts, returning the `Door_SetState` sounds.
- Movers (built in the collide caller in `Drones::tick`, drone.rs, after the control walk,
  before `Doors::calc`): the player with the per-substate capsule the port already builds for
  `drone::Player::capsule` [M 0x194df0; the radius arm is recovered from the capsule's top
  offset], alive only (class 0x12 is skipped by the selection [M]); then every present drone as
  a point capsule at its root with the 0.4 DoorActor radius **[S — the drone HITTEST is
  unported]**, valid = present ∧ non-corpse ∧ health > 0 **[S — stands in for the
  `+0x4F8`/`+0xAC`/`+0xFE` gate]**.
- Ordering: before `Doors::calc` so the busy marker the tail sets clears the same tick — the
  game's calc clears it before any `Door_Update` reads it (step 7), the port's calc sits at the
  tick's end (its placement is prior tasks'), so before-calc reproduces the game's observable.

## Live checks for the main agent

1. On 07000005 or 01, walk a door obj (TheDoors list → obj) and read `obj+0xF0`: expect bit
   `0x20000000` set (the ForcedList membership the pick relies on) [M build_LinkToRoom:63-66].
2. Watch a door `obj+0xD0` while Bond walks into the closed doorway: expect it non-NULL for the
   ticks Bond's capsule is within ~2.6 of `obj+0x80`, with `entry+0x58` = the player obj; and
   `obj+0xD0 == 0` again when he steps 3+ away.
3. On a closing shoji with a guard walking in, watch `droneExt+0x4F8` bits 0x100/0x600 and
   `ext+0xAC` the moment the door reverses — pins the port's valid-gate stand-in.
4. nf-view `--bond` spots: stand 0.5 in front of a closed 07000001 shoji, E to open, E again —
   the door must re-open (state 8→4) with Bond there, and close only once he steps out of the
   leaf's pick zone; a patrolling guard in the doorway holds it open the same way.
