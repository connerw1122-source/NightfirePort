# fix-searchlight-remainder — the searchlight G-033/G-036 remainder

Branch `s/fix-searchlight-remainder` (cut from master 7a614ba). Session 21:56–23:xx UTC 2026-10-13,
sandbox deadline honoured. Files: `crates/nf-game/src/{searchlight,light,gas,drone}.rs`,
`crates/nf-view/src/{scene,gpu,dev}.rs`, `crates/nf-view/tests/fix_searchlight_remainder_live.rs`.

## Done

1. **Late spot creation: settled — no mechanism exists** (G-036's open item, closed on disc
   evidence). `Searchlight_Create` 0x1cb4f8 has exactly one caller — the dispatcher case 0xDE
   [M 0x1d04f0:341-344], which the block walk `parsemap_block_map_data_dynamic` 0x1d1b68 reaches
   only for placements with `flags & 0xa000 == 0`; **every 0xDE placement on 07000005/06 carries
   flags-hi 0** (probe over the disc data), so all are created at load and nothing creates more
   later. The placement census IS the snapshot histogram:
   - **07000005**: 7 heads / 7 mounts / **2 spots** / 6 beams / 7 coronas — five heads are spot-less
     by level design and can never arm, in the game either (the "only 2 spots at the snapshot" is
     the disc's own data, not a missing spawner);
   - **07000006**: 4/4/4/4/4, and every spot placement (indices 489-492) comes **after** its head
     (2-8) in creation order, so all four link — no orphans on the playtest level.
   The other block-walk caller is `ResetMap_Load` 0x1d6360 (the restart path, param_7 = 1), not a
   play-time creator; case 0xDE is ungated on param_7 either way.
2. **The spot's dynamic light: a real illumination mismatch found and fixed.** The game's
   `psiLight_SetLights` 0x10e588:34-38 uploads the node's **static** +0x10 — the placement/housing
   position, written once at create *before* the −10000 move [M 0x1cb4f8:154-167] and never changed —
   while only the intensity `(1 − d·0.25)·50` [M store 0x1cb400] and the 4.0-accept test measure the
   tracked spot's **live** position [M `Lights_CalcClosestLights` 0x1cb2e0:41-44]. The port resolved
   the uploaded position to the live spot: it drew a halo at the ground circle instead of the beam
   lighting Bond from the housing — very plausibly the "spotlights look weird" illumination delta.
   Fix: `Light` keeps the housing (`Part::spawn_pos`, the pre-shift translation),
   `ResolvedLight` carries `pos` (upload = static) and `track_pos` (live spot) separately
   [M-cited in light.rs], `closest`/`pick2` measure kind-2 distance from `track_pos`.
   **Flicker: none exists** — `psiLight_Create` 0x10e788 is a stub, there is no `psiLight_Update`,
   and `Light_Update`'s flicker machine is gated on period +0x24 ≠ 0 while the searchlight record
   writes 0 [M 0x1cafb8:24; 0x1cb4f8:146] — the `Rand_Rand(0)` seed draw (already consumed) is the
   only flicker trace. The port's model was already right; the question is closed with proof.
3. **The broken-head smoke (`Gas_Init`) now consumes the game's 6 shared-RNG draws with the game's
   formulas, and its real life.** The decompile drops `Gas_Init`'s 11th argument at every call site;
   the disassembly passes it — the searchlight puff's life is **128 ticks** (0x80 on the stack,
   [M 0x1cbce0/0x1cbcb0]), not the port's guessed 24. The six draws in order [M disassembly]:
   the caller's `Rand_FRand(π)` rot-z (0x1cbca0), the spin `Rand_FRand(π/2·REC_FRAME_RATE)`
   (0x1ab060), the sign `Rand_Rand(0x100)` negated at ≥ 0x80 (0x1ab08c), and the type-7/NULL-vel
   wind spread `Rand_FRand(0.2)` ×3 with `vel = (wind + (r−0.1), wind + r, wind + (r−0.1))·REC_FRAME_RATE`
   (0x1ab160/0x1ab184/0x1ab1a4; wind = `Env_GetWindVel` 0x17c048 = 0 with no cel wind radiator —
   the cel wind system stays unported, [S] noted). The gate is also game-exact now:
   `obj+0xFC != 0` (the head actually drawn) and `(GameState+52 + obj+0xEC) % 7 == 0` where
   **obj+0xEC = GameState+52 at create** [M `control_init_object` 0x134630:15] — the port's
   `(frame + created) % 7` with created = 0 (the port creates at frame 0) replaces the old
   `(frame + part_index)` stand-in. Implemented as `GasPool::init_searchlight_smoke` (gas.rs),
   called from the kind-3 control block in drone.rs; the draws are consumed even when the port's
   pool cap rejects the push (the game's RNG has no notion of the cap).
4. **The corona alpha byte reaches the draw** (the "white blobs" remainder). The searchlight part
   vertices previously shared one 1.0 colour slot; `SkinLayout` gained a per-part colour region
   (`col_sl`) and `skin_palette` writes the corona's `obj+0x106 / 255` into its slot's `.a`
   [M 0x1cb900:334-336: alpha = `(dot−0.2)·4·255` low byte]; the skin shader multiplies
   `c0.a * cols[sc].a` — every other slot keeps 1.0, so characters/pickups/attachments are
   untouched. The GS reads the byte as alpha/0x80 and the shader's existing ×255/128 rescale makes
   byte 0x2e → 0.36 exactly [L coronas 0x2e/0x7d/0x1f on the live parts]. Scale was already wired.
5. **The 0x4E level-light flicker machine made faithful** (same machinery the spot question rode):
   `Light_Update` computes `active = (mask == GameState+52 & mask)` [M 0x1cafb8] — mask 0 compares
   0 == 0, so a mask-less flickering light is ALWAYS active in phase; the port's extra
   `flicker_mask != 0 &&` guard made it permanently dark. Removed. And the case-0x4E call site's
   disassembly [M 0x1d0bd4-0x1d0c04] pins the record tail the decompile hides: +0x40 = the flicker
   period, **+0x44 = the counter-seed source** (Light_Create stores `seed>>1` at +0x26 and
   `seed + Rand_Rand(seed)` at +0x28), **+0x48 = the mask**, +0x4c = the wrap sound (n32 8-byte
   stack slots). The port parsed only the period and derived the seed from it; now
   `LightList::create_full` (additive) takes the real seed and mask. Dormant on 05/06 (probed: no
   0x4E placements there), fixed for every level that has them.
6. **`searchlight-tracks-bond` re-added** (the G-036 follow-up): the dev scenario is back in the
   registry (now pointing at 07000006, whose four heads all carry linked spots), with the removal
   rationale rewritten — G-036 was closed as a misdiagnosis (docs/atlas/searchlight.md): 07000005's
   1/7 linked heads are the level's own data and the mid-6.11 head genuinely sweeps ≈10° above
   horizontal. An NF_ISO test now covers the same ground headlessly (below).

## How it was verified

- `crates/nf-view/tests/fix_searchlight_remainder_live.rs` (NF_ISO, `nf_view::load` +
  `Sim::step_bond`), all green:
  - `the_spot_light_shines_from_the_housing` — the resolved kind-2 record uploads the housing
    (`l.pos` = the placement translation) while `track_pos` = the live spot; `pick2` for an object
    at the spot lights it **from the housing** (or the game's own 30-accept cap crowds the spot
    records out — asserted explicitly, the list order and cap are the game's [M 0x1cb2e0]).
  - `the_broken_head_smoke_uses_the_game_stream_and_life` — a poked state-2 drawn head smokes on
    the `(frame + created) % 7` phase, the puff's life is 128, and the puff's spin/velocity equal
    the game's 6 draws replayed at a fixed stream offset of the pre-tick RNG snapshot (pre-fix the
    fields came from a counter hash and life was 24 — no offset reproduces them).
  - `a_courtyard_head_tracks_bond_until_the_lost_limit` — riding a landed spot arms a head (state
    1); Bond walks with the spot for 120 ticks and the head holds state 1 (the re-added
    tracking coverage); the tracking corona draws with alpha < 255 and its colour slot carries
    `alpha/255` while non-corona parts stay 1.0; Bond leaving flips the head back to the sweep
    after the game's `FRAME_RATE·15` = 900 ticks (measured within [890, 940]).
- Unit tests: `gas.rs::searchlight_smoke_consumes_six_shared_draws_in_order` (the 6 draws bit-exact
  against a twin stream + a seventh-draw stream-continuation check), `light.rs::
  the_kind2_light_uploads_the_housing_and_measures_the_live_spot` (with the pre-fix reading pinned
  as the sister), `a_maskless_flicker_light_stays_active_in_phase` (mask-0 always active; a real
  mask gates only the count-up phase), `the_4e_parse_consumes_the_seed_draw_from_the_seed_field`.
- Fail-first: the live test file cannot compile against pre-fix master (it pins new additive
  fields — `ResolvedLight::track_pos/track`, `SkinLayout::col_sl`, `Part::spawn_pos/created`);
  the pre-fix values the assertions replace were directly observed during development: the resolve
  upload rode the live spot (the first run's mismatch printed the spot position where the housing
  belongs), the puff life was 24, no shared draws were consumed, and every part vertex shared
  `col_one`. The light.rs unit test carries the old reading as an explicit sister assertion.
- Full suite (`--no-fail-fast`, slim ISO): see the count below (the 3 `disc_golden` tests are the
  only allowed failures).

## Shared files touched (small, additive, listed)

- `crates/nf-game/src/drone.rs` — the kind-2 light record's `pos` now `part.spawn_pos` (comment +
  one token), and the state-2 smoke block (the spawn list + `init_searchlight_smoke` call replace
  the old `gas.init` loop; the kill-mirror block below it unchanged).
