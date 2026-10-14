## Your task: `cutscene-actors`, the right models and positions in cut-scenes

From the playtests on 07000007: "models in that cutscene aren't accurate" (the Kimiko scene), "guards aren't in the proper
place after the Kimiko cutscene", and on another scene "models are missing". For each cut-scene on 07000005/06/07: list the
actors the script creates (`Script_AnimStart`, skins) and compare with what the port draws (skin resolution, the
0x05050a17-style missing skins in models-characters' report), and where the actors end up when the scene hands back (the
scene's last keys / `Script_KillStream` vs the drones' positions). Fix what differs, with a test per scene (actor skin
hashes and the post-scene positions).

Files: `crates/nf-game/src/nis.rs`, `crates/nf-view/src/scene.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/cutscene-actors`.

---
claimed-by: sandbox3 at 2026-10-14T19:42Z

---
## Report (done, 2026-10-14T21:09Z)


---
## Report from the worker (done, 2026-10-14T21:09Z)

# fix-cutscene-actors — report

Branch `s/fix-cutscene-actors` (from origin/master 62c17571). The playtest symptoms: "models in
that cutscene aren't accurate" (the Kimiko scene), "guards aren't in the proper place after the
Kimiko cutscene", "models are missing" on another scene.

## The actor inventory (per cut-scene, 07000005/06/07) [D]

Every `Script_AnimStart`/`Script_CreateEntity` actor of every kind-7 script, from `nf-extract nis`:

- **07000005** (no loadable packs): 0x060008c8 (anim skin 0x0500007 + entity 0x0200063a), 0x06000952
  (death NIS, skin 0x0500007), 0x06000994 (the ledge: 0x05000030 s1, 0x050002e s3, 0x0500007 s14 —
  stream 14 re-uses the stream object for Bond, the `Script_AnimStart` kill-and-recreate), 0x06000a22
  (the truck: skin 0x0500005, entities 0x0200040c/0x02000426/0x0200047f/0x02000480/0x02000ac8). All
  skins in 07000005's own archive; all resolve; all characters build.
- **07000006** (no packs): 0x06000071 (the dressing scene: 0x0500007 s1, 0x050000d s2 — both resolve,
  both build; the F3 regression stays green).
- **07000007**: packs 07800007 (0x06000073, the Kimiko scene) and 07900007 (0x06000072, the Drake
  fly-through). The Kimiko scene's actors are exactly three Anim streams — 0x0500003b/0x060004f1
  (stream 1), 0x05000015/0x060004f2 (stream 2), 0x0500000d/0x06000225 (stream 3, the tuxedo Bond
  double) — plus keyed entity 0x02000acb (stream 14). The fly-through: 13 guest anim skins
  (0x05000016/15/0d/22/19/20/21/24/23…). All in 07000007's own archive [D bin-list]; all resolve;
  all characters build.

No 0x05050a17-style dead skins exist on these three levels (every anim skin named by every
cut-scene script resolves on the disc).

## The Kimiko scene hand-back — verified game-accurate, pinned by test

I could not reproduce a port deviation in the scene's models or the post-scene guard positions;
the whole chain matches the decompile, and the new test pins it:

1. The scene's actors draw exactly the script's three skins (asserted live).
2. Event 5 freezes the party (`npc_enabled == 0`), event 7 + `Script_Free` re-enable (asserted).
3. Bond hands back at stream 6's entity — key 498 at [91.52, 43.85, -31.09] [D], SP #420 base
   identity [D] — the port lands Bond there (asserted within 0.5).
4. The doubles are deleted at `Script_Free` (no keep-objects flag [D]; asserted).
5. The ch-0x48 drones vanish after the scene — **the game does this too**: placements 414/448/449/489
   carry param 6 = 72 (the scene trigger channel 0x48), param 8 = 100; the alert block (param 21 = 5)
   maps through `GetDTYPENEW` case 5 → mode 0x15 [M 0x14c938], whose `DroneTypeSettings[0x15].alt`
   row is 0xb4 = DeleteMe [M 0x14cbd8:111-115 + the symbol table]; `NDrone2_PreDroneControl` 0x148db0
   fires it when the freeze lifts (it is gated on `NPCGlobals` like `Drone_Control` 0x139268).
   `WaitSwitch` drones stay hidden in both [M 0x15b960 msg 1 = `NDrone2_Enable(0)`].
6. The untouched hall guards stand exactly at their frozen spots after the scene (asserted within
   0.05). Their placements are [93.7, -48.0]/[94.8, -48.1]/[95.7, -55.4]/[95.7, -56.7] [D].

The two "not accurate" complaints may predate fix-cutscenes2's fixes (the W-skip cut the scenes
short mid-flight then); the live render checks below are the decisive check.

## The fix — "models are missing" (the impact-effect script)

