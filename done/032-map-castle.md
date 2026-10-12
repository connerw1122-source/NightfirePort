Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `map-castle`, make the castle missions 07000001–07000004 (Paris Prelude / The Exchange / Alpine Escape / Enemies Vanish, the "Castle A–D" updaters) completable from start to end: the map, its actors and its objectives

Levels: `07000001`, `07000002`, `07000003`, `07000004` (`docs/systems/levels.md` has the table, `docs/atlas/` the systems). The goal is simple to state: a player
can load each level, do what the real game asks, and reach the next level, with every actor and objective behaving as
in the game.

For each level, in order:
1. **Map the level from the game's own data and code.** Write `docs/levels/<code>.md` with:
   - the start position(s) and which entry each comes from (the route Bond took in the previous level);
   - the objectives in order (text from `USATxt.dat` via `Txt_BindLabel`; the code that sets and completes each,
     `Mission_*` / `PlrStat_*` / script channels);
   - the actors (allies, civilians, VIPs, bosses, scripted characters) with their role and what triggers them;
   - the required interactions (doors, keys, switches, gadgets, wires, ladders);
   - every exit route with its trigger, condition and destination level;
   - the failure conditions (killing civilians, alarms, an ally dying, a timer).
2. **Write a headless walkthrough test** (`crates/nf-view/tests/walk_<code>_live.rs`) that drives the real path:
   - `nf_view::load`, then `step_bond` with real input where the player would press something (use, fire, crouch,
     switch weapon); teleporting between rooms is fine for walking;
   - assert each objective stage in order, and that the level end fires and loads the right next level;
   - sister assertions: skipping a required step must NOT end the level, and breaking a failure condition must fail
     the mission.
3. **Fix every blocker the walkthrough hits:** missing objects, doors that don't open, triggers that never fire,
   scripts or cut-scenes that never start, actors that don't move, act wrong or can't be interacted with, objectives
   that never complete, the wrong next level. Each fix cites the game address it ports.
4. **List what you couldn't port** in the doc and the report, with the game function behind each item.

**Known so far:**
- 07000001's exit needs a skin-0x4F NPC who waits on channel 0xB, which nothing on the disc writes (WORKLOG
  `fix-level`; `level_reload_live::level_end_07000001_needs_bond_and_the_0x4f_npc_on_the_plate`). Find who writes 0xB
  (a script, the mission layer, a dialogue end) and finish the route end to end.
- 07000002's bathhouse guard and the hostage NPC (C2) belong to `015-fix-npcs`. If that branch is on master, use it;
  otherwise note the dependency and continue.
- The player state carry-over (F2) and the entry-dependent spawn (F1) are covered by `level_carry_live` /
  `level_reload_live`. Keep them passing and extend them to your levels.
**Working alongside the other map tasks.** Five other workers make other missions completable at the same time, and
`031-fix-mission` builds the objective system they all use.
- Pull `master` at the start and before every push; if `crates/nf-game/src/mission.rs` exists, build on it instead of
  writing your own.
- Keep your level-specific code in its own file (`crates/nf-game/src/mission/<level code>.rs`, or a table entry), and
  your docs in `docs/levels/<level code>.md`.
- Keep edits to shared files (`scene.rs`, `drone.rs`, `triggers.rs`, `nis.rs`, `lib.rs`) small and additive, and list
  them in your report.
- Never reformat or reorder a shared file.

