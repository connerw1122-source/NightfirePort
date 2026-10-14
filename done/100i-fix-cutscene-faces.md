# fix-cutscene-faces — the cut-scene face tracks move mouths, not bones

Branch `s/fix-cutscene-faces` (master 966671a + `origin/s/fix-cutscene-draw` d1b5e08 merged at
61d2d8e). Final fix commit c5833d5. Continuation (07:32–09:11 UTC): merged `origin/master`
66651ba (fix-0c-hatch, fix-searchlights-guards, fix-grapple-hook) at 5d602b5; audit citation
correction deb1682.

## What was broken

The cutscene-draw fix (widest track owns every bone it covers) brought the cast back but failed
`fix_cutscenes2_live::the_kimiko_scene_characters_play_their_face_tracks` ("the face track must move
its bones (frames 399): the mouth animation") — the body track now owns the face bones, so the test's
bone-level mouth assertion could never pass.

## The diagnosis — the game's own classification and data, measured

The premise "the face track moves its (prefix) bones" was the weights-as-bones misread. Three
independent reads of the game say the face track is a MORPH track:

1. **The window classifier is the sequence's flags bit 0x8** [M `AnimProcessScriptCmds` 0x1c1b38:
   `uVar5 = *(int *)(lVar6 + 0xc) >> 3 & 1` on the raw sequence record; a facial window sets the
   script node's 0x40000 flag and plays in the node's +0x54 facial slot — docs/systems/anim-engine.md
   "+0x50 / +0x54 (body; facial)"]. [D measured] face seq 0x04000467 carries flags 0x08, body seq
   0x04000332 flags 0x00.
2. **The facial channels are the skin's facial morph weights** [M `AnimFacialBlend` 0x1be648 reads
   the playing sequence's frame buffer as the weight array; `SkinIt` 0x10bf30 consumes them with the
   (morph + channel) addressing, docs/formats/character.md "Facial morphs"]. [D measured on this
   scene] skin 0x0500003b's facial count 22 == the face seq's 22 channels; the values stay in
   [0, 1] (sweep: max 1.001, 312/433 sampled frames > 0.01), while the body seq carries
   222 = 3·73 + 3·1 channels (the skeleton's per-bone walk — the port's bit-exact [M WORKLOG 0017]
   sequential layout) whose root channels hold the actor's world position (83..91, 43.7, −35..−31).
   Read as bones, the face track's leading [0,1] weights zero the root — the missing-cast bug that
   started this chain. The "channel index → bone table" the merge report guessed at resolves to
   this: channel *i* of a facial-flagged track → facial morph weight *i* (the counts match by
   construction); the body track's channels → bones in the measured per-bone order.
3. **The guest's character carries the morph targets** the weights drive ([D] the level's
   MORPH_DATA block, fix-facial2's pairing) — the mouth animation is vertices moving under the
   weights, which `Scene::animate`'s CPU morph term renders.

On 07000001/05/06/07 the flags-bit classifier and the old channel-count heuristic agree on EVERY
sequence (26/26, 26/26, 28/28, 82/82 — zero disagreements, measured by a probe), so using the game's
bit is behavior-neutral where the suite pins and correct where a future skin's facial count differs.

## The fix

- `crates/nf-game/src/anim.rs` `script_pose_together`: facial-flagged windows (`seq.flags & 0x8`)
  are excluded from the base selection and the narrow fill — they own no bones. The body track (the
  widest non-facial window) owns the pose and the root; the cutscene-draw owned-mask fill for narrow
  non-facial pairs is kept. Cited inline.
- `crates/nf-view/tests/fix_cutscenes2_live.rs` `the_kimiko_scene_characters_play_their_face_tracks`
  re-pinned to the game's semantics, with sister pairs:
  - the face track carries the facial flag; its channel count == the skin's facial count; the guest
    carries morph targets;
  - the weights OPEN (max > 0.5 across the track's frames) and MOVE (the widest frame's vector
    differs from the closed frame 1) — the mouth animation;
  - the scene's live weights (`facial_weights_now`, what the renderer consumes) equal the track's
    weights at the object's frame;
  - the joined pose equals the body-only pose on EVERY bone (the facial window never touches bones)
    and the root stands at the library position (> 10 units from the origin — the cast at the
    scripted world positions, the cutscene-draw behavior kept).

## Verification

- `fix_cutscenes2_live` **5 passed / 0 failed** (the rewritten test included).
- Blast radius: `cutscene_draw_live` 2/2 (the cast render test), `cutscene_models_live` 4/4,
  `facial2_live` 2/2 (the ledge weights), nf-game `--lib` 140/0.
