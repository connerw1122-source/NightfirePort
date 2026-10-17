# level-03-complete — 07000003 "Alpine Escape" (Castle C) played start to finish

Branch `s/level-03-complete`. Everything below was driven through the real input path:
`nf_view::load` + `Sim::step_bond` (walking, turning, the use key), the level's own scripts,
triggers, doors and mission layer running untouched. No teleports after the spawn.

## What's done

**`crates/nf-view/tests/walkreal_07000003_live.rs` — 4 tests, all green
(`--test-threads=1`, 102 s):**

1. `walk_the_alpine_route_to_the_exit` — the courtyard route to the Load Point; the exit refuses
   E without ch 0x52 [M Trigger_Activate 0x1b2318:76-90]; after the objective leaves are handed,
   the disc's own AND trigger writes 0x52, obj2 announces, and E loads 07000004 through the real
   reload path.
2. `walk_the_full_route_and_complete_every_objective` — the full play: spawn → the 0x4d reveal →
   **the front gate** (E-plate → ch 0x31 → the gate SS animates + the locked door opens → Bond
   walks through to the north side) → the servants → both objectives complete → the AND opens the
   key gate → E on the exit → `level_exit == 0x0700_0004`. 102,411 ticks, no falls, no stuck.
3. `the_key_gate_stays_shut_without_both_objectives` — the AND sisters: 0x4e alone never writes
   0x52; 0x4f+0x50 writes 0x51 but still not 0x52.
4. `killing_a_servant_fails_the_mission` — real fire on a servant → its death channel 0x60 rises
   → fail label 0x0400_0034 "Killed Civilians" + MissionState 2 + ch 99 [M 0x1827d8:98-104,
   0x182c60 case 2].

`crates/nf-view/tests/walkreal_probe03.rs` — the level census probe (diagnostic, --nocapture).

## The objective list and how each completes

The mission table [L ee_start_05.bin @ 0x2a3f18 via mission::c07000003]:

| obj | text | watch | enable | how it completes |
|---|---|---|---|---|
| 0 | "Retrieve Dragon Safe contents" | 0x4e | 0x4d | announced by the spawn reveal touch (out 0x4d at (−9.0, −23.2) [D]) through the real monitor [M 0x1827d8:55-67]. **The writer of 0x4e is not on the disc** (every placement, trigger, script, pickup, door and SS scanned [D the probe]) — the un-ported mission/dialogue layer's, handed by the test at the level's own beat (the walkreal-a precedent: 07000001's 0x47, 07000002's 0x4c). |
| 1 | "Rescue Mayhew's servants" | 0x51 | 0x4d | **the writer IS on disc, one level down: AND(0x4f, 0x50) → 0x51** [D the kind-3 trigger, out 0x51 list 4f,50]. The test hands the two LEAF channels as Bond stands with the servants, and the disc's own mux completes obj1. The three servants are skin 0x0500002b, team 3, death channel 0x60 (protected) [D]. |
| 2 | "Find door leading to Mayhew" | 0x53 | 0x52 | announced when 0x52 fires — **AND(0x4e, 0x51) → 0x52 is on disc** [D the kind-3 trigger, out 0x52 list 4e,51] — and completes in 07000004 (its story script 0x06000082 stream 9 writes 0x53 [D]; the level doc records it). The walk exits the way the game does: the Load Point loads 07000004. |

**Correction to the docs:** `docs/levels/07000003.md` said "the writers of 0x4E/0x51/0x52 are not
on the disc". Wrong for 0x51 and 0x52 — both are written by the disc's mux triggers (the old scan
missed the virtual kind-3 ANDs at (0,0,0)). The walk test header and WORKLOG carry the correction.

**The key gate (the task's known open item):** the exit's key gate ch 0x52 is the AND of both
earlier objectives — the gate opens itself when the objectives complete; no code needed. The
"key-gated route" the old walk notes is this AND, plus the front gate: the E-plate (kind 1, mask
4 = E-usable [M Trigger_Activate 0x1b2318:38-49]) writes ch 0x31, the locked front-gate door
(door[0], ch_lock 0x31) then opens to E [M Door_Activate 0x135be0's ch_lock refusal / Door_Update's
rising-edge unlock] and the one-shot gate SS #502 (ch 0x31, 2 keys, wrap disarms to state 0 [M
0x1a9948:119-128]) animates the gate mesh with its pose held at the end key. All three verified
through the real E path, with a locked-refusal sister.

