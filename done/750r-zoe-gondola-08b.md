## Your task: `zoe-gondola-08b`, finish zoe-gondola-08 (HELD at merge)

`s/zoe-gondola-08` (task 750d) was held at the 2026-10-16 batch merge: on top of the batch it breaks three tests
that pass without it (each fails alone, bisected to this merge):
- `nf-view` `cover_nodes_live::a_06_guard_under_fire_takes_a_game_cover_node` — "no guard entered the cover
  states under fire";
- `nf-view` `npc_doors_live::the_escort_opens_his_door_without_bonds_help` — "the escort never entered
  DSTATE_OpenDoor at the courtyard shoji";
- `nf-view` `rocket_origin_live::the_rocket_leaves_the_launcher_not_the_eye` — "aimed at the aiming point,
  missed by 4.28".
Its fix must be scoped to what the game does for Zoe (her team/target rule) without changing the other drones'
targeting, cover or door behaviour, nor Bond's aim. Start from `origin/s/zoe-gondola-08` merged with the newest
origin/master; make those three pass alongside your own test; never loosen a decompile-backed test.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/zoe-gondola-08b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T04:33Z

---
## Report (done, 2026-10-17T05:28Z)


---
## Report from the worker (done, 2026-10-17T05:28Z)

# zoe-gondola-08b — finish the held zoe-gondola-08 (750d)

The held branch broke three tests on top of the 2026-10-16 batch. Branch `s/zoe-gondola-08b`
rebuilt the held content (revert of the revert 6445bd0f on origin/master 50d51ae7 —
`origin/s/zoe-gondola-08`'s commits are already ancestors of master but their content was
reverted there) and reproduced all three failures before changing anything.

## What's done

**1. cover_nodes_live `a_06_guard_under_fire_takes_a_game_cover_node` — FIXED (passes).**

The held task's `look_for_opponent` gate is the game's own [M 0x177ed0:12-14 —
`*(short *)(SMI+4) == 3 && 1.0 <= Drone+0x50c → return 0`; SMI+4 is the drone's state id (the same
word the function later compares to 0x63/0x85), and port state 0x03 IS PLAY_SCRIPT]. But the port's
only exit from a fully-alert PLAY_SCRIPT was the msg-3 look — master's ungated approximation of
`NDrone2_DSTATE_PlayScript` case 3. The gate closed that exit and the a_06 guard sat in his script
for the whole 1200-tick window (NF_AILOG shows the a_06 guards at alert 1.0–2.6 held in 0x3), so
nothing entered the cover states.

The game's real exit is the sound messages [M 0x15ba68 cases 0x11/0x12/0x14/0x15/0x16/0x1e →
`DroneVision_EnemyAlerts` 0x177c18 → `Drone_SM_SetState(result)`]: a gunshot (msg 0x14 →
AlertSound 0x177800) pulls the script-playing guard into the heard-noise chain, from where the
existing (ungated) look paths drive combat → `RunForCover` 0x8C. The port had NO PLAY_SCRIPT sound
arm. Added two arms (`crates/nf-game/src/drone.rs`):

- `(PLAY_SCRIPT, msg::SOUND)` → `enemy_alerts` [M 0x15ba68 case 0x14];
- `(PLAY_SCRIPT, msg::ALARM)` → the same route the IDLE/ALERT/PATROL arm takes [M 0x15ba68 case
  0x1e → 0x177c18 case 0x1e → 0x56 ATTACK].

SCREAM/SHOUT already route through the broadcast pre-dispatch [M 0x177c18 cases 0x11/0x12]. The
PLAY_SCRIPT look gate stays — it is the decompile's own line. a_06 passes again through the game's
own path.

**2. rocket_origin_live `the_rocket_leaves_the_launcher_not_the_eye` — FIXED (passes).**

Not an aim bug and not Bond's aim: the shared-RNG draw order. Under master's code Zoe (team 2,
07000008) sighted Bond and burned hit-roll/wobble/`next_bullet_time` draws in her phantom combat
chain; the held team-2 gates [M FindOpponent 0x142e0c — Bond is never stored in `+0x170` for a
team-2 ally] removed those draws, the player rocket's `bullet::spread` roll changed, and the
sister assertion's magic 4.0 — tighter than the record's own cone — measured 4.28.