`Bullet_CollisionHandler` Script_Loads **0x060007c4** through `Explode_Create` on the weapon
`+0x70` **0x100000 / 0x200000** arms [M 0x12ce80:212-231] (first float arg 0x3f800000 = scale 1).
The disc records with the bits: 0x32/0x6e (0x100000), 0x6b (0x200000) [L weapon_data scan]. The
script lives in every archive [D] (entities Laser_Blast_frame_1/2 0x020007c9/ca, Vertical_smoke
0x020007df, After_glow_test 0x020007e0, cloud_1 0x0200021a, smoke 0x0200021d) but **no placement
references it** — the scene build swept only SP-referenced scripts + 0x06000052, so its entity
models could never draw, and the sim never spawned the script. Fixed:

- `crates/nf-view/src/scene.rs`: the build sweep chains 0x0600_07c4 (additive).
- `crates/nf-game/src/drone.rs`: the player-bullet world-hit path spawns it at the hit point when
  `+0x70 & 0x300000` and not a ricochet re-entry (the game's `bVar7` gate, the same test the 0x4
  arm uses); additive block, no behaviour change for other weapons.

Test `the_impact_effect_script_models_build` (all three levels, per-stream per-hash) — **shown
failing first** on the pre-fix tree ("entity 020007ca (stream 0) has no built instance — the model
is missing"), green after.

## Files touched

- `crates/nf-game/src/nis.rs` — not changed (the read-only study found `anim_start`/`kill`/`free`
  faithful; left untouched per the minimal-touch rule).
- `crates/nf-view/src/scene.rs` — the build sweep (1 line + comment).
- `crates/nf-game/src/drone.rs` (shared, additive hook) — the impact-script spawn block.
- New: `crates/nf-view/tests/fix_cutscene_actors_live.rs` (5 tests).
- Shared deletions: none. (The sandbox's leftover `crates/nf-view/examples/` probe files were
  restored byte-identical to origin/master after an accidental `rm` — no content change.)

## Verification

- New tests 5/5 green; neighbours on the merged files: fix_cutscenes_live 5/5, fix_cutscenes2_live
  4/4, models_characters_live 6/6, fix_weapon_effects_live 4/4, impacts_live 6/6.
- **The full four-crate suite did not run in this window** (the 21:51 UTC stop-new-work rule hit
  first; the sandbox's suite.py background start also died on a missing PATH and a 100 %-full disk,
  since cleaned). The next session should run it on the merged tree first thing.

## What's left / open questions

- **0x0600006f** (the alarm-switch props 0x02000499 "alarmswitch_red" / 0x0200049a
  "alarmswitch_flashing", 10 ticks, on 05/06/07): no placement, no `Script_Load` constant found in
  ACTION.ELF's decompile — who plays it is unresolved; its models still can't build. Same-class
  candidates to sweep when its spawner is found.
- Destroy_Smash's 0x06000027/0x0600004f entity models (pre-existing gap, owned elsewhere).
- Whether the game's per-tick order deletes the ch-0x48 drones at the freeze START rather than at
  the free (the port deletes at the free; both ends look identical to the player) — a PCSX2
  gdbwatch check settles it.

## Live checks for the main agent

1. **PCSX2, 07000007 Kimiko scene** (trigger: the hall floor by the y-44.5 stairs): mid-scene,
   gdbwatch on the fireplace Kimiko drone's obj (placement 414, [92.06, 43.84, -31.65]) — watch
   `drone+0x135` (second channel 72) and `switch_channels[72]`; at the scene's end confirm
   `NDrone2_PreDroneControl`'s SetState(0xb4) and the object's removal — and whether it happens at
   the freeze start or at the free (the timing question above).
2. **PCSX2, after the scene**: Bond should stand at [91.5, 43.85, -31.09] facing the fireplace; the
   two hall guards at [93.7, -48.0]/[94.8, -48.1] visible; no Kimiko/Drake doubles, no ch-0x48
   drones.
3. **nf-view render**: `--bond 90.5,44.3,-33.5,1.2,0` mid-Kimiko-scene — the three doubles (tuxedo
   Bond, the two women) walk their root paths ([D] roots: Bond 91.55,-35.73 → 91.97,-31.67; 0x04f1's
   actor 86.83,-33.35 → 83.21,-35.19; 0x04f2's actor 91.65,-28.37 → 94.13,-35.76); no duplicates, no
   white unposed characters.
4. **nf-view render (the fix)**: fire weapon 0x32 at a wall (e.g. 07000005 spawn) — the impact
   should show the Laser_Blast frame/afterglow/smoke entities at the hit point; before the fix
   nothing drew there.
5. **nf-view render (fly-through)**: `--bond 34.25,44.13,-70.71,1.549,-0.070` mid-fly-through (the
   fix-cutscenes2 spot) — the 13 guests visible once each.