## The route

spawn (−8.85, 6.44, −23.19) → west lawn (0, −30) → the west lane (1, −33) → along the moat
(8, −36) → the gate plate (11.8, −35.6): E the locked door first (refused, sister), E the plate
(0x31), the SS plays, E the door, through the gate to (14.7, −36.8) → servants at (15.4, −27.0),
(30.8, −5.7), (32.9, 1.6) — each visited alive, 0x4f/0x50 handed at the first two → 0x4e → the
exit lawn (18.5, 13.0) → E the Load Point (18.11, 6.65, 16.05) → 07000004.

The direct courtyard line to the plate is NOT walkable: the garden moat ditch drops Bond into the
water at (11.8, −34.5) and (16, −31) [D the 5 water cels]. The west lane (z ≈ −33..−36, standable
end to end) is the real approach. The swim system is the game's own alternative through the ditch.

## What the walk still cannot do (with the exact reason)

- **obj0's and obj1's leaf writes (0x4e, 0x4f, 0x50) are hand-offs, not port logic.** Their only
  owner in the game is the mission/dialogue layer outside the level archive; nothing in the
  decompile's channel-writer enumeration or on the disc writes them [D the probe scan; the
  fix-finale-blockers enumeration]. The tests hand them at the moments the level offers (Bond
  standing with each servant; the safe contents beat has no on-level entity at all) and pin every
  DISC-proven consequence through the real path. Porting the dialogue layer itself is level-01/02
  sized work, not a 07000003 blocker.
- **The optional ladder (34.3, 7.6, 18.2, top 10.7) and the 5 water cels are not on the walk.**
  Both systems are ported and tested on other levels (ladder_02_live, fix_swim_vent_live); they
  gate no objective here — the walk climbs nothing because the route to every objective and the
  exit is on foot.
- **`mission_events` drains every step** (hud_tick runs inside the headless `Sim::run` too), so
  the tests pin the persistent mission globals (states, fail_label, MissionState, ch 99) instead
  of the event vec. The events remain available to the view path, which is their real consumer.

## How it was verified

- The four 03 tests: `cargo test --offline --release -p nf-view --test walkreal_07000003_live --
  --test-threads=1` — **4 passed, 0 failed**.
- The recon scratch (walkreal_probe03 `recon03`) found two real port facts now cited in the tests:
  the mux family writes its output EVERY tick (a direct 0x51/0x52 hand is overwritten — the
  mission layer's writers must be the leaves), and `Mission_Init` zeroes its watch channels
  (0x4e/0x51/0x53) on the first tick [M 0x182270:106-107] — hands must come after init.
- The fast suite (`tools/test/suite.py`, NF_ISO set): built 496 test binaries in 486 s and started;
  **this sandbox has 3 GB RAM and the runner OOM-killed binaries** (`CRASH weapon_matrix_live
  (exit -9)`, `CRASH walkreal_probe13 (exit -9)` in suite.out; the same SIGKILL killed a parallel
  4-test run of mine until `--test-threads=1`). The final-tree rerun hit the same wall. The 03
  tests themselves run green single-threaded; the suite total line is `not available in this
  sandbox` — the OOM is a machine limit, not a tree failure. (The pre-OOM output showed only the
  3 `disc_golden` failures the slim ISO always has.)

## Live checks for the main agent

- The 0x4e beat: fresh-boot PCSX2, 07000003, open the pause OBJECTIVES tab after retrieving the
  Dragon Safe contents — obj0 shows COMPLETE. The port has no on-disc writer; if the real game
  completes it via a dialogue/cut-scene beat, a gdbwatch on `switch_channels[0x4e]`
  (ACTION.ELF BSS) during the level would name the writer instruction for the report.
- The servants' rescue beat: the same watch on 0x4f/0x50 while reaching/using each servant —
  confirms (or corrects) the hand-off moments the tests assume.

## Usage

not visible to me
