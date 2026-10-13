# fix-facial — the facial morph decode + the blend arithmetic (G-020/G-030)

The port's faces were frozen: it parsed no facial data at all. Branch `s/fix-facial` from master
13abef4. Everything landed cites the decompile address; the full decode is in
`docs/formats/character.md` §"Facial morphs".

## What the decode established

1. **The skin's facial count** is disc `+0x12` → runtime `AnimSkin+0x2e` [M `AnimProcessSkinData`
   0x1bd500]. `AnimObjectNew` 0x1be910 mallocs `count × f32` (zeroed) per character as the morph
   weight array (`animData+0xb0`, read through sAnimObject `+0x40`). Measured [D]: 22 on every
   face-bearing skin — the same count as the cut-scene scripts' 22-channel face tracks
   (docs/formats/animation.md: 07000005's ledge pair 06000996/97, the 07000006 truck driver).
2. **The weights come from the face tracks' sequence frames** [M 0x1be648 `AnimFacialBlend`]:
   the three facial script slots (markers 0xe0000000 idle / 0xe0000001 blink / 0xe0000002 talk)
   are read as `script+0x54 → sAnimSeq+0x84` — the sequence player's per-channel float frame
   buffer (`AnimSeqNew` 0x1c7248 / `AnimFrameSet` 0x1c75a0) — and blended into the weight array:
   slot 0 copies straight; slots 1/2 override only |w| > 0.01; the anim object's eye bytes write
   +0x97 → indices 14 (value/255) / 13 (1 − value), +0x98 → index 11 (value) / 12 (inverse), each
   over the same 0.01; under a script camera ONLY slot 2 applies, straight copy.
3. **The idle/blink driver** [M 0x1be310 `AnimFacialUpdate`]: state byte +0x99 / timer +0x92 over
   `Male_Facial_State_Anims` / `Female_Facial_State_Anims` — [D ACTION.ELF .data 0x2c6ea8 /
   0x2c6ec0, read via the ELF symbols: male 060000f8/060000f9/060000f5/060000f6/060000f3, female
   0600036d/0600036f/0600036a/0600036b/06000370; table[0] = idle, table[4] = blink] — with
   `Rand_Rand` draws for the blink cadence (RNG note: the driver adds draws when it runs).
4. **The weights reach the vertices** [M 0x1c4490 `AnimObjectDraw` → 0x111828
   `psiDrawSkinObjectMatrix` → 0x10bf30 `SkinIt`]: near objects only (the min of animData
   +0x110..0x113 < 13; no gate under a script camera), the (weight, index) selection sorts, and
   per vertex the skinning word's upper half-word (the short at +0x1E, −1 = none) indexes
   `target_base + (vertex_index + Morph_Index[m]) × 12`, weight-scaled and added.

## Landed

- `crates/nf-formats/src/character.rs`: `Skin::facial` (+0x12).
- `crates/nf-game/src/anim.rs`: `facial_blend` (the exact 0x1be648 arithmetic, both the normal
  and the ScriptCam path), `facial_weights` (the sequence's channels sampled as the morph weights
  through the existing `Sequence::value` — the port's `AnimFrameCopy`-exact evaluation), and the
  two state tables as constants.
- Tests `crates/nf-game/tests/facial_live.rs` (3, green, NF_ISO):
  - `the_dialogue_skins_carry_the_22_morphs` — skin 05000007 facial == 22, plus the archive's
    other dialogue skins; the tables' values pinned. Pre-fix the parse dropped +0x12 entirely —
    the field did not exist, which is the failing state this pin closes.
  - `the_face_track_samples_to_morph_weights_over_the_dialogue` — the real 07000005 face track
    (sequence 040007fa from 07F00005.bin, the 06000996/97 pair) sampled over the first 60 ticks:
    every tick's 22 weights equal the channel evaluation (the decompile's `AnimFrameCopy`
    arithmetic), and the ScriptCam branch passes the talk slot straight. The track's first weight
    crosses the 0.01 threshold at frame 767 of 1500 — the dialogue's mouth movement.
  - `facial_blend_matches_the_decompile_arithmetic` — the threshold/override/eye-index branches
    against hand-derived 0x1be648 cases (sister assertions per branch).
- Docs: `docs/formats/character.md` §Facial morphs (the whole decode with addresses).

## What's left (the vertex half)

- **The morph target buffer's disc location**: the skin-info +0xC word is the slot-bone list
  (port-validated), not the targets; a morph-bearing gfx block (0x04000048, 356 morph vertices,
  indices up to 5272) implies a ≳63 KB target array inside the block, and the runtime gfx record's
  +0x14 (SkinIt's target base) writer is a dropped store in the decompile. Once the base is
  pinned, the skinning upload applies `pos += Σ weight[m] × target(base + (idx + index[m])·12)` —
  the "a face vertex moves" test and the nf-view hook land then.
- **Re-pointing the face tracks at the weights**: `script_pose_drone` currently poses BONES from
  the 22-channel tracks ([L]-pinned by the 07000006 truck-driver immobility trace). The decode
  says those channels are morph weights, so the truck's immobility must then come from the
  cut-scene zeroing the body track's root — a live check first (below), then the rewiring.
- `AnimFacialUpdate`'s state machine (blink idles in gameplay) — the tables are banked; the
  wiring needs the anim-object state bytes in the port's drone anim loop.

## Suite status (honest)

nf-game: 32 result-groups green (incl. the 3 facial tests). nf-formats + nf-audio: 9 groups green;
the only failures are the 3 allowed `disc_golden` slim-ISO stubs and `pss_movies_live`'s 1 (needs
NF_MOVIE_ISO — the retail image; environmental). nf-view targeted subsets green: character_lighting
4/4, fix_cel_order 2/2, explode 2/2, fix_npcs 4/4, fix_weapons_all 4/4, walk_0700000d 3/3,
walkreal_07000001 2/2, ai_traces 4/4 (the cut-scene/anim traces — the change's blast radius), plus
the walk suites 01/02/03/05/06/07 (all result-groups green, 19:14 UTC). The remaining nf-view
binaries did not fit the window; re-run before merge.

## Live checks for the main agent

1. **Morph targets**: gdbwatch `psiCreateEntityGfx` (0x1171b0) during a level load for the store
   to the gfx record's +0x14 (the value should be a pointer into the loaded PS2_GFX block) — that
   pins the morph target base; then read 12 floats there for a face mesh and compare against the
   block bytes (the probe in `crates/nf-formats/tests/facial_probe.rs` prints the candidates).
2. **Truck driver**: at the 07000006 truck (start script 0600011d), watch whether the body track's
   root channel carries non-zero root motion while the driver holds position (if yes, the
   cut-scene zeroes it — freeing the face tracks to be re-pointed at the weights).
3. **Blink cadence**: fieldprobe a male guard's animObject+0x99/+0x92 in idle — expected: +0x99
   cycles 0→1→2 with +0x92 counting down at 60 Hz, and the blink script 060000f3 appended at the
   wrap.

## nf-view render spots (after the vertex half lands)

- `07000005` ledge cut-scene (06000996): Mayhew's mouth moves during the dialogue from ~frame 767.
- `07000001` Bond close-up: the eyes blink on the idle cadence (060000f3) and track the eye bytes.

## Files touched

`crates/nf-formats/src/character.rs`, `crates/nf-game/src/anim.rs`,
`crates/nf-game/tests/facial_live.rs` (new), `crates/nf-formats/tests/facial_probe.rs` (new, the
evidence probe), `docs/formats/character.md`, `docs/WORKLOG.md`. No shared file rewritten; no
Rand_* draws added to the sim (the facial state machine is not wired yet).

## ntfy failed

All three fallbacks failed with the server-side quota error (sent 19:13 UTC):
- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget: exit code 8 (server error), no body.
