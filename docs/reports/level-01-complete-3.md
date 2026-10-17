# level-01-complete-3 — the 01 full walk, held again at the escort test

Branch `s/level-01-complete-3` = the held `s/level-01-complete-2` (a141125d) merged with origin/master
d56aceb6 (the only conflict was `docs/WORKLOG.md`, resolved keeping both sides). **Status: blocked** —
the merge reproduces the hold exactly; I localized the mover to ONE sub-change and proved the true fix
path, but could not make both tests green inside the window.

## What I ran and what passes/fails on this tree

- `escort_follow_07000001_live::escort_at_walking_speed_keeps_the_games_distance_and_the_exit_opens`
  — **FAILED** at line 289 (the AND plate ch 0x10 never fires; verified twice with `--nocapture`).
- `walkreal_07000001_live` — **3/3 PASSED** (the full walk incl. `walk_from_the_spawn_to_the_exit_with_the_contact`).
- The full `suite.py` was started in the background; the window ended before a total line existed —
  the two binaries above are the verification (this task touches only `drone.rs`'s merge state).

## The bisection (all four states verified locally)

| armed-peer scan (drone.rs ~10656) | escort | walk |
|---|---|---|
| master's simple filter (team/started/enabled/¬corpse) + prop-0x50 gate kept | **PASS** | FAIL (the contact stalls at (7.0, 6.3, 9.5), state 0x1e PlayerInWay, route idx 15/37, armed_peer = MAX) |
| the held hardening (health>0 + obj_armed + ≤20m + head-to-head LOS) — the merged tree | FAIL (the contact parks at (−30.6, 6.3, 13.1), 1.8 east of the AND box) | **PASS** |
| the hardening minus the weapon gate (health + 20m + LOS only) | FAIL | not reached |
| the hardening minus the weapon gate, sight ray raised pos+1.5 (a bone-5 stand-in) | FAIL WORSE (12-round cap at line 274) | — |

**The mover is the armed-peer scan hardening** — specifically its head-to-head **LOS gate**. The
prop-0x50 hide-arm gate [M `NDrone2_NearArmedDrone` 0x177420:57-61,
`behaviour_util_getProperty` 0x1c7d98] is innocent: it never changed either test's outcome.

## Why the LOS gate breaks the escort (the decompile chain)

1. The escort's contact rides **LEAD_HIDE** (state 0x1f) from route idx ~16 to the stairwell: with the
   scan unfiltered, a LIVING armed guard 3.9 away counts as the peer and the hide's run-to points walk
   him west along the route to (−28.3, 6.3, 14.3); the AND plate (placement #180: Touch out 0x10,
   mask 0x18, filter player 0xE + skin 0x4F = Mayhew, box [−33.80, 0.64, 12.17]–[−32.41, 3.96, 18.68]
   [D triggers dump]) latches and the exit opens.
2. The hardened scan excludes that peer — `world.line_of_sight` (a raw wall ray,
   `world.rs:1252 = segment(a,b).is_none()`) says the Mayhew↔guard-2 head-to-head ray is blocked
   (`los false` for every candidate in a peer-logged run; guard 2 at (−29.7, 10.2, 13.7) is on the
   landing above the stairwell).
3. The game's sight test is **not** a raw wall ray: `DroneVision_CanSeeObjectFrom` 0x176f18:56-104
   builds the bone-5 head (`DroneFunc_GetBonePos` + `DroneFunc_ClampPosToCollisionCylinder`), then
   `if (same AI cel || AINetwork_TestRayCels(...))` runs **`Collide_LineOfSight(..., flags 0xa27)`** —
   a MASKED collision ray (0xa27). The port's unmasked `segment` hits the stairwell geometry the
   game's 0xa27 mask ignores, so the port is STRICTER than the game exactly where the escort's peer
   sits. The same strictness is what FIXES the walk (the phantom balcony guard 13.7 away, whose ray
   crosses real walls, must not block the hide's >14.0 exit — the held comment).

## New decompile finding for the next session

**The held branch's weapon gate is misapplied.** Its filter used `obj_armed()` (the byte ∈
[2..0x46] ∪ [0x6a..0x71] of `NDrone2_ObjectIsArmed` 0x178768). The disassembly of the actual peer
test (`capstone`, little-endian MIPS, ACTION.ELF):

- 0x17752c-0x177578 (NearArmedDrone): a NULL weapon record at obj+0xdc **qualifies**; otherwise the
  byte at record+0x62 qualifies **only when == 0x47** — `sltiu (b−0x6a) < 8` excludes [0x6a..0x71],
  `slti b < 2` excludes 0/1, `beq b, 0x47` passes exactly 0x47, `slti b < 0x48` excludes [2..0x46].
- 0x177640 (NotNearArmedDrone) repeats the same shape, and its exit rule is: the candidate equal to
  the remembered +0x2b8 peer returns "near" WITHOUT the sight test; other candidates need the sight
  test [M 0x177640:94-115]. The port's single shared scan approximates this only when the sight
  stand-in matches.

So byte +0x62 of the weapon record is not the plain weapon id (or the peer test intentionally accepts
only 0x47 there) — the held branch's `obj_armed()` filter is wrong for BOTH scans and should be
dropped or re-derived; it is not the binding constraint either way (dropping it alone did not fix the
escort).

## The fix path (next session)

Port the masked sight ray: `Collide_LineOfSight` 0x-flags 0xa27's cel-class exclusions into the
port's `CollisionWorld::segment` (or add a `segment_masked`), then the existing hardening
(health>0 + ≤20m + masked-LOS) should satisfy both tests for the game's reasons. The walk needs the
mask (the phantom dies), the escort needs the mask (guard 2 lives).

## Live checks for the main agent

1. `gdbwatch.py` on `Collide_LineOfSight` (0x1e04a0-ish entry — resolve via action-symtab) during the
   01 escort's stairwell approach: record the mask argument (expect 0xa27) and the return value for
   the Mayhew→guard-2 pair (Mayhew at (−28.3, 6.3, 14.3), guard at (−29.7, 10.2, 13.7)) — expect
   "see = true" in the game where the port's raw ray returns blocked.
2. `fieldprobe.py`: the AND plate #180's two hit latches (Trigger_Update case 10, 0x1b1a78) — do the
   player hit and the skin-0x4F drone hit need to be simultaneous, or do they latch independently
   (the port's tests currently pass either way at different ticks)?
3. Fresh boot, 01: Mayhew's `Drone+0x4d8` behaviour props — does prop 0x50 read 1 (the hide arms are
   live for him per 0x177420:57-61)?
4. The weapon byte at record+0x62 for a standard guard — is it really the weapon id (0x47 = ?), or a
   raised/aimed state byte (the peer test accepts only 0x47, per the disasm above)?

## Left

- The escort test still red on the merged tree (the LOS-mask port above); the walk green.
- The weapon-gate correction (drop/re-derive per the disasm) rides with the mask work.
- The suite total line: not reached in the window (background run killed with the sandbox churn).

## Usage

not visible to me
