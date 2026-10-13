# fix-facial3 — the face tracks re-pointed to the morphs, the blink driver banked

Branch `s/fix-facial3`, cut from origin/master 5017f15, 2026-10-13/14, session 22:54–00:54 UTC.
Item 2 (the face tracks' consumer) is landed and pinned; item 1 (the blink driver) is fully
implemented and unit-tested but NOT wired into the drone tick — the wiring's `Rand_*` draws move
the ai_traces pins EARLIER (a real fidelity regression, not bookkeeping) until the game's
`AnimObjectIsClose` gate is ported; item 3 (the GPU path) is designed and scoped, not landed.

## Done: the face tracks' re-point (item 2)

The disc refutes the working premise: **there is no all-zero face root anywhere.** Measured from
the ISO (07000006's anim archive 07F00006.bin): the driver's face track 04000065 is 22 channels of
morph weights — channels 0..2 reach 0.78/0.68/1.00 mid-window (first |w| > 0.01 at frame 170) —
and his BODY track 04000064's root runs un-zeroed on all 880 frames (net z 0.568, the documented
0.57). The game's own trace (tests/traces/05-truck.trace, the 07000006 segment) holds the seated
driver to x 0.024 / z 0.063 spans while the hip's y span 0.119 ≈ the BODY track's ch1 span 0.129 —
the suppression is the truck-driver state + the collision probe, not a zeroed track.

- **`script_pose_drone` / `script_pose_together`** (`crates/nf-game/src/anim.rs`): a window whose
  channel count equals the skin's facial count (`Skin+0x12` [M `AnimProcessSkinData` 0x1bd500]) is
  a face track — excluded from the base pick and the bone-overwrite walks in both posers. The
  bones 0..5 the face track used to own (the weights read as root/face-bone translations —
  degenerate rotations whose rebuilt w collapsed to 0) now come from the body track.
- **The drone root-motion path** (`crates/nf-game/src/drone.rs` `script_frame` / `root_value_at`):
  the same exclusion in the window pickers (the last-started NON-facial window owns the root; the
  fallback skips face tracks too). `AnimFacialBlend` 0x1be648 samples those channels as weights,
  never bone channels.
- **The drone weights source** (`crates/nf-view/src/scene.rs` `facial_weights_at`, new —
  `facial_weights_now` keeps its signature): a drone-backed character samples its own playing
  script's face window via `anim.script`/`ftime` (the same source `Drone::script_frame` reads);
  pre-fix `facial_weights_now` returned None for every drone (`c.script` was required).
- **`AnimFacialBlend`'s slots** in `facial_weights_at`: the drone's facial script slots (idle
  straight, blink overriding |w| > 0.01) blend over the track weights via the banked
  `nf_game::anim::facial_blend` — the composition the blink driver feeds once wired.

Tests (`facial3_live`, NF_ISO, 3/3):
- `the_face_track_no_longer_poses_the_driver_bones` — failing-first: `script_pose_drone(0600011d @
  300)`'s root translation equals the BODY track's ch1 (`04000064.value(1, 300)`), not the face
  track's weight value; the body/face disagreement at that frame is > 0.1. Pre-fix the last-started
  face window owned the root.
- `the_truck_driver_is_a_morph_source_and_holds_his_seat` — failing-first: the drone-backed
  character's `facial_weights_at` goes live (max 0.151 at tick 425 of his window; pre-fix None);
  the seat span stays bounded (the known gap below).
- `the_ledge_dialogue_mouth_moves_at_frame_767` — the task's named test: the 07000005 ledge scene's
  040007fa character rides to window frame 767 [D the track's first 0.01 crossing], the weights
  exceed 0.01 there, and the mouth-bearing vertices move by the morph path (the lockstep sister
  with the targets cleared cancels the bones).
- `fix_cutscenes2_live::the_kimiko_scene_characters_play_their_face_tracks` — updated to the
  corrected semantics (its old asserts pinned the weights-as-bones bug: "the face track must move
  its bones"); now pins that the joined pose is the body pose on EVERY bone and that the same
  window samples live weights.

## Done (banked, unwired): the blink driver (item 1)

`AnimFacialUpdate` 0x1be310 fully reversed and implemented as the pure `anim::facial_update`
(+ the `FacialCmd` return): the init's state-before-draw (`Rand(180)` + 90), the wrap's
`Rand(120)` → 120+d calm / 60+d alerted with table[4] (the blink), the female double-blink rolls
(`Rand(100)`: ≤50 done, 51..75 → `Rand(6)` + bit 0x80, ≥76 → `Rand_FRand(0.2)+0.2` start offset),
the alert-3 combat freeze, the talk-state exit; the male/female tables and `facial_blend` were
already banked. `Drone` gained `facial_state`/`facial_timer`/`facial_reappend`/
`facial_idle`/`facial_blink`/`last_tick` (own lines, both constructors). Unit test
`facial3_tests::facial_update_state_machine_and_draws` (nf-game lib 135/0).

**Not wired into `Drones::tick`** — the measured reason: with the per-tick call live,
`ai_traces_live::ai_05_truck`'s pins move EARLIER (drones 4–10: 2168 → 885, drone 0: 1342 → 1464)
— the port over-draws because the game's caller gate `AnimObjectIsClose(obj, 0xc)` (the min of the
anim object's four LOD/lighting bytes raw+0x110..113 ≤ 12 [M 0x1befd8:127-136, 0x1c66f8]) has no
port counterpart, so every facial drone draws from tick 0 where the game draws only for close ones.
Wiring it is the next session's first item, in this order: (1) port the LOD bytes or a measured
distance stand-in for `AnimObjectIsClose` (live check 2 below pins the real semantics); (2) wire
the driver at the top of the drone loop (before `update_anim` — `AnimObjectUpdate` :133 vs :137);
(3) re-pin the moved ai_traces/ai_live seeds by WORKLOG 0072 against the same live expectations.
The driver's other gates (the big-skeleton flag raw+0xC8 bit 2, the +0x58 bit-0x10000 facial-off
flag, the ScriptCam freeze) and the alert hookup (level 0 assumed: no combat freeze, no alert-row
idle faces) are also outstanding.

## Designed, not landed: the GPU morph path (item 3)

The scoped design (from the code survey; all sites verified): `CharVertex.morph` never reaches the
GPU (the skin buffer is `SkinAttr {a,b,w,col}`, no spare field; attribute locations 7/8 are free in
buffer 1); the shaders are inline WGSL in `gpu.rs` (`SHADER`, skinning at the `sa.x != 0xffffffff`
block); the palette uploads per frame via `update_skin`/`upload_skin` (gpu.rs:524-534; the
interactive loop interpolates palettes between ticks at lib.rs:1639-1653). Plan: (a) `SkinAttr`
grows `morph: i32` (the vertex's target index pre-based by the character's target offset) and
`wslot: u32` (the character's row in a per-frame weights table, stride 24 f32 for WGSL alignment);
(b) two storage buffers — the per-level concatenated morph targets (~248 KB on 07000005: 5227 +
3×5294 targets of 12 bytes, static disc data, uploaded at scene set) and the per-frame weights
(≤4 morph-bearing characters × 24 f32 there); (c) the vertex shader adds
`p += Σ_m w[m]·T[morph+m]` BEFORE the palette skinning (the CPU order, scene.rs's `morph_vertex`);
(d) the oracle (`gpu_skin_live`) stays green at its ticks 1/240 (all weights 0) and a morph-bearing
lockstep variant (targets cleared) extends it — the shader step's CPU mirror (`apply` in the test)
gets the same term. No time to land it correctly this session; nothing partial was committed.

## Known gap (pre-existing, measured)

The seated driver's mid-window drift: the port's cab floor-ray wobbles his y by ~1.0 and the
un-ported sit suppression lets the body root's small x/z run reach him (x span 0.559 over the
window vs the game trace's 0.024). The re-point did not cause it (the old face-root read was also
untested mid-window and moved him more); the game's mechanism is the truck-driver state
(`TruckDriverInit` case 1 sets the +0x694 yaw-accumulator — `DroneAnim_SnapRotate` 0x13e198 — not
a position pin, correcting the port's "sit target" comment). fix_drone3 and ai_traces stay green.

## Verification

- `facial3_live` 3/3 (NF_ISO, ~4 s); nf-game lib 135/0 (the driver unit test included);
  ai_traces_live 4/4, fix_cutscenes2_live 5/5, fix_drone3_live green, fix_truck_live 2/2 — all on
  the pushed tree.
- **Not run** (next session, before merge): the full four-crate sweep
  `NF_ISO=... cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1` — the risk areas (the poser changes' blast radius)
  are covered by the suites above, and nf-formats is untouched by this session.
- Determinism: no `Rand_*` draws landed (the blink driver is unwired; the re-point added none).
  Seed-pinned tests are unaffected on the pushed tree.

## Live checks for the main agent

1. **`AnimObjectIsClose` semantics** (the blink wiring's gate): PCSX2, gdbwatch on an anim
   object's +0x110..113 (the four bytes the min is taken over) during 07000001 — what writes them
   (distance-quantized LOD? ambient corners?) and their values vs the camera distance; expected:
   a step function the port can mirror. Address: the player anim block 0x018f2050 [L
   ee_start_07000001.bin], bytes at +0x110.
2. **The blink on a close guard**: PCSX2, stand next to a calm 07000001 guard (facial skin) — his
   eyelids close and reopen every few seconds; walk away and check whether the cycle stops (the
   LOD gate) and whether alert (gun drawn) freezes it (the alert-3 gate).
3. **The 07000005 ledge dialogue**: the mouth morphs visibly from window frame 767 (the render
   spot below), and character 70's head/neck now follow the BODY track (no more weight-driven
   neck wrenching mid-dialogue).
4. **The truck driver**: 07000006, the gate truck — seated, hands on the wheel, mouth moves when
   he talks; the port's remaining y-wobble (the cab floor ray) is the visual to compare.

## nf-view render spots

- `07000005`: teleport onto the Touch trigger placement 301 (out 0x43) — character 70 (face window
  040007fa): Mayhew's mouth moves from window frame 767; his neck/head track the body anim.
- `07000006`: `--bond 66.12,39.17,28.87` facing the gate truck — the driver seated; after the blink
  wiring lands, his blinks (table[4] = male 060000f3) every ~2-4.5 s when the camera is close.

## Files touched

`crates/nf-game/src/anim.rs` (the poser exclusions + `facial_update`/`FacialCmd` + the unit test),
`crates/nf-game/src/drone.rs` (`script_frame`/`root_value_at` exclusions, `facial_count`, the
Drone facial fields), `crates/nf-view/src/scene.rs` (`facial_weights_at` + the slot composition),
`crates/nf-view/tests/facial3_live.rs` (new), `crates/nf-view/tests/fix_cutscenes2_live.rs` (the
Kimiko test's corrected semantics — a shared test file whose premise this task overturns),
`docs/WORKLOG.md` (appended). No shared file rewritten; no `Rand_*` draws added.

## ntfy failed

All three transports returned the server-side daily quota (00:08 UTC, 2026-10-14 — the quota did
not reset with the UTC date):
- `curl`: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan","link":"https://ntfy.sh/docs/publish/#limitations"}`
- `python3 urllib`: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- `wget`: no output, same 429 server response.