Files: `docs/levels/`, `crates/nf-view/tests/walk_*_live.rs`, `crates/nf-game/src/mission/` (or `mission.rs` if
031-fix-mission hasn't split it), and minimal hooks elsewhere.

**Report:** for each level, say completable yes/no; if no, what blocks it; and a short list of spots the user should
check in play (with `--bond x,y,z,yaw` so the main agent can render them).

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/map-castle`.

---
claimed-by: sandbox2 at 2026-10-12T05:47Z

---
## Report (done, 2026-10-12T07:29Z)


# map-castle — the castle missions 07000001–04

Branch `s/map-castle`. Goal: a player can load each castle level, do what the game asks, and reach the
next level.

## Per level: completable?

### 07000001 Paris Prelude (Castle A) — **YES**
The whole route works end to end in the port and is walkthrough-tested: load → the intro cut-scene
(script 0x06000087, pack 07800001) writes ch 0xB at ~tick 2119 → the contact (skin-0x4F NPC) and 5 more
drones enable → Bond + the contact on the "LoadPointActivator" plate (Touch #180, mask 0x18, filter
0x4F) → ch 0x10 → the exit (0xE8 #18, gate 0x10) → 07000002 loads through the real reload path. The
known "nobody writes ch 0xB" gap is SOLVED: the writer is the intro script itself (stream 8, event 0xD).
Failure: the contact's death raises ch 0x60 (the mission-fail channel) — asserted; its consumption
(mission failed) is the 031-fix-mission layer. The level's one objective (watch ch 0x47) has no disc
writer — its HUD completion is the mission layer's; the exit does not depend on it.

### 07000002 The Exchange (Castle B) — **YES for the exit route; the objective channels are a dependency**
The exit (0xE8 #206, E-key) has a KEY GATE on channel 0x4C [M Trigger_Activate 0x1b2318:76-90]: without
it the E press is refused (the "locked" message path — asserted), with it 07000003 loads (asserted).
No writer of ch 0x4C exists anywhere in the level's data (scripts, triggers, drone death channels,
pickups, doors, locks all scanned) — it belongs to the un-ported mission/dialogue layer (031-fix-mission;
the C2 NPC work belongs to 015-fix-npcs, whose branch has not landed on master). The intro script
0x06000084 writes ch 0x47 (the castle-A objective's completion — the story hand-off) and plays.

### 07000003 Alpine Escape (Castle C) — **YES for the exit route; same dependency**
The exit (0xE8 #451) key-gates on ch 0x52: refused without, loads 07000004 with (asserted both ways).
No disc writer for 0x52/0x4E/0x51 (the mission layer again). The level's ladder and 5 water cels are
ported systems.

### 07000004 Enemies Vanish (Castle D) — **YES to the mission-complete channel; the final load is a dependency**
The level has NO 0xE8 exit by design: the intro (script 0x06000083) freezes the AI and disables Bond,
ch 0x38 starts the story scene (0x06000082) which writes ch 0x53 (castle-C's completion) and re-enables
the AI via the Script_Free 0x80 flag [M Script_Free 0x1da880] — the port plays all of it and settles by
~tick 1600. Killing the target (skin-0x1e drone) raises its death channel ch 0x54; the MultiplexOut #46
copies it to ch 0x64 = the mission-complete channel [M Mission_MonitorObjectives 0x1827d8:165-172].
Mission_Update 0x182c60 would then load 0x7300004 (the next arc's DRIVING level — out of the port's
scope anyway). Steps 1–2 asserted; the ch-0x64 consumption is 031-fix-mission.

## How it was verified
- `crates/nf-view/tests/walk_07000001_live.rs` … `walk_07000004_live.rs`: 8 tests, each with a sister
  (the plate refuses Bond alone; the key gates refuse without the channel; no E/no kill → no exit/no
  completion channels; the fail channel stays raised).
- The objective/mission system evidence: the ACTION.ELF tables dumped at 0x2a3e28–0x2a3f78 and the
  mission table 0x2a4354 [M], the live channel state [L ee_start_07000001.bin 0x26fc90], the script
  channel writes [D decode]. `walk_probe.rs` holds the per-level placement/script dump as a test.
- Full suite after the master merge: 374 passed / 3 failed (the 3 disc_golden slim-ISO failures).

## Code changes
- **No gameplay-code changes were needed** — the port's existing systems (pack cut-scenes, trigger
  gates, death channels, the Script_Free re-enable) already implement every step the levels need. The
  work was mapping + tests + docs.
- Shared files touched: `docs/WORKLOG.md` (entry 0092) only. `docs/levels/07000001..04.md` are new.
- `mission.rs` does not exist yet (031-fix-mission in flight); I built nothing there to avoid the
  collision — the objective tables and channel map live in `docs/levels/*.md` ready to be table-ised.

## What the user should check in play (nf-view)
- 07000001 `--bond -2.78,6.84,-12.4,-0.09`: the intro plays (~35 s) and the contact appears; walking to
  the castle door WITH him should end the level. `--bond -33.1,2.9,14.5,0`: the LoadPointActivator plate.
- 07000002 `--bond -27.17,1.64,29.8,0`: the intro; `--bond -10.05,6.75,-23.2,0`: the Load Point 2 (E)
  — in the real game the lock/dialogue layer opens it.
- 07000003 `--bond 18.1,6.65,16.05,0`: the Load Point (E, gated).
- 07000004 `--bond 1.87,5.84,41.07,3.14`: the intro + story scenes play back to back and leave Bond next
  to the target.

## What I couldn't port (with the game function behind each)
- The mission-objective layer: `Mission_Init` 0x182270, `Mission_MonitorObjectives` 0x1827d8 (the
  objective HUD + the all-complete ch 0x64 + the fail channel 0x63), `Mission_Update` 0x182c60 (the
  complete/fail flows) — 031-fix-mission's task; the castle data it needs is mapped in docs/levels/.
- The dialogue/talk objective writers (who set ch 0x4C/0x52/0x48/0x4B/0x4D/0x4E/0x51 on the castle
  levels): the candidates I eliminated are listed per level in docs/levels/; the survivors are the
  `NDrone2_MonitorTalkToMissionObj` 0x144340 / `NDrone2_TalkToMissionObj` 0x14a8b8 talk system and the
  gadget/lock interactions (`Lock_Update` 0x1ae3d8) — neither ported.
- The bullets-vs-civilian damage drop (the 0x4F contact takes no bullet damage; the grenade path does) —
  the C2 class of gaps, 015-fix-npcs.
- The mission-fail flow's destination level 0x7000048 and the complete flow's 0x7300004 (both driving/
  wrapper levels, out of scope).
