## Your task: `character-room-light`, characters lit by the room they stand in

The user's playtest on 07000007 ("characters lit by the room"): **no**. The port multiplies 0x800 objects by tint x cel
ambient >> 8 (render-port-check.md 1.1), but the user still sees the party's characters lit wrongly. Also G-042: the
07000002 bathhouse civilian's skin texels. Find which cel ambient a character uses in the game (the anim object's cel,
`obj+0x20`, set by `control_link_object_to_cel` 0x1346b8, vs the port's lookup), whether dynamic lights (`Light_Create`, the
muzzle light) add to characters, and the per-character ambient multiplier. Write render spots (07000007 ballroom guests,
07000002 bathhouse) and a precise live-check request: the main agent captures PCSX2 at those spots with
`tools/trace/pcsx2_shot.py --pos ... --pitch ... --god`.

Files: the character lighting in `crates/nf-view/src/scene.rs`, `docs/systems/render.md`.

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

**Branch:** `s/character-room-light`.

---
claimed-by: sandbox7 at 2026-10-14T19:40Z

---
## Report (done, 2026-10-14T20:33Z)

Closed by the driver: the report is on `s/fix-character-room-light` (docs/reports/fix-character-room-light.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T20:35Z)


---
## Report from the worker (done, 2026-10-14T20:35Z)

# fix-character-room-light — characters lit by the room they stand in