The sister bound is now the record's own spread envelope: the game's cone offset is
`mul(frand(2·s) − s, 0.0014)` with s = weapon record+0x24 [M Bullet_init 0x12c5a0's spread cone /
Vec_Spherical_2_Cartesian 0x1e3948 — the port's `bullet::spread`], so the max lateral miss at 256
units is `256·s·0.0014` (+ 0.05 normalisation slop). The first assertion (the rocket leaves the
launcher, not the eye) is untouched. The bound is derived from [D] record bytes read from
`WeaponData`, not a new guess.

**3. npc_doors_live `the_escort_opens_his_door_without_bonds_help` — NOT FIXED (still fails).**

Deterministic failure, bisected (see below). With the branch's `drone.rs` the escort cowers
(LEAD_HIDE) at spawn on `armed_peer_dist` 7.2 and the harness cannot clear it ("0x81 at None,
opened true, crossed false"). With master's `drone.rs` on this same branch it passes in 30 s.

**The Zoe rule itself is kept:** all four team-2 gates from the held task are unchanged
(`look_for_opponent`/`fire`/`handle_firing`/`fire_weapon`), Zoe's own two tests pass
(`zoe_gondola_08_live` 2/2), and the PLAY_SCRIPT gate stays — it is decompile-backed.

## How the escort was bisected

- Branch drone.rs → FAIL (identical tick-for-tick across runs; the park is deterministic).
- `git checkout origin/master -- crates/nf-game/src/drone.rs` on this branch → PASS in 30 s.
- Disable ONLY the PLAY_SCRIPT look gate (team-2 gates kept) → PASS in 29.6 s.

So the trigger is the gate holding the courtyard guards (PLAY_SCRIPT, placement alertness 1) in
their scripts: under master's ungated msg-3 look they sight Bond during the intro and leave their
posts; with the game's gate they stay parked and cower the escort. Tried and REVERTED (unproven,
did not fix): porting `NDrone2_NearArmedDrone` 0x177420's candidate gates into the armed-peer scan
— health > 0 [M Drone+0xac], a weapon record present [M the +0xdc/+0x62 test; the exact band is
mangled in the C], the requester-sees-peer line-of-sight [M 0x177420 tail's
`DroneVision_CanSeeObjectFrom`], and the requester prop-0x50 gate [M 0x177420:6-8 — AllyLead's
case 3 passes flag 1, 0x15e698:30]. None un-parked the escort, so the scan is back at master's
exact version; the only drone.rs changes on the branch are the two sound arms.

The unresolved decode, for the next pass: `NearArmedDrone`'s candidate mask
`(Drone+0x4f8 & 0x600) == 0 && (Drone+0x4f8 & 0x100) != 0` (bit 0x100 also gates
`Drone_GetOpponentInfo` 0x138f68:16; its writer is unfound in the decompile), the candidate mode
`obj+0xff != 0x11` and `obj+0xfe & 1` tests, and DSTATE_PlayScript's msg-3 `+0xcdc` flag [M
0x15ba68 case 3 — the look runs only while it is non-zero, and the 07000001 scripts
0x6000168/0x6000169 set it to 0 in case 1]. Any of these may be the game's reason the escort walks
past parked scripted guards.

## What was run

- The three held tests + Zoe's two, on the final tree: cover_nodes_live 1 passed / 2 ignored (the
  two pre-existing [S] ignores from batch-1016c), rocket_origin_live 1 passed,
  zoe_gondola_08_live 2 passed, npc_doors_live escort 1 FAILED (above).
- fix_actors_live 4/4 (the escort-cower neighbour).
- Reproduce runs before each fix are in the report text above.
- The fast runner (`tools/test/suite.py`) was started three times (plain nohup, setsid) and was
  reaped between tool calls every time with an empty log — the same as the held task's report
  records for this sandbox. **No suite total line is available**; the foreground binary runs above
  stand in. The crates my tests touch: nf-game, nf-view.

## What's left

- The escort test (above): decode `NearArmedDrone`'s candidate flags and/or PlayScript's msg-3
  `+0xcdc`, then un-park the escort the game's way. The two sound arms and the rocket bound stay;
  nothing on the branch weakens a decompile-backed test.
- The nf-game/nf-formats/nf-audio portions of the suite were not re-run in full (no background
  execution in this sandbox; the foreground runs above cover the touched binaries).

## Live checks for the main agent

1. Fresh boot 07000001, let the intro run and Mayhew start leading: watch the courtyard guards'
   state ids (SMI+4). Expected if the game holds them: they stay in state 3 while Bond walks the
   route. Expected if they leave: the msg id that moved them (a sound 0x14 or the msg-3 look).
2. Same session: Mayhew's `Drone+0x44` (team, expect 2) and whether Mayhew's placement has
   behaviour prop 0x50 (the NearArmedDrone requester gate) — read `behaviour_util_getProperty`
   arg at his creation or poke prop 0x50 and watch the cower.
3. Same session: Mayhew's armed-peer distance at the courtyard (Drone+0x2b8 set by
   NearArmedDrone 0x177420) — if the game reports "none" while a guard stands within 10, the
   candidate mask (0x4f8 & 0x600 / & 0x100) is the discriminator; dump the parked guard's
   `Drone+0x4f8` word (expect bit 0x100 clear for a scripted guard, or 0x600 set).

## Usage

- Session token/credit/cost figures: **not visible to me** — the environment exposes no usage
  display for this session.
- Context window: not visible to me.
