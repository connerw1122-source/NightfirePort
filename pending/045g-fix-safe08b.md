## Your task: `fix-safe08b`, finish the 07000008 bedroom: the take, and room-scoped collision and drawing

**Start from `s/fix-safe08`** (3e7dad4; read `docs/reports/fix-safe08.md` first). It found both root causes:
- the safe's items are spawned by the SafePickUps script 0x060009E2 (SP #286 on ch 136) through
  `Script_CreateEntity` 0x1d8968's 0xF0 arm. That's ported: the shelf pickups exist at the game's key positions. But
  `fix_safe08_live::the_safe_pickups_take_after_the_safe_opens` is still red: the take's `Collide_LineOfSight`
  (flags 0x721) is blocked in the port by a hit the game's query evidently ignores (t 0.65–0.88 on the rays).
- the blue block is `Room_Outside_3` (map51 obj16), the exterior room's geometry poking into `Room_Guardhouse_F2`.
  The game never draws it or collides with it from indoors: drawing is the `Vision_Portal_Recurse` cel set
  (0x1e86e8 → 0x1e82e0), collision is `Collide_Pick` 0x1eb320 over the cels `Collide_StraddleCels` 0x1eac98 collects.
  The port draws by frustum per cel (the portal walk `portal_visible_cels` exists but is NF_PORTAL-gated since the
  07000006 incident) and collides with one flat static world.

**Do:**
1. Scope the static collision by the room/portal graph as the game does (`Collide_StraddleCels`: the cels the query
   straddles via `Intersect_Portal`, cap 0x7e), first for the actor queries (`capsule_push`, the walk), then the
   rest; and evaluate un-gating the draw's portal walk on this level (the walk from every room must keep at least
   the room itself; re-check the 07000006 case). Coordinate with `130-fix-portal` (pending), which owns the general
   portal draw: if you do the draw side, say so in your report so 130 can be trimmed.
2. Decode the LOS query-type filter (HITTEST +0x94/+0x98, 0x201/0x721 in `Collide_Intersect` 0x1ebaa8's switch, the
   per-node +0x3c word) enough to name the class the game skips, and make the take fire. The main agent can run the
   live checks in the report (the GDB watch on the take's `Collide_LineOfSight`, the straddle cel dump) with the
   user: ask for exactly what you need.
3. Run the FULL suite (all four crates) before pushing: the previous session couldn't.

Tests (NF_ISO, fail-first): Bond walks from the bedroom door to the safe without the block (the capsule passes where
`Room_Outside_3`'s tris are); the take fires for both items after the safe opens; the sister (no take before ch 136)
stays green.

Files: `crates/nf-game/src/world.rs`, `crates/nf-view/src/scene.rs` (the cel graph, the draw), the pickup/LOS path,
`crates/nf-view/tests/fix_safe08_live.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/fix-safe08b`.