- **Rendered** (lavapipe, no GPU here): `nf-view $NF_ISO 07000007 --bond 94.2,44.6,-45.9,0,0
  --frame 399 --shot` and `--frame 1200` — the full cast (Kimiko, Bond, Dominique) at their scripted
  positions, the kiss plays, no origin collapse.
- **Independent audit (subagent, decompile-checked): PASS.** It verified the flags-bit read at
  AnimProcessScriptCmds 0x1c1b38:165, the +0x50/+0x54 slot split (AnimSeqSet 0x1c7460:17-18,
  AnimScriptNew 0x1c1398 allocates the facial slot as 88 B = 22 floats), AnimFacialBlend 0x1be648's
  float-array read, SkinIt 0x10bf30's separate morph path (Morph_Weight/Count globals,
  AnimObjectDraw 0x1c4490:143-154 marshalling), and that no out-of-scope file changed. Its one
  finding: this branch's inherited comment cited "psiBuildMatrixPalette 0x111498 reads
  sAnimObject+0x94" — the decompile reads the skin from **sAnimObject+0x24** (line 110); the +0x94
  skin reads sit on other bases ((obj+0xdc)+0x94 AnimListBuild 0x1c0fc0:114, obj+0x94
  AnimObjectDraw 0x1c4490:117). Corrected in deb1682 (comment-only).
- **Full suite on the final tree (master 66651ba merged)**, run with tools/test/suite.py
  (--test-threads=1, per-binary 600 s limit) — this sandbox kills background jobs between tool
  calls, so it ran as synchronous chunks over the 228 binaries; **totals: 951 passed, 4 failed,
  31 ignored**. The 4: the 3 allowed `disc_golden` failures (slim ISO) + `weapon_matrix_live`
  (see the open item below).
- `script_pose_drone` (the drone path, seed-pinned by the 07000006 driver trace) untouched — the
  facial flag agrees with its channel-count pickers everywhere measured, so there is nothing to
  change and no pinned behavior at risk.

## Shared files touched

- `docs/WORKLOG.md` — my entries appended at the end only.
- No shared source files: the fix lives in `anim.rs` (the task's file) + the test. `drone.rs`,
  `scene.rs`, `player.rs`, `lib.rs` untouched.

## Left / open

1. The GPU skin path (G-043) does not apply the morph weights yet (facial3's open item) — a
   GPU-path render would still-face; the CPU path morphs. The nf-view renders above were checked
   for the CAST (the task's ask); the mouth motion needs either the CPU path or the G-043 morph
   follow-up.
2. `Scene::facial_weights_now`'s window picker still identifies the face window by
   channel-count == facial count (works; agrees with the flag on every measured sequence). Moving it
   to the flags bit would touch `scene.rs` (shared) for no behavior change — left, listed here.
3. **`weapon_matrix_live` TIMED OUT on the merged tree** (this is new: it passed inside the
   previous session's 20-min full suite on the pre-merge tree). Evidence: with the suite's 240 s
   chunk limit it died mid-`matrix_0700000b` ("last started"); run standalone at the default 600 s
   limit it passed `matrix_07000001..08` quickly and was still inside the later maps when my
   590 s tool window closed — one of the later matrix-map tests takes > ~5 min (or hangs) on this
   tree. The only deltas vs the tree where it passed are master's merges (fix-grapple-hook's
   scene.rs draw sweep, fix-searchlights-guards' View_RotTransMatrix spot angles, fix-0c-hatch's
   ss.rs sweep rate) — none of which this task touches. Needs one uninterrupted run on the final
   tree (`python3 tools/test/suite.py . --only weapon_matrix_live`) plus, if it hangs, the same on
   origin/master alone to pin which merge did it.

## Live checks for the main agent

PCSX2 (07000007, the Kimiko scene) and nf-view at the same spot:
1. `--bond 94.2,44.6,-45.9,0,0 --frame 399` — the cast (Kimiko, Bond, Dominique) at their scripted
   positions in the library, the kiss playing (rendered here: confirmed).
2. `--frame 1200` — Dominique foreground, Bond/Kimiko dancing right (rendered here: confirmed).
3. The mouth: during the dialogue lines the actors' mouths move. In the game this is the facial
   morph weights driving the MORPH_DATA targets; in nf-view the CPU path shows it, a GPU-path render
   would not (open item 1). A close-up `--bond` spot facing Kimiko during a line would settle which
   path a given render used.
4. Trace request (gdbwatch): during the scene, watch the guest's morph weight array
   (`*(obj+0xDC)+0xB0`'s 22 floats) — expect values in [0, 1] oscillating with the dialogue, and the
   palette's face bones (jaw/eyes) holding near-bind values (the face track must not move them).
