## Your task: `p2k-fire-07`, the P2K never fires on 07000007 after the disguise lock

Found by level-07-interactables (707a, interact_07000007_live `the_party_vase_breaks_on_a_p2k_round`, left
`#[ignore]`): after the hall walk-in releases the weapon lock (ch 0x48, the P2K auto-drawn — weapon 2 out),
holding fire through `Sim::step_bond` for 100 ticks never spawns a shot (`last_shot` stays `None`). If the
real player can't shoot on 07 after the lock lifts, that is a game-breaking bug on the level. Find the fire gate
that holds it (suspects from the worker: the weapon-lock auto-draw's switch state vs the fire path's lockout
[M Player_Weapon 0x1a24c8:242-275]), compare with the game, fix it, and un-ignore the vase test (the vase,
statue #109 and the window #298 must break on the real path). Sister: the shot is still refused while the lock
channel holds.



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

**Branch:** `s/p2k-fire-07`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T15:59Z

---
## Report (done, 2026-10-17T16:42Z)


---
## Report from the worker (done, 2026-10-17T16:42Z)

# p2k-fire-07 — the P2K never fires on 07000007 after the disguise lock

Branch `s/p2k-fire-07`. The task: level-07-interactables (707a) left
`interact_07000007_live::the_party_vase_breaks_on_a_p2k_round` `#[ignore]` — after the hall
walk-in released the weapon lock (ch 0x48, the P2K auto-drawn), holding fire through
`Sim::step_bond` for 100 ticks never spawned a shot (`last_shot` stayed `None`). This session
found the gate, un-ignored the test, and added the lock-held sister.

## What was done

**The fire gate was never a weapon-lock fire lockout. The port's weapon code is correct.**

- `Player_Weapon` 0x1a24c8:242-275 [M] (the task's suspect) contains no fire suppression at all:
  with ch 0x48 off it only reroutes the weapon-cycle actions to `Player_WeaponChange(±1, mode 2)`
  (the fists-only cycle), and on the channel's rising edge writes the auto-draw request
  `BL+0x63 = Upgrade_Weapon(6) = 2` [M 0x1a24c8:247-250 — `sb` at offset 99]. The fire path
  (`Player_WeaponFiring` 0x1a5460 → `Player_RoundToFire` 0x1a18e0 → `Player_WeaponInitBullet`
  0x1a4bf0) reads nothing from the lock.
- The real gate is the hall plate's own cut-scene. TouchOnce #412 fires ch 0x48 [D placements],
  which starts the control-taking NIS 0x06000073; its script event 4 is `Player_Disable`
  [M Script_EventHandler 0x1d94c0; docs/systems/script-player.md "Event 4"] and the player's
  input dies behind the scene hold — the same mechanism the exit hold uses
  [M psiInput_MapInputs 0x10cd80:36]. The port models the hold as `Sim::disabled`
  (`Request::PlayerDisable`, scene.rs), and `step_bond` zeroes the input and the trigger while it
  stands. The old test never played the scene away, so the trigger never reached the gun: with a
  probe, `last_shot` stayed `None` for the whole hold with `disabled == true`; skipping the scene
  with the game's own skip action (`Scripts::skip_request` = `Input_Action(−1, 0x19, 1)`
  [M nis.rs]) released the hold, and the FIRST held-fire tick after the hand-back spawned the
  shot (`last_shot` set, clip 7 → 6, the P2K's fire script 0x06000823 queued).
- The old test's `release_weapon_lock` also never skipped the scene, and its two aim legs were
  broken beside that: the vase leg faced +z (yaw 0) with the vase toward −z, and the sweeps set
  `bond.pitch` in radians while the game's aim pitch is the half-π unit (`BL+0x8a8·π/2`
  [M RotMatrixX 0x1e46a0:16-24 — `Bond::look` and the crosshair ray both scale it]).

**The test changes** (all in `crates/nf-view/tests/interact_07000007_live.rs`; no engine code
needed):

- `release_weapon_lock` now plays the plate's cut-scene away with the game's own skip
  (`skip_held_cutscene`), asserts the scene ran through the real trigger (the 0x06000073 pin the
  walkreal carries) and that the P2K is still out at the hand-back.
- `the_party_vase_breaks_on_a_p2k_round` un-ignored. The fire is the real input path: press/release
  cycles through `step_bond` (the P2K spawns one round per press edge — a held trigger fires once,
  the port's measured behaviour from the record's fire mode [L ee_start_05 record 2, +0x90 = 1]),
  with the aim swept across each target's centre in the game's half-π pitch unit. The stand
  settles 30 ticks first (a teleport lands Bond mid-air and he slides while falling), then the
  yaw and distance are re-taken from the settled position (the bullet spawns at the eye
  [M 0x1a4bf0]). The vase #49, the window #298 (kind 2, the shatter route) and the statue #109
  all break [M Break_Kill 0x12b970]; the neighbouring pane #299 stays standing (outside every
  fired line).
