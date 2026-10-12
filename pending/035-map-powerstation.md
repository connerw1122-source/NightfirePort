Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `map-powerstation`, make the power station missions 0700000c and 0700000d completable from start to end: the map, its actors and its objectives

Levels: `0700000c`, `0700000d` (`docs/systems/levels.md` has the table, `docs/atlas/` the systems). The goal is simple to state: a player
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

**Known so far:** not play-tested in the port. 0700000c has SimpleScript (0xE2) pulleys, a secret door and vent
meshes (`Door_secret1`, `obj_stor*_pul*`, `VentBlockingPoly`), and 0700000d a `shipping_handle`. Several are likely on
the route: port their activation and playback if your walkthrough needs them (`SS_Activate` 0x1a9ea8 /
`SS_Update` 0x1a9948), coordinating with `120-fix-explosions`. Check whether `VentBlockingPoly` is meant to be
invisible.
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

**Branch:** `s/map-powerstation`.
