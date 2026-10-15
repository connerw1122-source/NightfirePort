## Your task: `laser-watch-look`, the laser watch: match the real game's beam and wrist

The user, repeatedly: "the laser isnt visible", "The laser watch sucks", "Still not right... look at it in the
emulator". The main agent measured the real game in PCSX2 — read `docs/live/2026-10-14-weapon-measurements-pcsx2.md`
("The laser watch 0x4e") on master first: on the press the wrist turns so the watch face points along the aim and a red
beam runs from the watch face to the crosshair point (wide pink-white glow at the watch, tapering to a thin red line
ending at the aimed-at surface); the meter drains ~31/s while held. Compare the port at 07000005's spawn
(`nf-view <iso> 07000005 --dev`, select the Laser with the gadget cycle, hold fire): the wrist anim (the fire script
+0xB0/+0xB4 of 0x4e and its view model), the beam's start point (the watch face bone, not the eye), its width taper,
colour and glow, its end at the crosshair hit, and the drain rate. Fix every difference; render the port at the
same spot before/after next to the description. Tests (NF_ISO) for the beam endpoints and the drain per tick through
the real input path.

Files: `crates/nf-view/src/scene.rs` (the beam + view model), `crates/nf-game/src/gadget.rs`.

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

**Branch:** `s/laser-watch-look`.

---
claimed-by: sandbox1 at 2026-10-15T05:27Z

---
## Report (done, 2026-10-15T07:19Z)


---
## Report from the worker (done, 2026-10-15T07:19Z)

# fix-laser-watch-look — the laser watch 0x4e: match the real game's beam and wrist

Branch `s/fix-laser-watch-look`, merged with origin/master @ 2e5a162b. Task 109q.
Ground truth: `docs/live/2026-10-14-weapon-measurements-pcsx2.md` §"The laser watch 0x4e" [D].

## The user's symptoms → the root causes found

