# drake-fight-1b — the report

Task 745: port the 0700001b final fight's mechanics from the decompile and the level scripts
[M + D] — Drake's AI and phases, the missile launch sequence (the missile channels 1..8,
`MissileDeploy`, `SpaceMissile_Update`'s states), and the win/fail conditions. Branch
`s/drake-fight-1b`, from master bee8c129.

## What is done

1. **The missile launch sequence** — new `crates/nf-game/src/space_missile.rs`, the five-state
   machine read from the disassembly (the decompile's C dropped every store):
   - **state 0** [M 0x1aa1c8]: returns while `switch_channels[p7]` (the launch order, 19) is
     clear; then `ext+0x14 -= 512.0` (the float at 0x31d0dc [M]) and stays while >= 0
     (c.lt.s/bc1f 0x1aa200-208); on < 0: `MissileDeploy[p4-1] = 1` (base 0x30cd70 [M 0x1aa218]) and
     state 1. All eight p3 seeds (105/40/20/0/60/105/105/60 [D]) are < 512, so every missile arms
     on the first gated tick.
   - **state 1** [M 0x1aa23c]: the p0 launch-anim script plays; its end (`script+0x9c != 0`) →
     state 2. The port ticks a counter against the decoded script lengths [S — the object-side
     script VM is not wired into `Drones`; the lengths and the in-stream writes are [D] decoded].
   - **state 2** [M 0x1aa25c-338]: `switch_channels[p4]` (the console channels **1..8**) set →
     the p2 (disarmed) script, `obj+0xf6 = 0`, `MissileDeploy[p4-1] = 2`; clear → the p1
     (launched-away) script, `obj+0xf6 = 1`, `MissileDeploy[p4-1] = 3`; state 3.
   - **state 3** [M 0x1aa33c-3d0]: on the branch script's end, `0xf6 == 0` →
     `switch_channels[p6]` = 1 (21..28), else `switch_channels[p5]` = 1 (**199**, the abort
     channel); both timestamped; state 4 (inert).
   - The eight kind-0x39 placements (#129, #131-137 [D]) attach in `Drones` init with the
     [M 0x1a9ef0] param map (p0 script, p1/p2 branch scripts, p3 countdown, p4 index 1..8,
     p5 = 199, p6 = 21..28, p7 = 19) and tick in the FX pass. `MissileDeploy` is
     `Drones::missile_deploy` (8 bytes, the game's 0x30cd70 [M]), ready for `HUD_UpdateSpacePane`
     0x19d3b8's lamp rules (hud.rs already documents them; the pane's draw is not switched to the
     field yet — visual, needs the main agent's render anyway).
   - The p0 scripts' own channel writes are modelled at their decoded ticks [D]: gates 31..38 at
     t359 (missile 4's 06000a30 writes ch 192 at t1 and no gate).
2. **Drake's fight init** — the playtest's core symptom fixed: the port ran Drake as a generic
   mode-0 guard (Idle → AimStand → falling through the station floor in gravity; probe recorded).
   `DefaultInit`'s skin switch sends skins **0x05000078 / 0x05000094 / 0x050000ba** to skin kind
   0x18 [M 0x14b300:236-243], and kind 0x18 forces mode +0xC5 = 0x1d (DroneTypeSettings row 29),
   start +0x5A2 = 0xbd AstronautLaunch, alt 0, and health
   `DroneInit_TakeXHits(difficulty==1 ? 1.0 : 2.0, weapon 0x33)` = weapon_data[0x33]+0xc × hits
   [M 0x14b300:722-733, 0x14d928], **×3 for Drake** [M :734-739]. Implemented in `type_setup`;
   the live probe now reads mode 29, state 0xbd, health 60.0 at boot difficulty 2.
   `AstronautLaunch` 0xbd's arms are ported [M 0x16dd28]: ENTER (0x4f8 |= 0x10000 | 0x2000,
   AssignAIPath(8), CallAnim(0, 0x72)), impacts (cases 8/9 → HandleImpact), EXIT's 0x20 noted.
3. **SpaceDrake 0xc2** — the DSTATE ported verbatim [M 0x16dee0]: ENTER (health = 10.0 at
   obj+0xAC [drone.md], sight-processing + may-notice, stand idle, prop 0x46 → SetIdleTimeOut(
   0x2d, 5)), UPDATE (switch_channels[10] → 0xb4 DeleteMe), the idle-timeout arm (prop 0x46 →
   CallAnim(0, 0x4b, Rand(3)) + re-arm — the game's own shared-stream draw), anim-done → stand
   idle, and impacts only while switch_channels[10] is on.
4. **Win/fail conditions** — already on master and now pinned by tests: the ch-199 fail hook
   (FailLabel 0x0200_005a [M 0x1827d8:113-115]), AND #139 (21 & 22 & 27 → ch 9, "Disable the
   missiles" [D #139; L record 1b]), objective 2 enabled by ch 9, the defeat NIS 0x06000a20's
   t600 ch 0x0b / t720 ch 100 [D] (fix-finale-blockers), and the monitor's AND (ch 9 & 0x0b →
   ch 100) via the existing objective monitor [M 0x1827d8].

## How it was verified (fail-first, through the real input path)

`crates/nf-view/tests/drake_fight_live.rs` (opt-in via NF_ISO), all three fail on the pre-fix
code (the missiles did not exist; Drake ran mode 0):
- `the_missile_launch_sequence_reaches_each_phase_and_disarms`: ch 19 driven (AND #202's output;
  its 0xc2 input has **no writer** in the level's 77 scripts [D sweep], the same shape as
  fix_finale's ch 0x0a drive) → MissileDeploy all 1 [M state 0] → gates 31..38 at t359 + ch 192
  [D] → the console SPs #213-220 play and the console scripts write ch 1..8 at t682 [D, the
  port's real SP chain] → state 2's disarm branch (MissileDeploy all 2, 0xf6 = 0 [M]) → the
  done channels → AND #139 → **ch 9 → objective 1 completes** [D #139; L record 1b]. Sisters:
  ch 199 never rises on the disarm path; ch 24's later clear is the speech script 060009a4's own
  ch-24 stream [D] — the AND watches only 21/22/27.
- `the_missile_abort_path_fails_the_mission_on_ch_199`: with the console SPs removed (test
  setup — the proven precondition "switch_channels[p4] clear"), missile 4's state 2 takes the
  launch branch (MissileDeploy[3] = 3, 0xf6 = 1 [M]) → **ch 199 → the mission fails with
  FailLabel 0x0200_005a** [M 0x1827d8:113-115]; sister: a disarmed missile still writes its done
  channel on the same run.
- `drake_initializes_as_the_zero_g_boss`: mode 0x1d, state 0xbd, health 60.0 [M 0x14b300,
  0x14d928].

Regression runs before the push (all offline, single-threaded): `drake_fight_live` 3/3,
`fix_finale_live` 4/4 (Drake's defeat → NIS → ch 0x0b/ch 100 → MissionState 6 → 0x0730001b still
holds), `nf-game --lib` 241/241, `level_live` 1/1 (the per-level drone/object/placement counts —
type_setup's blast radius), the nf-view unit batch 17/17, `drone_portcheck_live` 35/35,
`ai_live` 5/5.

**The full suite did not run**: `tools/test/suite.py` was started twice in the background
(00:05 and 00:11 UTC) and the sandbox killed it between tool calls both times (suite.out 0
bytes, no process left). The crate runs above are what I could verify. The final-tree suite run
requested by the task could not be produced in this window — the next message should run
`NF_ISO=<abs> python3 tools/test/suite.py` (foreground or a surviving background) on this branch
before merging.

## What is left

- **The zero-g movement** (`DroneMove_AstronautCombat/Anim/HitMove/DeathMove`, the drift/
  thruster family): not ported — that is `s/astronaut-guards-1b`'s scope (its branch carries
  that work). Drake holds AstronautLaunch 0xbd [S unverified hold: the game's route-done →
  SetState(0xc1 AstronautCombatMove) has no ported destination] and still sinks under the port's
  gravity; his impacts, health and defeat path are game-faithful.
- **The HUD space pane** still reads a hardcoded 0 for MissileDeploy (hud.rs): the field exists
  now (`Drones::missile_deploy`); switching the pane over is a visual change needing the main
  agent's render.
- **The launch order's writers**: ch 19 = AND #202 (0xc1 & 0xc2) [D]; 0xc1 is written by the
  speech script 060009a4 at t1510 (behind the 190/191 chain, which DOES run in the port — ch 34
  rises); **0xc2 has no writer** in any of the 77 scripts [D sweep]. The chain cannot fire from
  disc data alone — in a normal playthrough the missiles never launch (and the mission still
  completes via Drake's NIS writing ch 100 directly [D 0x06000a20 t720]). docs/levels/0700001b.md
  §6's "0xc1 has no writer on the disc" is half-wrong per the decode; §2's "the AND inputs are
  console touch entities" is stale (they are the missiles' p6 channels [D p6 + M state 3]).
- The p1/p2 script waits use the last decoded event ticks (1001/1131 [S]) — the exact script
  lengths are unpinned; the tests assert inside generous windows.

## Live checks for the main agent

1. **Drake's state word after the fight gate** — watch drone #186's state (+0x5A0-ish SM state)
   for ~10 s after ch 10 fires in a fresh boot on 0700001b: expect **0xbd** (AstronautLaunch)
   per [M 0x14b300:722-727]; then whether it leaves 0xbd (→ 0xc1 on the AI route done [M
   0x16dd28 case 3]) and what he does when Bond is visible (my port holds 0xbd [S]).
2. **Drake's health** — read drone #186's +0xAC after the spawn: expect 60.0 at difficulty 2
   (or 30.0 at difficulty 1) per [M 0x14b300:726-739 + 0x14d928] — confirms weapon 0x33's
   damage record and the ×3.
3. **MissileDeploy during the launch** — if the launch order can be poked on (ch 19), watch
   0x30cd70..0x30cd77: expect 1s within a tick of the gate, then 2 (disarm) or 3 (launch) per
   missile; and `switch_channels[21]/[22]/[27]` + ch 9 rising at the p0-end + ~1131 ticks.
4. **The zero-g question** — whether the game's Drake (+0x4f8) ever has gravity bit 0x80
   cleared or 0x200 set after AstronautLaunch's ENTER [M 0x141e58 DoGravity reads both], i.e.
   where the float actually comes from (the anim's root motion vs a cleared gravity gate).

## Usage

`not visible to me` — no token, credit or context-window figures are exposed to this session
(no usage panel in this environment; nothing to copy exactly).