- `crates/nf-view/src/scene.rs` — `SkinLayout.col_sl` field, the searchlight attrs' colour index,
  and the palette's per-part alpha write (three additive blocks).
- `crates/nf-view/src/gpu.rs` — the one shader line `c0.a * cols[sc].a` (+ comment).
- `crates/nf-view/src/dev.rs` — the scenario registry entry, the SCENARIOS list, the un-`#[allow]`
  of the kept fn, and the corrected 07000006 pointer/comments.
- `crates/nf-game/src/gas.rs` / `light.rs` / `searchlight.rs` are this task's named files; all
  changes there are additive (new fn/fields/tests) except the flicker guard's one-line correction
  and the placement_light parse swap to `create_full`.

## What's left / live checks for the main agent

1. **The spot-light visual** (no GPU here): on the fixed build render
   `--bond 4.8,42.1,-16.0,-2.2,0` (07000006 courtyard entrance) and `--bond 7.1,48.6,-23.6,-1.4,0.2`
   — when a spot lands on Bond or the wall near him, the surrounding geometry should brighten
   along the housing→object direction (the beam-illumination effect), not glow at the ground
   circle. PCSX2 cross-check: the same stance in the game.
2. **The corona alpha**: same spots — coronas facing away should fade out; a tracking head's
   corona should be bright but not the old full-white blob. If the GS alpha convention differs
   (the port multiplies the mesh alpha by byte/0x80 via the 255/128 rescale), a live
   `fieldprobe` poke of a corona part's `obj+0x106` (e.g. 0x40) with a screenshot settles it.
