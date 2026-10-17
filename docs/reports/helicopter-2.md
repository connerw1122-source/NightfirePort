# helicopter-2 — finish the helicopter (the merge hold)

Task 750h2, branch `s/helicopter-2`. `s/helicopter` (750h) was held at the 2026-10-17 batch
merge: on master it broke two tests that pass without it —
`nf-game guard_reaction_live::sight_to_first_shot_for_the_three_sighting_paths` (panic at
guard_reaction_live.rs:216) and `nf-view fix_weapon_sounds_live
::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` ("0x3a: missing sounds
[1093, 1094] never played; 0x41: [639] never played"). Both now pass.

## What was wrong

07000005 carries two copters (map50 #241 gate 0, #242 gate 0x42 [L the level's params]). The
port's `Copter_Update` case 1 ran the spot check — the timer increment, the 240/DiffMod
cone/LOS test and the `Rand_Rand(10)` attackmode draw — UNCONDITIONALLY, but the decompile
keeps all of it inside the conjunct `record+0x74 != 100` [M 0x17dbb8:140, the `Rand_Rand(10)`
at :184 inside it]. The Mayhew patrols carry +0x74 = 100 [L the level's params], so in the game
they never spot: they fly their loop forever, never draw from the shared RNG, never fire, never
play 0x5df, never request combat music.

The pre-fix port spotted the courtyard player at t160 and attacked for the rest of the window:
the copter's `Rand_Rand(10)` + `Rand_Rand(0x86e6)` pod picks joined the shared RNG stream,
its 0x71 turret rounds and 0x5df voices consumed sound voices, and the guards' sight-to-first-
shot stream shifted (`guard_reaction_live` path 3) while the weapon-sound sweep got interrupted
(`fix_weapon_sounds_live` 0x3a/0x41 rows).

## The fix (commit 9b40d7ef, from the first helicopter-2 session)

`crates/nf-game/src/copter.rs` case 1: the timer increment, the spot test, the attackmode draw
and the combat-music request are gated on `self.arrival != 100` (record+0x74). Case 2 was made
decompile-exact on the attack path at the same time:

- the attack tick fires BEFORE the bail checks [M 0x17dbb8:191-198 → :199-242];
- the bail timeout is strict (`5·60·DiffMod < timer`) and a keep tick does not reset the
  counter [M :200]; the no-player arm lands in the same reset [M :199-201];
- `Copter_FireMissile`'s `Rand_Rand(0x86e6)` pod pick draws on EVERY call, before the
  `60·DiffMod` launch gate [M 0x17d9f0:47 vs :58-64].

## This session

- Merged `origin/master` (4e70052e) into `s/helicopter-2`; the only conflict was
  `docs/WORKLOG.md` (append-only, both sides kept). `drone.rs`/`lib.rs`/`menu.rs` auto-merged.
- **One test was wrong and is fixed (commit cfa8291e).** Our own
  `copter_patrol_latch_live::the_ambient_patrol_copters_never_spot_or_draw_on_07000005`
  asserted `state == 1` for BOTH copters every tick, but #242 sits behind gate 0x42 and the
  window's scripts never open it: `Copter_Update` returns before the state machine while the
  gate channel is off [M 0x17dbb8 head — `record+0x70 != 0 && switch_channels[+0x70] == 0 →
  return`], so the game keeps it idle at state 0. The probe run confirmed: #242 state 0,
  un-started, spline t = 0 across all 500 ticks. The test now pins state 0 + `!started` for the
  gated copter, state 1 for the free #241, attackmode 0 / gun_shots 0 for both (the latch's
  teeth — no RNG draw, no turret tick), #241's flight advance (t in (80, 100) after 500 ticks —
  the latch must not stop `Copter_Move`) and #242 never flew (the early-return sister). Per the
  task rule, the TEST was wrong against the decompile and was fixed with the address; no
  measured test was loosened — the two held tests pass unmodified.
- Docs first: `docs/systems/helicopter.md` gained the four decompile facts this task pinned —
  the case-1 `+0x74 != 100` conjunct (the latch, with the merge-hold story), the one-shot gate
  (the head unhide + zeroes record+0x70 on the rising edge [M 0x17dbb8 head :99-105]), the
  fire-before-bail order with the strict timeout, and the FireMissile pod pick drawing before
  the launch gate.

## How it was verified

- The two held tests on the merged tree, foreground: `guard_reaction_live`
  (sight_to_first_shot_for_the_three_sighting_paths) and `fix_weapon_sounds_live` (5 tests:
  3 pass, 2 ignored for the documented asset gaps) — green.
- `copter_patrol_latch_live` 2/2 (the latch through the real sim path + the pod-pick draw-order
  pin with the level's own #241 record).
- The first session's full-suite run on the pre-merge tree with the fix applied
  (05:41 UTC): 2049 passed, 6 failed, 74 ignored — the 6 are the allowed 3 `disc_golden`
  (slim ISO) + the 3 known no-GPU pylon probes (`probe_pylon_render`,
  `fix_0c_pylon_live` x2), pre-existing on master per the merged task reports
  (e.g. music-triggers: "suite 2012 passed / 6 failed; 3 allowed disc_golden + 3 inherited
  pylon render probes").
- The final tree (merged with origin/master) full suite via `tools/test/suite.py`: see the
  "Final suite" line below (run completed after this section was written — the WORKLOG entry
  carries the number).

Final suite: FILLED_AT_FINISH

## What's left / known divergences

- **The one-shot gate is not ported.** The game zeroes record+0x70 (and unhides the body) on
  the first on-tick [M 0x17dbb8 head :99-105]; the port keeps `gate` set and re-checks the
  channel every frame in `update()`/`visible()`. Equivalent while the channel stays on; a
  channel that drops after the start would freeze/hide the port's copter where the game's
  would not. Not changed here because nothing in the ported flow toggles a started copter's
  gate, and `fix_objects2_live` pins `gate == 5` on the live list — the change needs its own
  task. The doc documents the exact rule.
- The per-part damage routing gap from 750h stands (the flight script's part entities play on a
  static base; damage attributes through the body sphere) — documented in
  `docs/reports/helicopter.md`, untouched here.

## Live checks for the main agent

Nothing is blocking. If convenient, one observation would firm up the latch story beyond the
decompile: on a FRESH boot into 07000005 (courtyard start, difficulty 1), the two copters
should fly their patrol loops and never fire — no 0x5df shots, no combat-music request, for as
long as the player stays in the open courtyard (the pre-fix port attacked at t≈160 = ~2.7 s).
A `--bond 67.31,26.5,99.47,·,·` render of the courtyard at t≈10 s should show both copters
overhead on their routes.

## Usage

not visible to me (no token/context figures are exposed in this environment)
