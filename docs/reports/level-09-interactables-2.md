# level-09-interactables-2 — the held 09 work un-held: the drone change scoped to the game, the four master tests green, the census and the kind tests kept

Task 709c: the HELD `s/level-09-interactables` (709a, 45c5dc5) — its 19-line
`crates/nf-game/src/drone.rs` change breaks four tests that pass on origin/master. This task
merges the newest origin/master, scopes the change to what the game does (cited), keeps the
census/interact tests, and makes all four pass.

## What's done

1. **The tick-head wake loop is dropped, not scoped** — the game has no such second poll, so
   "scoping" it would still be a divergence. What the game does:
   - The per-frame wake message is the drone's **own**: `NDrone2_ControlSTANDARD` 0x1490f0:81
     self-sends msg 3 every frame, and `Drone_Control` 0x139268 gates only on
     `NPCGlobals`/`Drone_bDisableSystem`/the MP slot — never on the enable flag. So a frozen
     waiter (`NDrone2_Enable(0)` 0x149dd0: obj+0xF0 |= 0x10|0x20, +0x4F8 &= ~0x1801A0) still
     samples `switch_channels[+0x134]` in `NDrone2_DSTATE_WaitSwitch` 0x15b960 case 3 from its
     own control run and wakes itself.
   - The port already models exactly that: the per-drone walk slot samples `channel_on` at the
     slot (the game's newest-first object order, 5aafe03d) and the WAIT_SWITCH msg::UPDATE arm
     wakes the disabled waiter there. The held loop sampled the same channels at the TICK HEAD —
     an earlier, out-of-order poll that sees pulses the game's own update order hides: on
     07000005, drones 271/272 update before mux 270 (newest first) and mux 270 rewrites 0xA4 off
     before drones 65–68 update, so 65–68 never see the pulse [L WORKLOG 0028] — the head loop
     woke them (ai_live `alarm_pulse_reaches_only_the_drones_updated_first`, "drone 66 never saw
     the pulse", left 112 = state 0x70). The same out-of-order wake hit the Drake flythrough and
     the gate-pair casts in the other three tests.
   - Dispatcher detail that settles the model: `Drone_SM_InitObject` 0x175838 wires
     `NDrone2_ProcessStateMachine` 0x175778 (indexes `NDrone2_StateFuncs[state]`) at smi+0x28; an
     unconsumed message retries with state index 0 = `NDrone2_DSTATE_Global` — the port's
     `handle(state)` → `handle(GLOBAL)` fallback. `NDrone2_Enable(1)` proceeds only while
     `+0x4F8 & 0x200` is CLEAR and restores `+0x4F8 |= 0x801A0` + `obj+0xF0 &= ~0x30` [M-dis
     0x149dd0-0x149e4c; the decompile's masks and its early-return sense are inverted].
2. **The 709a "re-freeze" finding was an artifact of that loop**, not a port bug: the loop's
   frame-1 wake on 09 landed before the intro's start path, and the start machinery then
   re-entered WaitSwitch on the woken drones ("both lobby-side guards end enabled=false state
   1 by the intro's end" — 709a's report). With the loop dropped, the diagnostic timeline shows
   all six ch-0x51 drones (placements 62/64/65/66/250/261) enabled from frame ~20 through the
   whole intro (3000-tick trace, `--test-threads=1`). The gdbwatch ask for the re-freeze is
   withdrawn.
3. **The Global entry condition is snapshot-based** — documented, not changed:
   `NDrone2_DefaultInit` 0x14b300:466-468 latches `+0x134` = the placement's switch channel and
   `+0x136` = `switch_channels[+0x134]` at creation; `NDrone2_DSTATE_Global` 0x15b830 msg-1
   sends a channel-armed drone to WaitSwitch only while `+0x136 == 0` (the snapshot, not the
   live channel), and the states a wake lands in consume msg-1 (Patrol 0x15c498 case 1 →
   `caseD_0` → returns 1), which is why the wake does not re-freeze. The port's GLOBAL ENTER
   reads the live `channel_on` — equivalent at creation time; a live check could pin whether the
   difference is observable anywhere (listed below).
4. **The census and the 12 kind tests kept** (merge of 709a): docs/levels/07000009.md's census
   corrections (0x20 = breakable glass, no 0x30 monitor, 0x71 = rotor), census_07000009.rs, and
   interact_07000009_live.rs.
5. **The guard-kill test un-ignored and passing** — its zero damage was its own weapon: 09's
   loadout opens on the K5 sleep darts (0x43, clip 5 + spare 10 [L loadout trace]) whose
   player-shooter hits route to the zero-damage dart msg [M `DealWithObjHit` case 5
   0x13fff0:269-306: 0x43/0x44 → msg 0x19; the darts stun (STUN_DART 0x50-0x52) and never reach
   `NDrone2_HitDamage` 0x145558]. `switch_to_armed` cycles until ANY gun has ammo — the dart
   gun. The new `cycle_to_lethal` walks to the first lethal owned gun (the P2K 0x2, 7+41), and
   the guard (hp 10) dies to the real fire path: health −1.8, corpse. The kill test's ignore
   reason also claimed bullets pass through frozen guards via `Drone_CollisionHandler`
   0x140968:62 — with the loop dropped the guards are enabled, so that gate no longer applies.
6. **Docs** (docs-first): docs/systems/drone-ai.md — the dispatcher wiring, the self-msg-3 wake
   (no enable gate in Drone_Control), the +0x134/+0x136 latch, Patrol's msg-1 consume, the
   disasm-corrected `NDrone2_Enable` masks. docs/levels/07000009.md — the guard section
   rewritten (the wake is the per-slot UPDATE sampling; the tick-head loop's story; the
   dart-vs-P2K weapon fact).

## How it was verified

All runs `--release --test-threads=1` with `NF_ISO` (the slim onfoot ISO) and `NF_SILENT=1`:

- **The four tests that 709a broke, on the final tree** — all green with master's drone.rs:
  - nf-game ai_live: **36 passed, 0 failed** (includes
    `alarm_pulse_reaches_only_the_drones_updated_first`);
  - nf-view fix_cutscenes2_live: **5 passed, 0 failed**;
  - nf-view spawner_kill_live: **5 passed, 0 failed** (1 pre-existing ignore);
  - nf-view kimiko_actors_live: **2 passed, 0 failed**.
- **The kept 09 work**: interact_07000009_live **13 passed, 0 failed, 1 ignored** (the E-route
  camera-pick test, pre-existing — below); census_07000009 (2 probe tests, `--ignored` dumps).
- **The 09 neighbours**: walk_07000009_live 3/0, walkreal_07000009_live 2/0,
  elevator_buttons_09_live 3/0.
- **The drone/AI neighbourhood**: drone_portcheck_live 5/0, fix_ai_remainder_live 2/0,
  spawner_kill (nf-game) 9/0, fix_portonly_review_live 3/0, fix_party_mission_live 9/0 (6
  pre-existing ignores), cutscene_08_actors_live 2/0, nf-game lib **278 passed, 0 failed**.

**The full suite did not run to completion in this sandbox.** `tools/test/suite.py` was started
in the background twice in the session's first minutes (plain `nohup` and `setsid nohup`); this
sandbox kills every process between tool calls, so both died during the build phase (suite.out
empty, no cargo). The foreground runs above are the session's verification; the full four-crate
suite needs a runner that survives, or the next session re-runs
`NF_ISO=<abs> python3 tools/test/suite.py` (foreground) on this tree.

## What's left

- **The E-route through the closed lift doors** (interact_07000009_live
  `the_load_point_e_route_exits_with_objective_2_incomplete`, still `#[ignore]`d): the port's
  use pick runs from the RENDER CAMERA (scene.rs passes `camera` as the head) while the game
  builds it from `Player_GetHeadPos` [M 0x1a5170:101-106]; at the doors the camera hangs ~2.5
  behind Bond so the 1.0 sphere never spans the exit box (z 10.33..12.49 [D #63]). The fix is
  shared with the doors/switches/SP reaches (tuned to the camera) and needs its own session.
- **The models sweep test** (NF_TEX_TRACE=1 white-fallback assertion per level) — carried from
  709a, not written.
- The full suite on a surviving runner.

## Live checks for the main agent

- The +0x136 snapshot vs the port's live `channel_on` in the Global entry: PCSX2, a channel
  armed BEFORE its drone is created (e.g. a channel a script pulses during the load) — the game
  should skip WaitSwitch entirely (0x15b830:26 `+0x136 != 0` → straight to the start state); the
  port would put the drone in WaitSwitch until the channel drops. gdbwatch: the drone's
  +0x136 byte at creation vs `switch_channels[+0x134]` at that moment, then obj+0xF4 (the state)
  on the first frames. Only levels where a channel is on at drone-creation time can show it.
- The E-route pick: fresh-boot 07000009, stand at the CLOSED lift doors (−3.7, lobby floor,
  z ≈ 13.9), press E — expect the 0700000a load with objective 2 incomplete (the doc's route
  (b)) [M 0x1b2318:76-90 key_gate 0]. Pins the camera-vs-head pick question.
- `NF_TEX_TRACE=1` on a fresh 09 load: expect no white-fallback prints (the loader says 0
  unresolved).

## Usage

not visible to me (no token, credit or context-window figures are shown in this environment).
