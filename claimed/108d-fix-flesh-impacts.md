## Your task: `flesh-impacts`, bullet impacts on people: the visual effect

What the port does today when a player bullet hits a character (crates/nf-game/src/drone.rs, update_player_bullets'
drone-hit arm): the hitbox test, `NDrone2_DoHitEffects` 0x1458e0 (damage, the hit sounds 0x601 head / 0x413 body —
the main agent confirmed that function has no visual effect) and the location reaction. It does NOT run the bullet's
own contact effect for the character: in the game every contact goes through `Bullet_CollisionHandler` 0x12ce80 →
`Effect_Create` 0x1ce688 with the hit's material (`HITDATA+0x50`; docs/fields/hitdata-spawner-ally.md says 0x10 =
flesh/none, while crates/nf-game/src/effect.rs treats material 0x10 as GLASS — settle which, from the decompile and the
EffectInfo table 0x3037c0's row contents). Find exactly what the game shows when a bullet hits a guard, Bond or a
civilian: the material the character's collision reports, the EffectInfo row (puff/debris/mesh/sound), any
blood/spark sprite or decal on the character, the flesh pass-through rule (0x800 / material 0x10 [M 0x12ce80:136-141]),
and the same for guard bullets hitting Bond. Port what is missing. Tests (NF_ISO): a P2K body shot and head shot on a
05 guard spawn the game's effect (ids/counts) at the hit point; a wall hit is unchanged; sister assertions. Render a
hit (frame after the shot) before and after. PCSX2 captures (CRT filter) are fine for presence/placement.

Files: `crates/nf-game/src/{drone,effect}.rs`.

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

**Branch:** `s/flesh-impacts`.

---
claimed-by: sandbox2 at 2026-10-15T00:39Z
