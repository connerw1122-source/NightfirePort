## Your task: `fix-death-anim-events`, the death script's op-4 events for Bond

When Bond dies the game plays anim script 0x6000151 on him (`Player_ChangeSubState` 0x190630 case 0xd [M :66-73]). The
script (07000005.bin, `nf-extract script-info <iso> 07000005.bin 6000151`) is one sequence window (1..297, 0x040000a3)
plus four op-4 commands: args [5,5], [8,4], [98,4], [116,6]. The script clock runs 2 frames per tick while dead
([L] PCSX2 trace: the script's +0x90 reads 12, 32, 52 at dead frames 8, 18, 28).

The main agent has ported the death's camera motion (`nf_game::death::death_root`, branch `mine/death-anim`, not
yet on master: do NOT touch `death_root` or the eye/camera code). This task is only the events.

Do:
1. Find what op 4 does in `AnimProcessScriptCmds` 0x1c1b38 (the event/callback opcode: sound? footstep? a message to
   the object's handler?) and what the player's handler does with args 4/5/6 (search the player's anim callback in
   the decompile). Document it in `docs/systems/anim-engine.md` with addresses.
2. If the port already runs op-4 events for drones (`fix-anim-events`, #288: grep `op == 4` / `anim_event`), reuse
   that path for Bond's death script. Fire each event at its script frame on the dead-tick clock above.
3. Test (NF_ISO, `crates/nf-view/tests/fix_death_anim_events_live.rs`): kill Bond on 07000005
   (`sim.drones.health.health = 0.0`, then `step_bond`) and assert each event fires once, on the tick the clock passes
   its frame (frame 5 → dead tick 3, etc.), with the effect the game gives it (e.g. the sound id queued). Sister
   assertion: nothing fires while alive. Tests must stay silent (NF_SILENT).

Files: `crates/nf-game/src/death.rs` (only a new event-tick function, not `death_root`), the anim-event code you
reuse, `docs/systems/anim-engine.md`, the new test.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).


**Branch:** `s/fix-death-anim-events`.
