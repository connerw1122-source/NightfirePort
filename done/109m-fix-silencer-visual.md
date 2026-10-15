## Your task: `silencer-visual`, the silencer works but isn't drawn on the gun

User (05): "The suppressor still stays on the gun, but visually it's not there." The silenced twin 0x03 has no muzzle
flash and its own sound (works), but the silencer MESH isn't on the view model. Find the game's attachment for the
silenced record (a different fp model, or an attached sub-mesh) and draw it. Render the P2K and the silenced P2K.
This comes from the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`, mouse +
keyboard). The user was very disappointed: several merged fixes passed their tests yet were wrong or broken in play.
Your test MUST drive the real input path (`Sim::step_bond` with the `Input` the window builds - lib.rs maps the mouse
and keys), not internal setters, and must fail first on master. The main agent will compare against PCSX2 after you
finish; put the exact steps to reproduce in the report.

Files: `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/silencer-visual`.

**PCSX2 ground truth (main agent, 2026-10-14):** read `docs/live/2026-10-14-weapon-measurements-pcsx2.md` on master first.

---
claimed-by: sandbox5 at 2026-10-15T04:31Z

---
## Report (done, 2026-10-15T06:12Z)

# fix-silencer-visual (109m) — the silencer mesh draws on the silenced twins

**Status: done.** The silenced P2K's silencer — the tube the user doesn't see ("The suppressor
still stays on the gun, but visually it's not there", playtest 1014-1725) — is the game's
**view-entity mesh** (`weapon_data+0x7C`), attached to **datum 0** of the weapon AnimObj every
tick while the held record's flags carry **0x800**. The port built the silenced twin's view-model
character but never attached that mesh. Fixed in `Scene::build`'s first-person section.

## The decode (evidence)

- [M] `Player_WeaponFiring` **0x1a5460:96-110** (ACTION.ELF): `if weapon_data[w]+0x68 & 0x800`,
  then (outside view-anim state 8) `AnimDatumSetEntity(BL+0x7E8, 0, weapon_data[w]+0x7C, 0xff)`
  — datum 0 of the player's weapon AnimObj gets the record's +0x7C mesh; state 8 (the swap/
  holster anim) clears it instead. **0x1a5460:111-118** (the twin arm): while the held record's
  *twin* (+0x05) has the 0x800 flag, datum 0 is cleared — the bare record never shows the tube.
- [M] `AnimObjectDraw` 0x1c4490 draws a datum's entity at `QuatTransToMat(datum.q, datum.t) ·
  bone world` (docs/formats/weapon-data.md "Drawing") — the port's established attachment
  formula (the muzzle flash rides the same path).
- [L] The golden weapon table (SHA-1 `2fd48d6d…`, the ACTION.ELF static initialiser 0x1b3488):
  the silenced twins **0x03/0x05/0x07/0x09** carry flags **0x1884/0x188c** — bit 0x800 — over the
  bare **0x02/0x04/0x06/0x08**'s 0x1004/0x100c; each pair shares +0x7C and the fp skin (P2K:
  mesh **0x02000842**, skin **0x05000088**). Record 0x14 (flags 0x880) carries the same pattern.
  The P2K twins' fp-flash (+0x48) is 0x020001f6 on the bare record and **0** on the silenced one
  — the already-correct "no muzzle flash".
- [D] The disc census: **07000005 carries 0x02000842 natively** (with 0x020009a3/0x020001f2/
  0x0200099d/0x020005b2 on most story archives).
- This fills session 102b's open decode ("the game's persistent mount is a draw-side gate this
  session did not decode", `fix_silencer_modes_live.rs`): the per-tick 0x1a5460 attach IS the
  persistent mount; the 0x06000828/0x06000829 swap anims animate part-bone 43 (whose mounted
  translation ≈ datum 0's (−0.001, 0.091, 0.141)) — the slide-in during the swap.

## The fix

- `crates/nf-view/src/scene.rs` (the task file): in the view-model build loop, after the AIMS-20
  lights block — for each weapon id whose record has `flags & 0x800 && +0x7C != 0`, attach the
  +0x7C mesh (from `level.by_hash`) to the record's **own view-model character** on datum 0's
  bone with the datum's q/t as the local. A plain rigid attachment: it draws only while that
  record is the held weapon (the character only draws when active — the game's per-tick gate),
  and the bare record's character gains nothing (the twin-clear arm).
- `crates/nf-view/src/lib.rs` (shared, small hook): `wanted_weapon_models` walks `+0x7C` too, so
  the `--give`/dev-menu merge pulls the tube's mesh for a foreign-level give.

## Verification (failing first)

`crates/nf-view/tests/fix_silencer_visual_live.rs` — the REAL input path throughout (`Sim::step_bond`
with the window's `Input`; the mode press is `Input { mode_switch: true }`, fire_modes3's harness):

- `the_silenced_twin_draws_the_silencer_mesh_on_the_view_model` — **on the pre-fix master it
  FAILED**: "the silenced twin's view model carries exactly ONE attachment over the bare record's
  set" — got 0. Post-fix: exactly 1, on datum 0's bone (38), local = the datum's q/t, and its
  palette matrix keeps its rotation (draws with the held silenced twin). Sisters inside: the bare
  record's set gains nothing through the press.
- `the_bare_p2k_stays_bare_through_a_round_trip` — two real mode presses (0x02→0x03→0x02) leave
  the bare record's rigid set byte-identical (the twin-clear arm). Green before and after.

Regression sweep (all green post-fix): fire_modes3_live 5/5, fix_weapon_modes_live 6/6,
fix_weapon_fire_modes_live 5/5, fix_weapon_sounds_live 3/3, fix_silencer_modes_live 2/2,
fix_aims20_modes_lights_live 3/3, models_weapons_live 4/4, weapon_anims_live,
weapon_assets_live, weapon_fx_levels_live, weapon_matrix_live (its non-sweep tests) — all ok.

## Suite (the final tree, merged with origin/master f818f7c5's parents)

The sandbox reaps background processes (setsid included), so the fast runner cannot survive a
tool-call boundary here — the suite ran in foreground chunks (`--no-fail-fast`,
`--test-threads=1`), per-crate totals:

- **nf-game: 276 passed / 0 failed / 2 ignored** (26 binaries)
- **nf-formats + nf-audio: 99 passed / 3 failed / 0 ignored** — the 3 are the allowed
  `disc_golden` trio (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
  `directory_tree_matches_the_golden_manifest`; the slim ISO has no retail image)
- **nf-view: 929 passed / 3 failed / 37 ignored** across 257 of 262 test binaries — the 3
  failures are the known environmental GPU-adapter class (`fix_0c_pylon_live` ×2,
  `probe_render_pylon_only`: `request_adapter NotFound`, failing identically without this
  task's diffs — the previous session documented the same trio)
- Unrun for the wall (timeout-cut, none touch the view-model path): `wire_live`, `wire_repro`,
  `world_fx_live`, `worldobj_live`, `zz_obj_drives_probe` (+ `weapon_matrix_live`'s
  `#[ignore]`d 27-level matrix sweeps). **Total: 1304 passed / 6 failed / 39 ignored.**
