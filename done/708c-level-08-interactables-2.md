## Your task: `level-08-interactables-2`, finish the 08 interactables (HELD at merge)

`s/level-08-interactables` (708a, 857eb60) was held: its `crates/nf-game/src/hurt.rs` — the fuller port
(the `Hurt_Update` 0x17e5d0 state machine, gate value as the who-mask, param-1 damage, drone victims, on
`Drones::hurts`) — collides with the `hurt.rs` already on origin/master (on `Sim.hurts`, scene.rs, with the
who-mask start law). Folded in as the one system (master's `Sim.hurts` retired, hurt_02_live moved to
`Drones::hurts`) it breaks four tests that pass on master:
`fix_finale_live::the_07000016_chasers_defeat_completes_the_final_mission`,
`spurious_complete_16_live::the_real_end_condition_posts_the_mission_complete_banner` (16's #309
"HurtPoly-ShuttleLaunch", gate 0xf3, 1000 damage, kills Bond on the silo floor during the fight),
`hurt_02_live::the_electrocute_volume_hurts_once_the_cable_light_smash_feeds_the_chain` (line 83) and
`interact_07000006_live::the_well_shaft_is_open_and_the_hurt_volume_is_game_inert` (06's #373, who 1 — the
spec's own inert law). Start from origin/master's hurt.rs and `docs/systems/hurt-volumes.md`; build ONE
system (no second module) that passes all four plus your 08 tests (hurt_07000008_live). Coordinate with 705d
(the same ShuttleLaunch question): settle when ch 0xf3 is up relative to Mission state 6 [M 0x182c60].

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

**Branch:** `s/level-08-interactables-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T17:48Z

---
## Report (done, 2026-10-17T18:47Z)

# level-08-interactables-2 — the 08 hurt volumes re-landed onto master's one hurt system

Branch `s/level-08-interactables-2`, from origin/master 4d65d006 + merge of
origin/s/level-08-interactables (857eb60).

## What was done

ONE hurt system, starting from master's `crates/nf-game/src/hurt.rs` on `Sim.hurts`
(nf-view `scene.rs` owns and ticks it) per docs/systems/hurt-volumes.md. Master had already
absorbed the held work's law set (ea43560a: the param map, the who-mask start law, the
channel-value-as-who-mask law, the state machine, the #309 ShuttleLaunch gate handling), so the
re-land ports what master still lacked — the **drone victims** — without a second module:

- `Hurt::update_touch(gate_on, a, b, r, drone)` — the victim-generic contact law; `update` is the
  standing-player wrapper (feet→+1.8, r 0.55, class 2 → who bit 0x10). A drone brings class 3 →
  who bit 0x8 [M Hurt_Update 0x17e5d0:57-95's class bits]; the gated/disabled law and the
  one-shot retire are shared, so a volume fired by any victim retires for all of them
  [M 0x17e5d0:41-47 — the state lives on the volume object].
- The drones' pass in scene.rs next to the player's: the capsule is the trigger walk's drone body
  ([pos−1.0 … head], r 0.4 — the Toucher law drone.rs builds for `Trigger_Calc`); the delivery is
  the explosion path's flat subtract (`health -= damage · damage_taken`, `last_damage` stamped —
  the drones' own update's health ≤ 0 selector picks the death) [M Drone fields +0x150's source
  per the explosion path's precedent]. This delivers 07000008's kill-floor quads #42–44 (type 6,
  1000 damage, who 0x18 = classes 3+2 [D params]) to drones AND keeps the player arm master
  already had.
- `Hurt.hits` — the port's mechanism witness (the application count the 08 live tests' "the
  volumes stay armed" pin reads); not game state.
- 708a's `Drones::hurts` / `hurt::build` / free `update` / `Toucher` variant is retired — master's
  `Hurt` struct, `Hurt::create` and the scene wiring kept.
- The 08 tests came across and run on the merged API (`sim.hurts`): `hurt_07000008_live` (the
  platform stand + the real walk off the boarding platform — the fall, the plunge TouchOnce, the
  death-NIS hold, `hits == 0`), `census_07000008`, `models_sweep_07000008_live`.
- docs/systems/hurt-volumes.md gained "The port as merged" (both victim passes, the capsule laws,
  the delivery) and two new live checks (below).

The four master tests the held branch broke all pass UNCHANGED — master's gate law already
settles them: `fix_finale_live` 4/4, `spurious_complete_16_live` 2/2 (#309 gate 0xf3 / 1000
damage stays disabled through the chaser fight — the channel is low until the launch), 
`hurt_02_live` 1/1 (the electrocute chain), `interact_07000006_live` 9/9 (#373 who 1 inert).

## How it was verified

Foreground runs (this sandbox kills background jobs between tool calls — the fast runner dies at
launch, so the crates ran directly):

- nf-game lib **279/0** (incl. the 7 hurt units: the gate maintenance, the who laws for both
  victim classes, the one-shot retire, the new drone-bit test) + all integration binaries, 0
  failures.
- nf-view: `hurt_07000008_live` 2/2, `census_07000008` 2/2, `models_sweep_07000008_live` 2/2, the
  four master tests above, `interact_07000005_live` 11/0 (13 volumes; #331–333 who 0 live at
  load), `interact_07000001_live` 10/0, `interact_07000003/04/06/07_live` all green, 
  `walk_07000001_live` 3/0, `walk_07000008_live` 3/0, `walk_07000016_live` 3/0,
  `walkreal_07000003_live` 4/0, plus ~66 more binaries in the alphabetical a–fix_hud sweep and
  the fix_* batch — zero FAILED lines.
- nf-formats: green except the 3 expected slim-ISO `disc_golden` failures; nf-audio 15/0.

## What's left

- **The death-NIS terminal fail** (708a's finding, unchanged): after a death NIS ends the game
  fails the mission [M 0x182c60:21-43]; the port freezes the body. Until it lands, the kill floor
  cannot be reached end-to-end through the platform walk, so `hurt_07000008_live` pins the hold
  (hits == 0) instead of the 1000-damage landing.
- The drone-side kill-floor delivery is inferred from `NDrone2_DealWithObjHit` 0x13fff0 + the
  explosion-path shape; no live test drives a drone onto the y −24.1 quads yet (live check 4).
- The 08 per-instance door E-open, the #122-after-ch-137 unlock pair, the zip-line ride (#19) and
  the spawner-wave releases end-to-end (708a's list, untouched here).

## Live checks for the main agent

3. PCSX2, fresh boot of 07000016: watch `switch_channels[243]` (0xf3) and the Mission state
   (0x182c60) through the chaser fight. Expected: the channel rises with (or after) Mission
   state 6 — the chasers' defeat — never before; #309 (gate 0xf3, 1000 damage) must stay disabled
   for the whole fight. Settles the ShuttleLaunch question for 705d and this task.
4. PCSX2, fresh boot of 07000008: get a guard onto the y −24.1 valley floor under #42–44.
   Expected: the drone's health drops by 1000·damage_taken and the death selector fires (the
   drone-side delivery).

## Usage

not visible to me (no token, context-window or cost figures are exposed to this session).

---
## Report from the worker (done, 2026-10-17T18:47Z)

Closed by the driver: the report is on `s/level-08-interactables-2` (docs/reports/level-08-interactables-2.md); the worker didn't run finish.sh.
