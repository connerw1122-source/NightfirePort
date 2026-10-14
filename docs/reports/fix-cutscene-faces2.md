# fix-cutscene-faces2 — cut-scene faces: merge onto master's facial rule

Branch `s/fix-cutscene-faces2`, 2026-10-14, worker 100i2. Task: merge `origin/master` into the
finished-but-unmerged `s/fix-cutscene-faces` base (316c759a), settle the facial-vs-body window
classifier on ONE rule — the game's own sequence-flags bit 0x8 [M `AnimProcessScriptCmds`
0x1c1b38: `uVar5 = *(int *)(lVar6 + 0xc) >> 3 & 1`] — everywhere master had used the channel
count (fix-facial3's stand-in probe), and keep master's exact-math code paths.

## What's done

1. **Merge 1 (2b47f3ec)**: `origin/master` (62c17571, fix-facial3 + the exact-math swap + the
   blink state machine) into the base. Three conflicts:
   - `crates/nf-game/src/anim.rs` `script_pose_together`, two hunks (base picker + the
     facial-window-skip in the window loop): kept the base branch's `flags & 0x8` classifier
     (`is_facial`) with its full evidence comment, dropped master's
     `s.channels.len() != skin.facial` probes. Master's per-bone loop assertion is carried by the
     kept `assert_eq!(body_only, together)` (strictly stronger).
   - `crates/nf-view/tests/fix_cutscenes2_live.rs`: kept HEAD's block (flags assert + count
     assert + morph-targets assert + the weights scans + `body_only == together` + the root
     position check); master's conflicting block re-asserted the same properties with weaker
     thresholds (max_w > 0.01 vs 0.5) and is fully covered by the kept text.
   - `docs/WORKLOG.md`: union merge, both sides' entries kept.
2. **The one rule applied to master's third site**: `script_pose_drone`'s `is_face` (auto-merged
   from master as a channel-count probe) now reads `s.flags & 0x8 != 0` [M 0x1c1b38], with the
   comment citing the game's rule and the measured counts ([D] 07000007 face seq 0x04000467
   flags 0x08, 22 ch == skin 0x0500003b facial 22; the 07000006 driver face track 04000065).
   These are the only three classifier sites in the tree (`grep -rn 'skin.facial'` — the
   remaining uses read the count as the WEIGHT count, which is its real meaning: scene.rs:5252
   and the test asserts).
3. **Master's exact-math kept**: `facial_update` (the `AnimFacialUpdate` 0x1be310 blink state
   machine with its `Rand_Rand`/`Rand_FRand` draws), `FacialCmd`, `facial3_tests`, and master's
   test-side weights machinery all ride the merge unchanged; the count-vs-flags swap does not
   touch any math, only which windows are classified facial.
4. **Merge 2 (0e7042c0)**: re-pulled `origin/master` (eb52fb56) before the final push — clean
   auto-merge (other workers' drone.rs/scene.rs/lib.rs hooks); my files untouched; targeted
   tests re-run green on the final tree.

## How it was verified

- **Reproduce/protect first**: the merged `the_kimiko_scene_characters_play_their_face_tracks`
  asserts the game's rule directly — `face_seq.flags & 0x8 != 0` for the first op-0 window of
  060004f1, `face_seq.channels.len() == skin.facial` (the by-construction count correspondence),
  `facial_weights` live values, `Scene::facial_weights_now` equality with the direct track read,
  `body_only == together` on every bone, and the root at the scripted world position (> 10 units
  from the origin). It failed on master's tree before the base branch's fix (the count probe was
  its stand-in) and pins the rule now. On the drone side `facial3_live` 3/3 (including
  `the_face_track_no_longer_poses_the_driver_bones` and
  `the_truck_driver_is_a_morph_source_and_holds_his_seat`) proves the flags swap is
  behaviour-identical where the count probe was verified: 04000065 carries bit 0x8.
- **nf-game lib on the final tree** (merged with eb52fb56): 152 passed / 0 failed — master's
  facial3_tests (the `AnimFacialUpdate` 0x1be310 state machine) and every anim unit test green
  with the flags classifier in place.
- **Targeted tests, final tree** (merged with eb52fb56): fix_cutscenes2_live 5/5,
  fix_cutscenes_live 4/4, facial3_live 3/3, facial2_live 2/2, cutscene_draw_live 2/2,
  cutscene_models_live 4/4 — 20 passed, 0 failed.
- **Renders (lavapipe), final tree** — the three task shots, all correct:
  - `nf-view Nightfire-onfoot.iso 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 399 --shot` →
    renders/faces2_07_f399.png — the kiss plays, Bond + Kimiko at the scripted spot, a third
    guest entering left.
  - `--frame 1200` → renders/faces2_07_f1200.png — the full cast: Dominique foreground, Bond +
    Kimiko dancing right, faces present (mouth morphs ride the CPU skinning; the GPU-skin path
    without morphs is still facial3's open item).
  - `07000002 --bond -82.30,5.30,32.05,1.570,-0.100 --frame 170` → renders/faces2_02_f170.png —
    both actors present and lit dark (Bond in the doorway, the doctor at the monitor desk).
- **Full suite** (`tools/test/suite.py`, 270 test binaries, NF_SILENT, slim ISO, foreground
  chunks — the nohup background run dies between tool calls in this sandbox, the 102c/100f
  precedent): **1074 passed, 3 failed, 35 ignored, 1 TIMEOUT**. The 3 failures are exactly the
  allowed `disc_golden` trio (slim-ISO golden dumps). The TIMEOUT is `weapon_matrix_live` at its
  600 s cap — the base branch's final run carried the identical TIMEOUT as its open item; the
  binary contains zero references to the facial/pose classifiers this task touched (grep: 0
  hits), and the weapon-matrix workers are still landing volume on it (s/weapon-matrix4b/5
  appeared during this session).

## What's left

- `weapon_matrix_live` 600 s TIMEOUT on this 2-core sandbox + slim ISO — pre-existing open item,
  owned by the weapon-matrix task, not facial-related.
- The GPU-skin path still doesn't apply the facial morphs (facial3's open item, unchanged); the
  CPU skinning path renders the moving mouths.
- Nothing of this task's scope is open: the merge is settled on the game's rule, everywhere.

## Files

- `crates/nf-game/src/anim.rs` — the three classifier sites (two conflict hunks in
  `script_pose_together`, the `is_face` swap in `script_pose_drone`), comments citing
  [M 0x1c1b38].
- `crates/nf-view/tests/fix_cutscenes2_live.rs` — conflict resolution, kept the flags-rule
  assertions.
- `docs/WORKLOG.md` — union merge + this task's entry.
- Shared files: none touched by me (drone.rs/scene.rs/lib.rs changes in merge 2 are other
  workers' auto-merged code, unmodified by me).

## Live checks for the main agent

- PCSX2, 07000007 Kimiko scene (the party hall): during the dialogue the cast's MOUTHS move
  while the bodies play the scripted tracks, and nobody stands at the world origin. Spot:
  Bond 94.2, 44.6, −45.9 area.
- PCSX2, 07000002 intro at the lab door: both actors visible in the dark corridor shot, the
  doctor's face reacts while he talks (blink at ~2-4 s idle: `AnimFacialUpdate` 0x1be310).
- nf-view re-render on the merge candidate:
  `nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 399 --shot` and `--frame 1200`,
  `nf-view <iso> 07000002 --bond -82.30,5.30,32.05,1.570,-0.100 --frame 170` — compare with
  renders/faces2_07_f399.png, faces2_07_f1200.png, faces2_02_f170.png.