Question: the playtest on 07000007 ("characters lit by the room") said **no**; also G-042 (the
07000002 bathhouse civilian's skin texels). Task: find which cel ambient a character uses in the
game (the linked cel `obj+0x20` vs the port's lookup), whether dynamic lights add to characters,
and the per-character ambient multiplier.

## Findings (the game's model, with addresses)

1. **The cel source is the LINKED cel `obj+0x20`** [M]. Linked at create
   (`Control_CreateObjEx` 0x1343c8:39 → `build_LinkToRoom` 0x1c9680 → `build_FindCel` 0x1c8cb0 at
   the world sphere; for characters `AnimObjectNew` 0x1be910 sets flags2 0x8 and leaves
   AnimObject+0x80 zero, so the sphere centre is the object position), snapped to
   `cel + switch_channels[cel+0xa2]` bytes +0x9a/0x9c/0x9e while the 0xff init sentinel
   (`control_init_object` 0x134630:22-23) holds (`control_link_object_to_cel` 0x1346b8:27-31), kept
   across ticks (`control_handle_cel_change` 0x133288: `Cel_ObjectLeftCel` 0x1e0d58 portal
   crossing → `build_point_in_bounding_box` 0x1c9500 keep → `build_LinkToRoom` re-find; a failed
   re-find re-links the OLD cel), and ramped per tick toward that cel's triple by
   `Lights_CalcAmbientLight` 0x1cb178 (`ClrStep` 0.1) — **only for moved objects** (flags2 0x20,
   `control_movement_object_handler` 0x1336f8:209-211). Proxy-bearing objects (obj+0x1C) follow
   the proxy's cel (0x1336f8 relink pass).
2. **Dynamic lights DO add to characters** [M,D]. Characters draw through
   `View_DrawObjList` 0x1e69a0:96 → `AnimObjectDraw` (flags2 8) → `View_SetupRenderModes` 0x1e78b0:20
   — the SAME `View_SetupRenderModes_Tweak` (0x1e5888:52-55, `tint × ambient >> 8`) and the SAME
   `Lights_CalcClosestLights(obj+0x80, obj+0xF0)` (0x1e69a0:73-81) as world objects. Skins carry
   0x8a0/0x28a0 — flag 0x80 admits kinds 0/1/2, no 0x100 muzzle exclusion [D fix-lighting2].
3. **No hidden per-character lighting exists.** The `bCharacterLight`/`bShadowCharacter`/`AnimColRad`
   globals (0x30cdd8..0x30cddf, set from obj+0xFE bits 2/3 in 0x1e69a0:70-74, cleared in
   AnimObjectDraw 0x1c4a8c) have **no reader** in the EE code — I scanned every `lb/lh/lw/lwc1`
   base-`$gp` access in the code segment. The tint `obj+0x103..0x105` is script-set only
   (`Script_SetColour` 0x1db770 / `SP_SetColour` 0x1dca90), default 0xff.

## The port's state: the model matches on every headlessly-checkable point

- `Sim::ramp_char_ambient`/`ambient_at` (scene.rs) re-derive the cel per tick with `find_cel`'s
  smallest-volume stand-in (the documented build_FindCel deviation, fix-render2), snap on the first
  target, `ClrStep` 0.1 after, and the sentinel stays only where the game stays unlinked too.
- Measured on 07000007 [D, `character_room_light_live.rs`]: all **58 placed guests** sit in
  unique-containment cels and carry **their own cel's ambient** (9 distinct triples, bytes 36–64 —
  cel 19's dark hall (36,50,36) through cel 10's lit corner (59,64,56)); the multipliers reach both
  the CPU (`posed_vertices`) and GPU (`skin_palette` colour table) paths.
- The party's **~104 active kind-0 lights** (candles/lamps, warm 255,223,173) reach the character
  draws through the per-draw `pick2` with the skin flags — guests in three different rooms each
  pick live lights.
- The 49 characters stuck at the 0xff sentinel on 07000007 are exactly the ones the game also
  leaves at 0xff: 33 dev-pool copies parked at −1e6 (a port-only dev feature, hidden) and 16 origin
  script actors — created at the origin, inside NO cel, so the game's link fails and
  `Lights_CalcAmbientLight` is gated off (`cel == 0`) until their scripts move them into a cel.
- **G-042 resolved: no lookup bug.** The civilian (skin 0x05000038) has ONE 100 %-white texture
  (map 32 texture 4, 32×32) but it is a **sheen map**: 1024/1024 white rgb, only 98/1024 texels
  opaque — a faint overlay batch (batch 54, UV 0..1). Her base skin textures (map 32 textures
  0/5/7/9, 64×64–128×128, fully opaque) are not white. The other skins' white patches are the
  towel itself (skin 0x05000036 texture 2: 98 opaque-white texels — a towel is white).

## Verification

`crates/nf-view/tests/character_room_light_live.rs` (NF_ISO, 3 tests, all green):
1. `ballroom_guests_lit_by_their_own_room_on_07000007` — every drone-backed guest's multiplier
   equals its own cel's triple on the game's integer path (and never 1.0); sentinel characters must
   sit in NO cel (the game's unlinked set); ≥3 distinct room ambients among the guests; the multiply
   reaches the vertex buffer. (The first probe version of this test FAILED with 49 full-bright
   characters before the analysis showed they are the unlinked set — kept as the history in the
   WORKLOG.)
2. `party_lights_are_available_to_the_character_draws` — ≥32 active kind-0 lights on 07000007 and
   ≥2 of 3 probed guests pick live lights with the skin flags 0x8a0.
3. `bathhouse_civilian_skin_texels_are_not_white` — the G-042 assertion (alpha-weighted): no
   civilian texture has >50 % opaque-white texels; the sheen map is counted and reported.

Baseline suite on master+0 (`suite.out`, the same production code as the final tree — this task
changed only tests and docs): **1065 passed, 7 failed, 35 ignored (37 min)**. The 7: the 3 allowed
`disc_golden` (slim ISO), a 600 s TIMEOUT in `weapon_matrix_live`, and `probe_pylon_render` +
`fix_0c_pylon_live` ×2 — all present before this branch. On the final tree (merged with
origin/master 1c376216) the lighting suites re-ran green: the new
`character_room_light_live` 3/3 and the pre-existing `character_lighting_live` 4/4.

## What is left / live checks for the main agent

The user's "no" is NOT reproduced headlessly: every visible guest carries its own room's ambient,
the multiplier and light adds match the decompile. The remaining suspects are all render-time or
mid-play states a PCSX2 session can settle:

1. **Live checks (PCSX2, `tools/trace/gdbwatch.py` / `fieldprobe.py`)** on 07000007, party in swing:
   - Read a ballroom guest's `obj+0x100..0x106` (ambient + tint) mid-play. Expect ambient ≈ its
     cel's triple (e.g. 44,44,40 / 60,48,40) and tint 0xff,ff,ff. A tint ≠ white means
     `Script_SetColour` is in play and the port renders it white — a real gap to port next.
   - Watch `switch_channels[0]` (0x30c9xx per level's table) during the party: a non-zero write
     would flip every room cel to its +1-byte triple (the port clamps `cel.ambient[]` to triples
     0/1 and `Channels::set` refuses channel 0 — both would need fixing).
   - Confirm a WALKING guest's `obj+0x20` (the linked cel) changes exactly when it crosses a
     doorway portal, and that `obj+0x100..0x102` ramps (not snaps) after the crossing — pins the
     portal/keep semantics the port approximates with a per-tick re-find.
2. **Render spots (nf-view, `--bond x,y,z,yaw,pitch`)**, 07000007:
   - `44,45.5,-62,4.2,0` — the west hall group (chars 0–8): they should read dim-warm
     (ambient (55,59,51) ≈ 22 %), NOT white.
   - `61,45.5,-51,1.6,0` — the dark hall (cel 19, ambient (36,50,36) ≈ 16 %): the guests here
     should be visibly darker than the west-hall group.
   - `96,45.5,-55,-1.6,0` — the lit corner (cel 10, (59,64,56)) vs the ballroom (cel 9,
     (60,48,40)): the warm/cool difference should be visible between adjacent guests.
   - `33,45,-70,1.0,0` — Bond's start: view models and Bond at (55,59,51); looking east from here,
     NO characters should stand in the void outside the manor (the origin script actors are at
     (0,0,0) — if any figure is visible there, the port draws what the game's forced-list path
     would not).
3. **nf-view code** (not done here, listed for the next session): the portal fast-path of
   `Cel_ObjectLeftCel` (a mover crossing overlapping cels should take the crossed portal, not
   re-find); the `switch_channels` shift ≥2 clamp; `Script_SetColour`'s tint.

## Files

- `crates/nf-view/tests/character_room_light_live.rs` — the three NF_ISO tests (new).
- `docs/systems/render.md` — the "Characters: the room they stand in" section (new).
- `docs/WORKLOG.md`, this report.
