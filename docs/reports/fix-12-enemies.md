# fix-12-enemies — the 07000012 guard-damage audit

Task 292: on 07000012 the playtest saw "There were some killable MP, unkillable NPCs, like
enemies" — some characters die, others never take damage. Branch `s/fix-12-enemies`.

**Result: the audit found NO port-side damage bug on this level.** Every guard class that can be
reached through the real input path takes damage and dies; the port's damage chain matches the
decompile at every point the task named. What could NOT be reproduced (with everything that was
tried) and the live checks that would settle the user's report are listed below. No code change
landed — per the reproduce-before-fix rule, nothing was guessed.

## 1. What the port does on 07000012 (verified against the disc)

- 32 drones load: skins `0x05000006` (the kind-3 captains, hp 20), `0x05000008` (guards), kind-21
  actors with skins `0x0500000e`/`0x0500000f`. Most guards sit in **WaitSwitch (0x01)** on their
  wing touch-plate channel 0x01–0x09; 5 spawners (0xF1) hold reinforcement templates — spawner 1
  (group 2) and spawner 3 (group 20) fill immediately at load (drone 0 visible near the start;
  drone 21 waits on ch 0x9). Test `dump_every_drone_at_load` prints the whole table.
- Waiting guards are **disabled and hidden** — `NDrone2_Enable(0)` sets `obj+0xF0 |= 0x10|0x20`
  [M 0x149dd0:8-15], `Collide_PickObj` drops `obj+0xF0 & 0x20` objects from bullet picking
  [M 0x1eae80], and the renderer hides `!present || !enabled` drones (the flags-0x30 rule, scene.rs
  "Held back"). Their channel-wake path re-enables them [M `NDrone2_DSTATE_WaitSwitch` 0x15b960
  case 3 → `NDrone2_Enable(1)`; port `(WAIT_SWITCH, msg::UPDATE)`].
- The damage chain matches the decompile: body-part multipliers, the armour bits `+0xbb`
  (0x8 Combat / 0x2 Jacket / 0x1 Vest / 0x4 Helmet) and the difficulty factors are
  `NDrone2_HitDamage` 0x145558; the `+0x14` damageable gate is `NDrone2_DefaultInit` 0x14b300:204-207
  (only skins 0x0500003b/3c/53 start 0 — none on this level); health `+0xac`, the last-hit record
  `+0x150`. The port's `damage::hit_damage` mirrors all of it (docs/systems/drone-damage.md).

## 2. Every guard class fired at through the real input path

Test: `crates/nf-view/tests/enemies_07000012_live.rs` (`NF_ISO` opt-in), harness `Walker` +
`Sim::step_bond` — the real fire path (the gun, the bullets, the hit boxes, the trigger touch).

- **Load-enabled guards** (drones 8/9 by the start, drone 0 the immediate spawn): Walker::shoot
  killed drone 0 in 2 hits (hp 10 → −0.5, corpse), drone 8 died in the crossfire (1 hit), drone 9
  took 3 hits (hp 20 → 4.2) before Bond's P2K ran dry — damage lands on every one.
- **A woken placement guard**: the plate #28 touch (the box centre stand-in, out ch 0x6 — the real
  `Trigger_Update` 0x1b1a78 path) wakes drones 3/10. The clean-shot probe killed drone 3 in 2 taps.
- **A spawner-spawned reinforcement**: spawner 2 (trig 0x6) spawned template drone 20 on the same
  ch 0x6; the clean-shot probe hit it on the first tap (hp 10 → 4.8, then dead).
- **Hit boxes**: 16 boxes per guard from `HumanCollisionBones` [M ACTION.ELF 0x2c6be8], posed with
  the live animation; no present living drone has an empty set (asserted).
- `every_woken_and_spawned_guard_takes_damage` runs the last two as assertions; all 3 tests green.