3. **The smoke visual**: `fieldprobe poke <head>+0xf4 2 2` on 07000005's head 016e95a0 (the
   searchlight.md live-check recipe) — the puff should linger ≈2.1 s (128 ticks), drifting
   gently upward (vel ≤ ±0.0033/tick), spinning slowly. If the game's puff visibly jitters
   sideways, the cel wind radiator term is live there (the port models wind 0 — [S] to revisit).
4. **The 30-accept cap**: the probe showed the courtyard's 52 lights crowd the spot records out
   of `pick2` for objects near a spot (the cap fills with kind-0 radiators in list order). Verify
   in PCSX2 that the game behaves the same at the same stance (if the game's spot light visibly
   illuminates Bond there, the cap's semantics — accept-then-sort vs collect-all-then-sort — need
   a second look at 0x1cb2e0:64).
5. The 0x4E record's **sound byte +0x4c** is parsed nowhere's behaviour (the port stores 0; the
   flicker-wrap `Sound_Play3D` 0x42c80000/-1/-1 stays unported) — only relevant if a level ships
   a nonzero sound byte; grep the 27-level census if it ever matters.
6. G-033's last open item is now only the **bullet child-list (U-041)** stand-in and the last-ulp
   VU0 rounding of `Mat_Align2Dir` (visual-only, ±4 ulp asserted) — both carried in docs/GAPS.md.

## Doc updates

- `docs/WORKLOG.md` — this task's entry (appended).
- `docs/GAPS.md` G-033/G-036 rows: the spots/smoke/corona/flicker items this task closed are
  listed here; GAPS.md itself is a shared file — the merge can fold this report's counts in.

## Suite count

nf-game/nf-formats/nf-audio/nf-view full run on the slim ISO (`--no-fail-fast --test-threads=1`):
**nf-game 32/32 binaries green; nf-view fully green (the 3 new live tests included); nf-formats
only the 3 allowed `disc_golden` tests failed; nf-audio green.** Zero failures outside the
permitted trio.

## ntfy failed

All three fallbacks hit ntfy's daily message quota (the same 429 the fix-pickups-remainder task
recorded earlier today):
1. curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase
   your limits with a paid plan, see https://ntfy.sh/docs/publish/#limitations"}`
2. python urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. wget: exit code 8 (server error), no body.
