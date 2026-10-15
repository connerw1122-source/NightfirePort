# fix-heads-07 — the 07000007 sideways/detached heads (user 07000007, playtest 1014-1725)

Branch `s/fix-heads-07` (master 0404d95b + the fix, then merged origin/master 2e5a162b at db3dfe49).

## What was wrong

At the four F12 spots the guests' heads looked awful in two ways, one cause:

- **Spots 1 and 3** (`--bond 39.78,44.10,-70.86,-4.083,-0.108` frame 635, `--bond 40.58,44.13,-70.77,0.682,-0.080` frame 1488): a guest's head lies sideways on his shoulders/back, face pointing up — the head **position** leaves the neck while its orientation turns.
- **Spot 2** (`--bond 41.71,44.13,-72.05,-1.488,-0.154` frame 1102): the hair mesh rides as a jagged black patch at the **collar** instead of on the head.
- Spot 4 (`--bond 33.40,44.10,-70.63,1.569,0.000` frame 43, before any tracking) is fine — which pinned the symptom to the head-**tracking** path, not the skin, the morphs or the base anims.

## Root cause [M `AnimObjectHeadTrack` 0x1bf618 via `psiBuildMatrixPalette` 0x111498]

The game turns ONLY the head's orientation:

- HeadTrack resolves the tracked bone's palette entry — `0x1bf6ec sll $v0, $v1, 6` + `0x1bf6f0 addu $v0, $t1, $v0` (palette + bone·0x40; the palette entry is 0x40 bytes: rotation rows 0..0x30 then the translation at +0x30, docs/fields/animobject.md +0xB8) — and hands **the entry itself** to `Quat_QuatToMat` 0x1e3e80 (`0x1bfcc0 lw $a1, 0xd8($sp)`, `0x1bfcc4 jal 0x1e3e80`).
- `Quat_QuatToMat` 0x1e3e80 stores offsets **0x00..0x2c only** (`swc1` to ($a1), 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28 — the three rotation rows). The +0x30 translation row is **never rewritten**: the head keeps its anim-posed position on the neck; only its rotation turns.
- The port multiplied the head bone's FULL 4×4 world matrix by the two track rotations (`world[5] = world[5]·R_pitch·R_yaw`), which rotates the translation row about the object origin too. With the near-idle angles 101c tested this was invisible; at the **saturated** track angles a ~1-unit guest drives (yaw to the ±1.1 clamp, pitch to the up-clamp — measured `head_yaw ±1.100 / head_pitch +1.000` [probe]) the head position swung ~0.5 units off the neck onto the shoulders/back, and attachment meshes anchored to the head bone — the hair — followed it down to the collar.

Measured pre-fix at the spot-3 placement (real input path, `Sim::step_bond` ×1488): neck→head `dir.y` **0.308 / 0.298** on the two saturated trackers, head-to-neck length **0.488** — against `dir.y` **0.97..0.99**, length **0.397..0.40** on every non-tracking head in the same frames (the anims hold the head up on the neck).

## The fix

`crates/nf-game/src/drone.rs`, one hunk in `Drone::object_pose_inner`'s head-track block: compose the two track rotations on the **rotation rows only** (zero the translation row for the product) and restore the anim's translation row afterwards. The composition itself — `q_yaw ⊗ q_pitch ⊗ q_world_head` premultiplied in the pose's own axes (yaw about the neck axis, pitch about the side axis) — is 101c's `fix-head-look` work, unchanged. This is a shared-file edit (one hunk), listed per the task's shared-file rule. `scene.rs` and `anim.rs` turned out NOT to need changes: the bug was not in the skin/UVs, the morph path or the scene assembly.

## Verification