The clean-shot probe only taps when the torso-box hit is closer than the world hit — early audit
rounds that fired through office walls correctly recorded zero damage, which is the game's wall
behaviour (`Bullet_CollisionHandler`'s world-vs-object order), not a bug.

## 3. What was tried for "unkillable NPCs" and what it ruled out

- **Protected/invulnerable flags**: `+0x4F8` census (docs/fields/drone.md) — 0x200 (corpse) and
  the 0x100/0x20 clear set by Enable(0) are the only damage-relevant bits; the port models both.
  No "protected" bit exists on this level's drones.
- **HITDATA multipliers**: `NDrone2_HitDamage` 0x145558's part/armour/difficulty chain — ported
  bit-exact including the 0x07000014 double-torso and 0x0700001b head-only arms.
- **Spawner templates not in the hit list**: the port holds templates `present=false` until
  `DroneSpawner_Control` 0x137e68 spawns them (then present+enabled, appended to the llist) —
  and a spawned template dies on the first clean tap (above).
- **Script actors drawn but not in the hit list**: 23 script objects survive the 520-tick skybridge
  intro (SP #660's 20 created entities among them, kind 32/49), but only ONE character instance is
  still script-driven afterwards (char 77, the Q-Worm loop SP #765's actor, at the origin) —
  asserted ≤ 1 in `script_objects_after_the_intro`. The intro's actors are not drawn after the
  intro, so they cannot be the ghosts the user shot.
- **Skin-lookup failures** (`for_skin` has only the human and 0x0500_0078 astronaut sets): every
  skin on this level uses the human set with a complete pose — asserted non-empty.

## 4. What I could not do, and the live checks for the main agent

I could not walk the whole level as the player did (the walker's route crosses the middle; the
waking plates sit in the side rooms), and nf-view has no GPU here — anything visible needs the
main agent's render. The user's "killable MP" phrasing never mapped to a port object I could find:
the killable guards are SP guards; there are no MP bots on this level. The remaining suspects need
eyes on the real game:

**Live checks (PCSX2):**
1. On 07000012, stand at the start and shoot the two office guards by the elevator (the load-enabled
   pair) — they must die. `nf-view --bond 22.5,0.2,19.2,3.14,0` should show the same pair alive.
2. Walk into a side office WITHOUT touching its wake plate and look for a standing guard: in the
   game the waiting guards are INVISIBLE (obj+0xF0 flags 0x30). If the game shows them, the port's
   hide is wrong; if it hides them too, the user's "unkillable NPC" was the wall pass-through or a
   not-yet-woken guard. gdbwatch on `obj+0xF0` of one waiting drone: expect `|0x30` while waiting,
   cleared when the plate fires.
3. Fire at one of the intro's skybridge actors DURING the cut-scene: in the game these are script
   entities without a drone record — expect hit effects but no death. If the game lets them die,
   the port's script-entity collision needs the object walk, and I missed it.
4. After the intro, walk the offices and confirm no frozen guard meshes stand anywhere (the port
   asserts ≤ 1 script-driven character).

**nf-view (`--bond x,y,z,yaw,pitch`):**
- `22.5,0.2,19.2,3.14,0` — the start: two office guards ahead (killable).
- `25.9,1.1,0.8,0,0` — on plate #28: the wing pair (drones 3/10) visible after the touch; the
  spawned reinforcement (drone 20) at the security-centre door.
- `14.0,1.0,-1.9,1.57,0` — the security centre: spawner-3's drone 21 hidden until ch 0x9.

## 5. Suite

The full suite on the pre-merge tree: **1223 passed, 6 failed, 66 ignored** — the failures are the
3 allowed `disc_golden` tests plus 3 pylon-render probes (another session's 0700000c territory,
none in drone damage). The final tree (merged with origin/master 1288f0d6) differs from it only by
this task's test file and report; the targeted suites on it (`spawner_kill`, the `damage` unit
tests, this file's 3 tests) are green. The background full-suite run was still going at report
time; if it turns up anything new it lands on the branch as a follow-up commit.

## 6. What's left

The user's symptom is not yet tied to a port defect. Next session: drive the walkthrough
(`walkreal_07000012_live`) while periodically ray-checking every VISIBLE guard with the clean-shot
probe (a "shoot everything you can see" sweep), and work through the live checks above — the
game-side answers decide whether the port even has a bug here.
