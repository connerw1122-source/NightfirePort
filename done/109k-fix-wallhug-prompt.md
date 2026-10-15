## Your task: `wallhug-prompt`, the wall-shimmy prompt: range and third person; the claw's impact sound

User (05 trench ledge): "near the wall edging area I kept seeing a prompt and when I entered a prompt it was in first
person. should be 3rd person." and "That wall walking ledge area, its interaction prompt area is a bit too large. Like
I'm walking away from it and I still see it." Port the game's wall-hug (creep wall 0x3F) prompt volume (its exact
size/test) and the third-person camera during the shimmy (`Player_PositionCamera` for the creep substate). Also the
Q-Claw: "hook latched, pulled me in, but I also heard a bullet impact" - the claw's bolt must not play the bullet
impact sound on the hook.
This comes from the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`, mouse +
keyboard). The user was very disappointed: several merged fixes passed their tests yet were wrong or broken in play.
Your test MUST drive the real input path (`Sim::step_bond` with the `Input` the window builds - lib.rs maps the mouse
and keys), not internal setters, and must fail first on master. The main agent will compare against PCSX2 after you
finish; put the exact steps to reproduce in the report.

Files: `crates/nf-game/src/player.rs`, `crates/nf-view/src/{scene,lib}.rs`.

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

**Branch:** `s/wallhug-prompt`.

---
claimed-by: sandbox1 at 2026-10-15T04:01Z

---
## Report (done, 2026-10-15T05:27Z)

Closed by the driver: the report is on `s/fix-wallhug-prompt` (docs/reports/fix-wallhug-prompt.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-15T05:28Z)


# fix-wallhug-prompt — the wall-shimmy prompt's volume, the third-person shimmy camera, the Q-Claw's silent latch

Branch `s/fix-wallhug-prompt` (from origin/master d2ce47e5, merged again before push). Files:
`crates/nf-view/src/scene.rs`, `crates/nf-view/src/lib.rs`, `crates/nf-game/src/drone.rs`
(+ the tests `crates/nf-view/tests/fix_wallhug_prompt_live.rs`, `fix_wallhug_camera_live.rs`, and one
spot fix in `crates/nf-view/tests/fix_08_walls_live.rs`). Playtest 1014-1725, level 05 (07000005).

## 1. The shimmy prompt's trigger volume (user: "its interaction prompt area is a bit too large. Like I'm walking away from it and I still see it.")

**Root cause.** The port armed the ctx-6 "Press Action" icon with a flat [S] window —
`dist(bond.pos, icon.centre) < 6.0` (measured to Bond's FEET). The game has no flat window at all:
`Player_CollisionHandler` 0x1949a8:22 clears `BLData+0x95F = 0xFF` every tick and
`ThirdIcon_Update` 0x196e80 writes the context only into the player object named by the icon's own
obj+0xD0 straddle chain — whose candidates `Collide_Pick` 0x1eb320 admits ONLY when the picker's
sphere meets the candidate's world sphere, `Vec_SqDist3D(picker+0x60, obj+0x80) <
(picker+0x6C + obj+0x8C)²` [M LAB_001eb4e4], and `Collide_Intersect` 0x1ebaa8 (LAB_001ebf04)
re-tests sphere-sphere before the chain keeps the pair.

**The fix** (scene.rs, the icon scan): `dist(bond.sphere_centre(), icon.centre) < 2.5 + icon.radius`
— the game's sphere test, still gated by the icon's straddle-cel set (`Collide_StraddleCels`
0x1eac98's portal BFS, the port's `icon_cels`).

**The picker radius is the hittest's own +0x8C** (copied to +0x6C at 0x1eb320:99) — a pick-sphere
field, NOT the object's world sphere (1.0576 [L M_CONTROL+0x8C], which the previous tasks used as
the hint-marker stand-in). The measured pins bracket it:
- 05 crawl out-point (−34.10, 37.28, −28.65) arms at 3.00 (sphere centre) from icon[1] (r 2.2757 [D probe]) → picker > 0.73;
- 05 grapple spot (16.5, 29.6, 58.6) stays silent at 8.17 from icon[3] [L PCSX2 2026-10-14] → picker < 5.9;
- 05 gate door stays 0xFF at 24.06 [L PCSX2 2026-10-13] → picker < 21.8;
- 07000008's west-ledge cable grab arms at 3.47 from the r-1.1638 icon [D probe] → picker ≥ 2.31.
**2.5 sits inside every bracket [S]** — the live read is listed below.

**Verification.** `fix_wallhug_prompt_live::the_crawl_prompt_arms_only_inside_the_games_sphere`
(real input path, 07000005): the mouth reads ctx 6 and posts the 0x1000291 "Press Action" hint;
2.2 further out (5.3–5.7 from the icon, inside the old 6.0) reads 0xFF and posts nothing — **FAILED
on the pre-fix src** ("walking away must drop the prompt ... dist 4.87, got 6"); sisters: the
grapple spot and the gate door stay silent. `fix_swim_vent5_live` (the vent crawl spot), `fix_context_hints_live`
(6/6) and `fix_08_walls_live` (4/4, its spot now documented against the pick-sphere shell) all green.

## 2. The shimmy camera (user: "when I entered a prompt it was in first person. should be 3rd person.")

**The game.** `Player_CollCreepWall` 0x196758:114-127 holsters the weapon (`Player_WeaponNone`),
sets cam mode 0xB and the camera radius 4.0; `Player_PositionCamera` 0x18f5d8 case 0xb draws Bond
(`View_SetDrawInAllViews`), hides the view model (`View_SetDrawInNoViews(BLData+0x7e8)`) and runs
`camera_tracking` 0x1ba5e0 every creep tick: the tracked point is the closest point of the wall's
axis line with both ends' Y +1.0 to Bond's position (the line clamp [M
vecutil_calculate_closest_point_on_line]), and the camera sits at `closest −
Cartesian(4.0, wall.yaw + π, −2.7486268)` [M 0x1ba5e0:106-110, the creepwall pitch 0x402fede0] —
in the port's convention `closest + axis_dir·1.5317 + up·3.6952` — looking at the tracked point
(the tail atan2s). A wall-specific ThirdPersonCameras spline exists only when the wall's BLData
u16[1] (placement param 1) is set — 05's two 0x3F placements carry none [D probe], so the computed
spherical is the live path.

**The fix.** `Sim::creep_camera()` (scene.rs) ports the above; lib.rs's tick camera consults it
(one hook, after the script-camera and missile-ride hooks) and `view_model_entries` skips during
the creep (the view-model hide, the same pattern as the missile ride). The port has no player body
mesh, so the game's drawn-Bond half of case 0xb stays unrendered — the camera is third person but
Bond's body is not drawn (pre-existing port limitation, not a regression).

**Verification.** `fix_wallhug_camera_live::the_shimmy_camera_is_third_person` (real input path):
E at the vent mouth crawls, `creep_camera()` is Some, sits > 1.0 off Bond's eye, > 1.0 ABOVE his
head, and looks down (pitch < −0.5); on foot it is None. **On the pre-fix src the file does not
compile** (`no method named creep_camera` — E0599, documented). The main agent's render check owns
the look.

## 3. The Q-Claw's bolt impact sound (user: "hook latched, pulled me in, but I also heard a bullet impact")

**Root cause.** The claw records 0x50/0x51 carry `weapon_data+0x70 = 0x40` — the grapple bit, no
effects bit [L ee_start_05.bin @ 0x2bf150 + 0x50/0x51·0x10C]. `Bullet_CollisionHandler`
0x12ce80:187-190 raises the effect flags to 0x61 only with `+0x70 & 2`, and `Effect_Bullet`
0x1ce8a8:86-90 plays the `EffectInfo+0x14` impact sound only on flags 0x40 — the claw bolt's world
hit (the hook latch) is SILENT in the game. The port played the surface impact unconditionally at
both world-hit sites (drone.rs ~6116 the player bullets, ~7897 the drone bullets).

**The fix.** Both sites gate the impact sound on `weapon_data+0x70 & 2` (the laser gate at site 1
kept; site 2's duplicate `flags70` read unified). The records without bit 2 (the stunner 0x4A/0x4C
0x800, the gadgets, the melee) fall silent with it — the game's own rule.

**The user's exact path.** The port latches when the aim ray's pick accepts (`pick_grapple_point`),
swallowing that tick's trigger; when the aim misses the pick on the press tick (the user's run),
the fire machine spawns the REAL claw bolt, which dies on the wall — the impact the user heard.
(The forward castle wall at the pulley has no collision at the bracket gap — the bolt dies at its
50-range cap unsounded there; the mechanism under test is the bolt's world hit, whichever wall.)

**Verification.** `fix_wallhug_prompt_live::the_q_claw_latch_plays_no_bullet_impact` (real input
path, the user's spot): the away-facing wall shot — no impact id from the bank's whole impact
column while the bolt flies, the reel and latch silent too, and the latch still fires — **FAILED on
the pre-fix src** ("impact sound 8 played"); sister: the same harness with the P2K (+0x70 = 0x12
[L]) DOES hear its impact — the silence is the record bit, not a dead sound path.
`fix_weapon_sounds_live` 3/3, `gadget_laser_live` 7/7, `qclaw_live` 3/3, `gadget_grapple_user_spot`
2/2, `grapple_hook_live` 3/3, `wire_live` green.

## Live checks for the main agent

1. **The pick-sphere radius [S 2.5]** — gdbwatch the player hittest's `obj+0xd4+0x8C` (and `+0x6C`)
   on 07000005 while walking toward the vent (predicted: a constant ~2.5, well above the object
   world sphere's 1.0576 [L M_CONTROL+0x8C]); also on 07000008 pressing jump at the west ledge.
   If it reads ~1.05, the 08 ledge grab needs another arm path and the bracket shrinks.
2. **The 05 prompt range** — fieldprobe `BLData+0x95F` walking away from the trench-ledge vent:
   predicted none past ~4.8 from the ctx-6 icon centres (the old build showed 6 out to 6.0).
3. **The shimmy camera** — enter the crawl on 05's trench ledge; predicted: third person, the
   camera ~3.7 above and ~1.5 along the wall from Bond, looking down at him; the view model hidden.
4. **The claw** — fire at the wall from the playtest spot (and at the pulley): predicted NO bullet
   impact sound on any claw-bolt world hit; the latch and reel stay silent.

## nf-view render spots (--bond x,y,z,yaw,pitch)

- `07000005 --bond -34.10,37.28,-28.65,3.14,0.0` — the vent mouth: the "Press Action" prompt shows.
- `07000005 --bond -33.0,37.28,-31.0,0.0,0.0` — one step back toward the wall: still prompted.
- `07000005 --bond -31.5,37.28,-26.0,0.0,0.0` — walking away: NO prompt (the old build prompted here).
- Enter the crawl at the mouth (E): the third-person tracking camera, high and along the wall.
- `07000005 --bond -18.28,38.19,-71.83,-1.57,0.0` with the claw: fire at the wall — no impact sound.

## What's left

- **The full `tools/test/suite.py` did not produce a total line this session**: this sandbox reaps
  background processes between tool calls (two nohup starts died silently with empty logs), and the
  foreground window could not fit all 292 binaries. What ran instead, foreground, on the final
  tree: `nf-game --lib` 160/160, `nf-formats` 74 tested — 11+63 green with EXACTLY the 3 allowed
  `disc_golden` failures (bigf_header_fields, bigf_tables, directory_tree — the slim-ISO class),
  and every at-risk neighbour binary green (the list above). Next session: `tail -1 suite.out` on
  the merged tree.
- The jump-grab's pick-sphere radius (live check 1) — if the live read disagrees with 2.5, the
  constant moves; the pins' bracket is in scene.rs's comment.
- Bond's third-person body during the shimmy (the game draws it; the port has no player body mesh).
- The ThirdPersonCameras spline path (walls with BLData u16[1] set) is not ported — none of 05's
  walls carry one [D probe].
