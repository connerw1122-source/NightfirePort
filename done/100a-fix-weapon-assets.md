## Your task: `weapon-assets`, every weapon's model, animations and effect meshes on every level

CONTEXT (the user, 2026-10-13): "need heavy work on getting weapons to work no matter what level, with sounds and
proper animations and particles." The dev tools (the dev window / F2 menu, `--give id,id`) hand Bond ANY weapon on ANY
level. Today `scene.rs` builds a first-person view model only when the weapon's first-person skin (`weapon_data+0xDC`),
its sleeve and its parts are in the CURRENT level's archive (`level.skins.get(&fp)`, scene.rs "The first-person gun
models"), and the same holds for its view-model animation scripts, its muzzle-flash mesh (`weapon+0x48`), its casing,
its projectile and its sounds (the level's own sound bank). So a weapon the level doesn't natively carry is invisible,
silent, unanimated or fires the wrong effects (the user saw the grapple fire a bullet impact, the laser and stunner fire
"bullets", no Q-Claw model). The real game only ever loads a level's own set; the port goes further on purpose, so that
every weapon behaves exactly as it does on a level that carries it.

The four 100x tasks split the work: 100a the assets and animations (the foundation), 100b the sounds, 100c the fire
behaviour and effects, 100d the all-weapons x all-levels check. They run in parallel: pull `master` often, and if
100a's branch (`s/weapon-assets`) isn't merged yet, build on it (`git merge origin/s/weapon-assets`) and say so in
the report. Keep the per-level behaviour bit-identical for weapons the level carries (the existing tests stay green).

YOUR PART: a disc-wide weapon asset index and loader.
1. Index: for each weapon id 0x01..0x71, which level archives (the FILES.BIN `.bin` entries; fix-anim-events parsed
   all 99) carry its first-person skin, the sleeve (scene.rs SLEEVES), its extra parts, its view-model animation
   sequences (draw/holster/idle/fire/reload/alt: the scripts `weapon_data` names at +0xA0/+0xA4/+0xC0/+0xC4 etc., see
   gun.rs and docs/systems/player.md "Weapon switching"), its muzzle-flash mesh (+0x48), casing mesh, projectile mesh
   (rockets, grenades, bolts, darts, the Q-Claw hook), its third-person (held/pickup) model and the textures they use.
   Build it once per ISO and cache it (a small file next to the build or in the user's temp dir, keyed by the ISO's
   size and a SHA-1 of the archive table; never game bytes in the repo).
2. Load: when a scene is built, and when `--give` / the dev menu gives a weapon the level lacks, pull that weapon's
   assets from the owning archive into the scene, the way scene.rs already does for drone skins from other archives
   ("drones with a skin from another archive"): meshes, textures, skeleton, sequences, rebased into the scene's
   tables. A lazy load on first give is fine if loading every weapon up front is slow (measure; report the times).
3. The draw/holster/fire/reload animations play from the loaded sequences with the game's timing, the muzzle flash
   attaches to datum 0 with the record's colour, and the held/dropped world model shows.
Tests: for every weapon id with a first-person skin anywhere on the disc, on a level that does NOT carry it (07000005
for most): give it, run the draw, fire once, reload; the view model has its skin's vertex count, the sequences
advance, the flash attaches. On a level that DOES carry it, the result is identical to before (numbers, not images;
sister assertions). List the ids that have no first-person skin anywhere on the disc (expected: hands, some gadgets)
with the reason.

Files: `crates/nf-view/src/scene.rs` (the loader; small hooks), a new `crates/nf-view/src/weapon_assets.rs`,
`crates/nf-view/src/lib.rs` (the `--give` hook), tests.

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

**Branch:** `s/weapon-assets`.

---
claimed-by: sandbox4 at 2026-10-14T02:32Z

---
## Report (done, 2026-10-14T04:06Z)


---
## Report from the worker (done, 2026-10-14T04:06Z)