- New sister `the_lock_holds_the_shot_refused`: while ch 0x48 holds (before the plate), the real
  input path (the view loop's `weapon_lock_input` merge) refuses the held P2K selection and no P2K
  round ever spawns across 100 fire ticks [M 0x1a24c8:243-246]. The fists still punch (their
  fire script's op-1 is the 0x142 whoosh, no gunshot [D 07000005.bin; fix-melee]) — the assert is
  "no bullet with weapon 2", which is what the lock actually refuses.

## How it was verified

Every run this session, foreground, slim ISO, `--test-threads` default:

- `interact_07000007_live`: **7 passed / 0 failed** (the un-ignored vase test, the new lock
  sister, and the five existing sisters — including `walking_the_party_breaks_nothing`, which
  still pins that walking without firing breaks nothing).
- `walkreal_07000007_live`: **2 passed / 0 failed** (the full party walk is unaffected).
- `lighting_live`: **5 passed / 0 failed** (the P2K fire path on other levels unaffected).

The full `tools/test/suite.py` was started twice in the background per the task sheet; the sandbox
killed it between tool calls both times (empty `suite.out`, same as 707b's session), so the
foreground binaries above stand in. My diff touches only that one test file — no lib crate
changed — so nf-game/nf-formats/nf-audio are untouched by this branch.

Diagnosis probes (scratch, not committed): the gun's switch machine completes the auto-draw
correctly under the scene hold (state 3 → 0, draw script 0x0600082b → the idle 0x06000825), the
clip and fire scripts are healthy, and the first post-skip press spawns the shot. The suspects in
the task sheet (the auto-draw's switch state vs a fire-path lockout) were both checked against
the probe and the decompile and both are clean.

## Observations left open (not this task's blocker, flagged for the next session)

1. **Player bullets pass through the hall floor at the vase stand.** From (60.25, ~42.4,
   −38.0) firing down (pitch −0.20 half-π), the bullet's flight trace (`d.world.segment` per tick)
   returns no world hit: the round descends from y 43.7 through the hall floor (~43.5) to y 28
   and ranges out ~57 units later, with no impact and no death. The world trace DOES stop rounds
   at the statue stand's backdrop (ricochets observed there), so this is a localised hole in the
   level's bullet-collision geometry (or a cel-scoped trace miss on the baked hall floor), not a
   global trace failure. Shooting the floor in the port shows nothing; worth a live check (below)
   and its own fix task.
2. The statue leg's fire line must be re-taken from the settled stand: Bond slides ~0.36 west
   while falling from the teleport, which put the old fixed-x line 0.005 outside the statue's
   grown contact box — every shot missed by half a pixel. `fire_sweep` now re-aims after the
   settle; documented in its header in case the next live-fire test copies it.

## Live checks for the main agent

- At the party hall, stand at the vase table (world ≈ (60.25, floor, −38.0)), aim down at the
  floor in front of the table and fire the P2K: the real game should show a wall/floor impact
  puff and stop the round. If it does, the port's missing world hit there (observation 1) is a
  port-side collision hole at ≈ (60.25, 43.5, −42.5)…(60.25, 43.5, −52) worth a gdbwatch on
  `Bullet_update`'s collide call for that segment.
- Right after the control-taking cut-scene hands Bond back (skip it), press fire once with the
  auto-drawn P2K: expected — one shot, one muzzle flash, the first bullet leaves the muzzle.
  This pins the port's post-skip fire path against the real console.

## What's left

- Nothing for this task's scope: the fire gate is diagnosed, the vase test runs un-ignored, the
  lock sister is in. The floor-pierce observation above is a separate bullet-collision item on
  this level; the script event 10 hook (`Break_DoBreak`, the urn SP) and the party AI rules
  remain the 707a/707b carried items.
