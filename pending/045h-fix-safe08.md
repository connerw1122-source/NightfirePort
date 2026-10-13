## Your task: `fix-safe08`, the 07000008 bedroom: the blue block and the safe's pickups

**The user's check `safe08` (2026-10-12, after fix-party-mission merged):** "cant pickup the briefcase or rocket
launcher, big … obstacle in the way". Two screenshots:

1. **A huge dark-blue block of castle EXTERIOR stonework stands in the middle of the bedroom** (spot
   `07000008 --bond 155.19,43.12,-113.38,-3.837,0.010`, frame 6587: night-blue stone with the exterior frieze, filling
   the room from floor to ceiling beside the open safe). It also blocks movement. This is the "giant pillar" of the
   first report; fix-party-mission assumed it was the locked guard-room door, which it is not. Find the object (the
   scene's draw list / placements at that spot): an exterior piece placed or drawn where the game never draws it (a
   wrong map, a portal/cel visibility error, an open/closed pair, a 0x1000 cage, a LOD/destroyed variant). Fix the
   draw AND the collision against the game.
2. **The safe opens, but the briefcase and the rocket launcher inside can't be picked up** (spot
   `07000008 --bond 161.87,43.39,-116.54,1.583,-0.495`: both visible on the shelves, Bond right in front). Find why the
   pickup doesn't take (the pickup's enable/hide channel, its collision volume or radius, the safe door's collision
   still covering them, the pickup kind for the briefcase (type 4, LABEL)), and make both take as in the game; the
   briefcase writes ch 0x4A (`Pickup_Handler` 0x18e650:330-333).

The fusebox's real laser cut is `045h-fix-fusebox-cut`'s; keep the E stand-in until that lands.

Tests (NF_ISO, through the real input path, fail-first): Bond walks from the fusebox to the safe without the block in
the way, and takes both items; the sister: the briefcase is not taken before the safe opens.

Files: `crates/nf-view/src/scene.rs` (the draw), `crates/nf-game/src/{pickup,ss}.rs`, `crates/nf-formats/src/level.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

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

**Branch:** `s/fix-safe08`.
