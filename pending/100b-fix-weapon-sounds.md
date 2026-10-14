## Your task: `weapon-sounds`, every weapon's sounds on every level

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

YOUR PART: the sounds. Each level's sound bank (`nf-audio` sfx.rs, `scene.sound_bank`, docs/systems/sound.md) carries
only the effects that level uses. Map every weapon's sound ids (fire, the silenced variant, dry fire, the reload
stages, draw 0x281/0x282, holster 0x280/0x283, zoom, alt fire, the laser and stunner loops, the grapple's fire and reel,
grenade pin/throw/bounce, rocket launch and flight loop, explosions, casings per surface, impacts per surface material:
from `weapon_data`, the animation events fix-anim-events ported, and the effect tables) to the bank(s) that hold them.
When a weapon is given on a level whose bank lacks a sample, load that sample (its VAG data and effect record) from a
bank that has it and register it in the running driver under the same id, without disturbing the level's own effects
or the voice limits. Tests: for every weapon on 07000005 (and one indoor level), fire/reload/draw request each sound
id and a voice with that id's sample starts (the sample's hash = the owning bank's; numbers only); a level that
already has the sample uses its own (unchanged); the laser loop starts on press and stops on release (101e may also
touch this: merge its branch if it lands first); sister assertions.

Files: `crates/nf-audio/src/{sfx,lib}.rs`, `crates/nf-view/src/lib.rs` (the hook where sounds are requested), tests.

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

**Branch:** `s/weapon-sounds`.
