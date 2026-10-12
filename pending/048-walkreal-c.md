Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `walkreal-c`, play the final missions (15, 16, 1b, 4a) for real: walk every mission route with the player's own movement, no teleports

Levels: `07000015`, `07000016`, `0700001b`, `0700004a`. The map tasks (032–037, merged) proved each mission's logic with headless walkthroughs
(`crates/nf-view/tests/walk_<code>_live.rs`, `docs/levels/<code>.md`), but those tests **teleport Bond between
rooms**. A player can't. This task proves each route can be physically walked, and fixes what stops it.

For each level:
1. **Build the route** from the level doc: the start, every objective, every required interaction and the exit, as
   waypoints. Use the AI path graph (`nav`) and the doors and ladders between them.
2. **Drive Bond with input only:** a test (`crates/nf-view/tests/walkreal_<code>_live.rs`) that steers with the
   real `Input` (turn toward the next waypoint, forward, crouch/jump/use where needed, ladders and wires through
   their own systems). No `bond.pos =` writes after the start. Enemies may be disabled for the walk (say so), but
   doors, lifts and triggers must work as in play.
3. **Every time Bond gets stuck, falls or dies, that's a bug:** find why (a collision hole or invisible wall, a
   step too high, a door that doesn't open, a trigger that never fires, a missing ladder or lift, a floor he falls
   through) and fix it from the game's code (collision `docs/systems/drone-collision.md`, the player's movement
   `docs/systems/player.md`, doors `docs/atlas/door.md`). The user has seen an invisible wall at the stairs on
   07000008 (`060-fix-objects2` has it; coordinate through its report if it's on master).
4. The test passes when Bond walks from the start to the level end with the objectives completing on the way
   (or as far as a known logic blocker; then say which and stop there).

**Known logic blockers** (`045-fix-finale-blockers`, likely on master by the time you start): Kiko's defeat on 16, the missile consoles on 1b, the lift on 4a. Walk as far as each goes; if 045 has landed, walk to the end. 1b is a space station (zero gravity in places: the player substates 8/9 `Player_ZeroG`), so check those movement modes.
**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in
your report. Put new behaviour in new files. Pull `master` before you start and before every push: other workers
merge into it all day.

**Report:** per level, walkable end to end yes/no, every stuck point found and fixed (with `--bond x,y,z,yaw,pitch`
so the main agent can render each spot), and anything left.

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

**Branch:** `s/walkreal-c`.