1. **"the laser isnt visible"** — the renderer started the beam at the view model Character's STATIC
   creation matrix, which is the IDENTITY: the beam drew from the **world origin** to the crosshair
   hit (the drawn stretch was 115.85 units for a 39.60-unit watch→hit ray — caught by the failing
   test). The game starts it at the view model's **posed bone 0** — `AnimGetBoneWorldTrans
   (BLData+0x7e8, 0, 0)` [M Draw_LaserBeam 0x1a4560:20] — the watch face.
2. **"The laser watch sucks" / "Still not right"** — the beam's angles pointed the object's +z from
   the watch to the hit, but the sprite's geometry extends along **−z** [D the "lazer" mesh
   0x2000131 on 07000005: z ∈ [−1.0198, 0.00016]; the "watch_lazer" 0x2000133: z ∈ [−1.0, 0]] — the
   beam stretched **backwards through the camera**. The game stores `cartesian_to_spherical(start −
   end)` with the **elevation negated** [M 0x1a4560:44-62; the identical tail in `PositionBeam`
   0x1a9400], aiming +z from the hit back to the watch, so the wide origin end sits at the watch and
   the beam tapers to the hit. The tripbomb's beam had the same flipped convention and is fixed with
   the same change (one line, same arm).
3. **The drain** — while held the game's ammo pane is a **separate 0–100 charge**, not the clip
   percentage: `HUD_UpdateAmmoPane` 0x19a7f0:160-176 — in view state 0xb (the laser watch's held
   state [M 0x1a5460's `+0x6C & 0x400` arm]) the pane prints the beam bullet's instance fields
   (+0x110 u16 count / +0x104 f32 strip) and RETURNS before the kind-0x1a percent path. The clip
   itself spends 1/tick [M Player_RoundToFire 0x1a18e0, +0x91 = 1] — a percent of the 500 clip
   would drain 12%/s [L weapon_data 0x4e +0x92 = 500], but the measured held pane drains ~31/s ≈
   0.5/frame [D]. Master showed the clip percent (12%/s).

## What changed

- `crates/nf-view/src/scene.rs` (task file)
  - The beam's start = the view model's **posed bone-0 world matrix ∘ the gun matrix** — the same
    composition the attachment draw uses — instead of the static identity [M 0x1a4560:20].
  - The beam's angles = the spherical of (start − end) with the elevation negated, for the gadget
    beam [M 0x1a4560:44-62] and the tripbomb beam [M 0x1a9400 tail].
  - `Sim::laser_charge` (0–100, own field): a tick where the beam ran drains 31/60 [D], an idle
    tick refills 0.25 (the game's gadget +1-per-4-frames regen shape [M Player_Update 0x193600
    disasm 0x193f18-58] — [S] for the charge itself).
- `crates/nf-game/src/gadget.rs` (task file): `laser_charge_tick` + the drain/refill constants,
  with the citations above.
- `crates/nf-view/src/hud.rs` (**beyond the task's named list** — the held pane lives here; minimal
  additive block): while the fire machine is up on a watch laser with the beam computed this frame
  (the game's anim+0xF4 == 0xb proxy), the ammo count shows the charge and the strip shows it of
  100; otherwise the pane is unchanged [M the state-0xb early return 0x19a7f0:160-176].

Shared files touched: none beyond scene.rs (a task file). No renames, no repurposed constants.

## Verification (all through the real input path)

`crates/nf-view/tests/fix_laser_watch_live.rs` — 07000005 via `nf_view::load`, the Laser equipped
through the real switch input, held fire through `Sim::step_bond`:

- **Failing first on master** (shown in the transcript): the beam start summed to ~0 (the origin),
  the drawn z-length 115.85 vs the 39.60 watch→hit distance (backwards), 0.0 drained in 60 held
  ticks.
- `the_beam_starts_at_the_watch_face_bone`: the drawn beam start is not the origin, sits off the
  eye by the view model's wrist offset (> 0.1, < 3.0), i.e. at the posed watch bone.
- `the_beam_points_its_negative_z_at_the_hit`: the matrix's +z row dots positive with (start −
  end) [M the negated-elevation spherical], its length equals the watch→hit distance [M obj+0xE8],
  and the translation sits at the watch [M obj+0x30].
- `the_held_meter_drains_31_per_second`: 100 at hold start [D], 31 ± 2 drained over 60 held ticks
  [D], the clip keeps its own 1/tick spend [M RoundToFire], the release stops the beam and rings
  out through the release script 0x60004e8 to the idle 0x60004ea [L weapon_data 0x4e +0xb4;
  M 0x1a2e38 case 0xb], and the meter refills while idle [S].
- Unit sister: `gadget::tests::laser_charge_tick_drains_and_refills_the_measured_rates`.
- Neighbours after the merge, all green: nf-game lib 161/161; gadget_laser_live 7, gadgets_live 4,
  fix_satchel_tripbomb_live 5, qclaw_live 3, fix_stunner_press_live 3, ammo_pane_live 3,
  fix_sg5_casings_live 2.

## Remaining / live checks for the main agent

1. **The charge's writer + refill (the one open mechanism).** The pane reads the beam bullet's
   instance +0x110/+0x104 via the bullet walk [M 0x19a7f0:160-176]; the decompile grep found no
   writer of those fields, so the port stands the value in as `Sim::laser_charge`. In PCSX2
   (07000005 spawn, gadget cycle to Laser): hold fire 3+ s — (a) does the held count drain ~0.5 per
   frame to 0, and does the BEAM stop at 0? (b) release at some value X, press again — does the
   count resume at X or restart at 100? (c) leave it idle 10 s — does the count climb? The port
   currently: drains to 0 (the beam keeps firing — the CLIP gates it [M]), resumes at X, refills
   +0.25/tick.
2. **The beam's alpha/glow render.** Draw_LaserBeam sets the beam alpha 0xff once [M 0x1a4560:25];
   the per-frame flicker arm belongs to `Player_LaserPointer`, which is gated OFF for 0x4e
   (`+0x68 & 8` = 0 [L record]) — so the watch beam should NOT flicker. The port draws the sprite
   opaque; if PCSX2 shows a soft additive glow the material needs the render pass's word.
3. **nf-view render spots** (the before/after the task asks for):
   - `--bond 104.95,24.08,108.83,-2.148,0` (07000005's spawn, level aim at the gatehouse road) +
     hold fire: a red beam from the lower-left watch face to the aimed-at surface — wide pink-white
     glow at the watch, tapering to a thin red line at the hit; the wrist turned face-forward from
     the fire script 0x60004e9 (16 frames, seq 0x4000045f [L]); the HUD number drains ~0.5/frame.
     On master the same spot shows NO beam (it draws from the map origin).
   - The same spot, no fire: the watch idle on the left wrist, the meter "100 Laser".
4. **The 0c pylon tests** (`fix_0c_pylon_live` 2 + `probe_pylon_render`) fail on master BEFORE my
   change (verified by stashing) — pre-existing, not this task's.

## Suite

Full `tools/test/suite.py` on the final merged tree (origin/master 2e5a162b merged; the only later
commit is the collapsed-matrix guard, behaviour-identical for the reachable states and covered by
the re-run tests): **1227 passed, 6 failed, 66 ignored (40 min)**. The 6 = the 3 `disc_golden`
tests (allowed on the slim ISO) + the 2 `fix_0c_pylon_live` + `probe_pylon_render` — verified
pre-existing on master before this change (stashed re-run). The pre-fix baseline run: 1223 passed,
6 failed, same 6.
