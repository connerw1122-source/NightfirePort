# fix-render4 — what I found, what's verified, what's left

Branch `s/fix-render4` (from master 923595b). Playtest reference: docs/playtests/2026-10-11.md.
Honest headline: **no library code changed this session.** Both headline bugs are now measured to
their exact objects and leaves — the guesswork is gone — but neither is fixed yet, and the report
says precisely why and what the next pass does.

## The flat light-blue floor quads (07000007 and the following levels)

**Found which objects and leaves draw them.** They are the levels' baked shadow decals:
`hall_shadows1/3/4`, `OBJECT_banistershadows1..4`, `hall_shadow_banister01/02` [D 07000007.bin, main
map 47; flags 0x221/0x21/0x20]. "Banistershadows" — the quads near the stairs and railings are
exactly these. They draw from textures `002f:003e` and `002f:0055` (64×64, RGB ≈ black, mean 3,
sd 0 — the shadow shape lives in the alpha channel 0–255) as list-1 batches (object flags & 1) with
ALPHA_1 0x44 (normal blend), TEST_1 0x5001B (only A=0 discarded), per-leaf ZBUF, vertex rgba ≈
(14,14,14,67). In the game these are faint dark soft-edged shadows on the floor.

Everything the "missing texture / wrong blend" theories predicted, I measured, and none of it is
true in the port (headless, `crates/nf-view/tests/probe_render4.rs`, NF_ISO):

- **0 untextured World groups** (of 4,869 World materials on 07000007) — the port never falls back
  to a flat colour for these;
- **0 out-of-range texture REFs** across every map;
- **0 coronas (0xFC) and 0 searchlights (0xDE)** on 07000007 and 07000008 — not their quads either;
- the texture each decal binds is pixel-identical to the game's: `psiCreateEntityGfx` (0x1171b0)
  resolves each REF through the 0x10 texture-header entry to a runtime record's +0x1A global id
  [M], and `psiCreateMapTextures` (0x11d200) dedups textures by pixel CRC [M] — so even where the
  game's global id differs from the map-local index, the pixels are the same;
- `psiStartAlpha` (0x124900) / `psiEndAlpha` (0x124920) only toggle `AlphaFogObjects` when `FogOn`
  [M] — the port's list-1 distance fade gates on exactly that, so the fade matches too.

**What's left and why.** There is no GPU in this sandbox, so I cannot put the port's actual output
next to your screenshots. Every per-pixel state I *can* read headlessly (texture content, blend,
alpha test, depth write, list routing, fade gating) matches the game. The two candidate deltas for
the next pass, in order:

1. **`bShadowCharacter` (obj+0xFE bit 3)** — `View_DrawObjList` (0x1e69a0) reads it per object and
   feeds the hand-written VU1 pipeline; the port ignores obj+0xFE entirely. If the game's VU1
   forces these shadow objects' output colour/alpha, that's the visible difference.
2. **R2's depth rule** — the game's 24-bit integer depth with GEQUAL and coplanar decals vs the
   port's f32 depth with LessEqual: z-fighting shows the decals through floors (your R2
   screenshot), and through-floor decals would read as flat shapes lying where they shouldn't.

What should change on screen after the fix: the pale blue-white solid quads near stairs and
railings become faint dark soft-edged shadows lying on the floor, and they stop showing through
floors (R2).

## The 07000002 bathhouse guard (drawn, never attacks, can't be damaged)

`crates/nf-view/tests/dump_render4.rs` runs the real viewer path you asked for and dumps every
drawn drone with present / started / enabled / in_llist / held / drawn / team / mode / dtypenew /
channel. The result pins the defect:

- The bathhouse guards (skins 0x05000036/3a/39, team 1, channels 21/23) and the two "captain"
  bodyguards d22/d23 (skin 0x05000034, **team 3**, modes 7 and 5, dtypenew 14, channels 22/23) sit
  at tick 0 with `started=false enabled=true`, and land in WaitSwitch with **`enabled=false`**.
- The port's WaitSwitch *enter* disables the drone; disabled drones are not posed (no animation)
  and not hit-tested — while the renderer still draws their placement meshes. That is your symptom
  exactly: a guard standing frozen next to the hostage who can't be shot.
- In the game, WaitSwitch stops vision and engagement; the drone stays at its post, posed, and
  takes damage. The fix is to stop `enabled=false` from cancelling the pose and the hit boxes for
  channel-waiting drones (and to check the team-3 hit filter separately).

Also measured: plates 341/383 are kind 0xEA → channel 21 [D]. My headless walk over them didn't
raise ch21 — Bond got jammed at the upper-floor plate positions in the harness — so the plate wake
is still unverified end-to-end; it needs an unobstructed walk route.

## Everything else in the task

- **G-041 (bullet decals)**: fully measured in GAPS, small-medium effort — not started, out of
  budget.
- **G-042 (skin-texel check)**: needs the civilian skin's texture identified on disc first — not
  started.
- **O5 (searchlight beam and corona)**: 07000007/08 have no searchlights; the 07000006 work needs
  on-screen checks — not started.
- **07000005 torso flip, 07000006 stationary guards, searchlight→alarm sound**: no new evidence
  this session; fix-drone3's torso clamp is still in; Searchlight_Update's klaxon/music call is
  still unidentified.

## Verification

- New evidence harnesses: `crates/nf-view/tests/probe_render4.rs` (render measurements, all
  NF_ISO opt-in) and `crates/nf-view/tests/dump_render4.rs` (the drone dump pass). Both run green
  on the slim ISO; they are probes, so they assert nothing that isn't already pinned elsewhere.
- Full suite: `NF_ISO=<iso> cargo test --offline --release -j2 -p nf-game -p nf-formats -p
  nf-audio -p nf-view -- --test-threads=1` — see the commit; no library code changed, so the suite
  exercises the pre-existing behaviour.
- Determinism: no RNG draws added, no Sim code touched.

## Files

`crates/nf-view/tests/probe_render4.rs` (new), `crates/nf-view/tests/dump_render4.rs` (new),
`docs/WORKLOG.md` (fix-render4 entry), this report. No shared library files touched.