- **Fail-first**: `crates/nf-view/tests/fix_heads07_live.rs` `tracked_head_stays_on_the_neck_at_the_playtest_spot` — real input path (`nf_view::load` + `Sim::step_bond` ×1488 at the spot-3 F12 placement): FAILED pre-fix (neck→head 0.488, dir.y 0.308 — the detached head), passes post-fix (0.397 / 0.986 on both saturated trackers; `head_yaw ±1.100 head_pitch 1.000` proves the track still RUNS).
- **Sister**: `moderate_track_still_aims_the_head_at_bond` (spot-2 placement, the 101c regime |yaw| 0.15..0.9): the head still aims its forward at Bond's eye (0.019 rad), stays upright (up.y 0.975), and the hair anchor rides the head bone at the anim's neck length (0.397) — the fix didn't freeze or flatten tracking.
- **Old sisters green**: `head_look_live` 2/2, `gpu_skin_live` 3/3, `fix_cutscenes2_live` 5/5, `facial3_live` 3/3, `facial2_live` 2/2, `fix_ai_remainder_live` 1/1 — the premultiply composition 101c pinned still holds.
- **Full sweep** (NF_ISO, release, `--test-threads=1`; run per-crate/chunk because the sandbox kills long foreground commands): **nf-game 276 passed / 0 failed / 2 ignored; nf-formats + nf-audio 87 passed / 3 failed — exactly the 3 `disc_golden` allowances; nf-view 840 passed / 0 failed over all 267 targets** (chunks 241 + 317 + 264 + 18). **Total 1203 passed / 3 failed / 2 ignored**, the 3 = the documented slim-ISO `disc_golden` trio.
- **Final tree**: merged `origin/master` 2e5a162b (fix-sg5-casings; WORKLOG both sides kept) and re-ran the head sisters + sg5's own suite on the merged tree: 14/14 green.

## Renders (before/after)

Headless (`nf-view $NF_ISO 07000007 --shot out.png --frame N --bond …`, lavapipe in a user prefix):

| spot | before | after |
|---|---|---|
| 1 f635 | head swung sideways off the neck | head upright on the neck, natural idle |
| 2 f1102 | hair patch at the collar | hair on the head, face clean |
| 3 f1488 | head lying on the upper back, face up | head on the neck, glancing back/up at Bond |
| 4 f43 | (no tracking yet) | unchanged |

Files: `/home/z/my-project/download/heads07/before/*.png`, `/home/z/my-project/download/heads07/after/*.png` (outside the repo — renders are not committed to the public repo).

## The other head aspects the task listed

- **Head skin texture/UVs**: the faces read correctly in the after-renders (eyes/brows/mouth/hair); no texture or UV defect found at these spots.
- **Facial morph state (108c coordinate)**: NOT mid-morph garbage — 108c's GPU morphs are **not merged**; the CPU facial path (fix-facial3) left in master is what renders, and the mouths/eyes behave at all four spots. When 108c lands, re-render these spots once.
- **Lighting (108b coordinate)**: 108b (shading parity) is **not merged**; the faces carry the pre-108b shading (the spot-1 face has a slightly cool cast against the warm scene). Out of this task's scope — 108b owns it.
- **Head bone pose (101c's head angle)**: the composition was already fixed by 101c; the remaining defect was the translation row above.

## Exact reproduction steps for the PCSX2 compare (main agent)

1. Play 07000007 (real-input build, `nf-view <iso> 07000007 --dev`) to the party hall; the guests near the clock (≈ x 41, y 44, z −71) track Bond when he stands close (inside the 3.0 head-track range).
2. Headless port pairs (before = `origin/master` 0404d95b, after = this branch): the four commands above; the saturated trackers are the two guests either side of the clock (drones 4/5), state 0x30, script 100663739.
3. In PCSX2, stand Bond ~1 unit from a guest: the game's guest keeps his head **on his neck**, craned up/side toward Bond at the clamps; the port (pre-fix) throws the head onto the back and drops the hair to the collar.

## Live checks for the main agent

- **PCSX2**: 07000007, Bond 1.0–1.5 units in front of a party guest, slightly below his eye line (the spot-3 geometry): expected — head stays on the neck, pitches up toward Bond no further than the game's clamp; verify the guest's head ORIENTATION at the clamps matches the port's after-render (`after/spot3_f1488.png`) — the angles are saturated, so the exact craned pose is the one visual to compare.
- **PCSX2**: the same guest at ~2.5 units (the 101c regime): head upright, tracking Bond smoothly (already verified by 101c's live checks; re-confirm after this fix).
- **nf-view**: `--bond 40.58,44.13,-70.77,0.682,-0.080` frame 1488 — the two guests flanking the clock keep their heads on their necks; `--bond 41.71,44.13,-72.05,-1.488,-0.154` frame 1102 — the hair sits on the head, not the collar.

## Left / open

- The 5 binaries the last chunk cut (wire_live, wire_repro, world_fx_live, worldobj_live, zz_obj_drives_probe) DID run and passed (18 tests, chunk 4) — the nf-view total above is complete.
- `fix_guard_first_shot2`'s flagged STRAFE-LOCK (guard 20 never arriving) is untouched by this task (no drone.rs AI-path edits — the one hunk is render-path only).
- 108b/108c merges should re-render these four spots (one command each) to confirm the composed result.