- Final-tree re-check (after the last origin/master merge, `b084d6ff`): fix_silencer_visual_live
  2/2, fix_silencer_modes_live 2/2, fire_modes3_live 5/5 — green.

## Reproduce

Headless: `NF_ISO=<iso> cargo test --offline --release -p nf-view --test fix_silencer_visual_live`.
In play (`nf-view <iso> 07000005 --dev`): spawn on 05 with the P2K, press the mode key (the
window's B / the mapped mode key) — the HUD reads "Silenced" and the gun now shows the dark-grey
tube screwed onto the barrel end, in line with the barrel (the ground truth's description); press
again — back to "Semi", the tube gone, the bare gun unchanged.

## Live checks for the main agent

PCSX2 (07000005, the spawn; the mode button is ○ in the 1014 capture layout):

1. Spot: first-person, P2K idle, "Semi". Action: press ○ once, wait out the swap anim. Expected:
   the long cylindrical silencer screwed onto the barrel end (~a third of the slide's length, in
   line with the barrel) — docs/live/2026-10-14-weapon-measurements-pcsx2.md "The P2K's silencer".
2. Spot: gdbwatch `0x2bf150 + 3·0x10C + 0x7C` (record 3's +0x7C) — expected `0x02000842` read by
   `AnimDatumSetEntity`'s caller 0x1a5460 every tick while "Silenced" is held, and no attach while
   "Semi" (the record-2 arm reads the twin's flags at 0x1a5460:111-118).
3. Spot: the silenced twin's +0x48 — expected 0 (no first-person flash mesh) while the bare
   record's is 0x020001f6 (already ported).
4. Spot: press ○ again. Expected: the tube disappears with the swap anim back to "Semi".

nf-view `--bond` spots (first-person view model; the dev-menu give of 0x03 is the silenced spot):

1. The 05 spawn with the bare P2K — the gun exactly as before (no tube; the control).
2. Give 0x03 via the dev panel (or press the mode key in the window) — the tube on the barrel
   end, aligned with it; the hands/pose unchanged.
3. Give 0x03 then cycle to any other weapon — no tube anywhere in the world (the attachment
   rides the 0x03 view-model character only).

## Shared files touched (the hooks)

- `crates/nf-view/src/lib.rs`: `wanted_weapon_models` — one offset added to the merge walk
  (`0x7c`), cited inline. Additive only.
- No `drone.rs`/`player.rs` changes (the merge from origin/master brought other tasks' commits;
  none of this task's code is in them).

## What's left

- The 5 unrun nf-view binaries above (wall-time cut; they are world-object/probe files, not the
  view-model path) and the `disc_golden` trio needs the retail image for a full-suite zero.
- The swap-anim cosmetic: during the 0x06000828 slide-in the game also animates part-bone 43;
  the port plays the swap anim (session 102b's work) but does not animate the part-bone channel
  — the persistent tube (this fix) is what stays after the anim either way.
- PCSX2 live checks 1–4 above.
