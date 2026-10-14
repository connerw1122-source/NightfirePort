# 103k2-fix-weapon-anims2 — the three merge-held suite failures, fixed

Branch `s/103k2-fix-weapon-anims2` (base: master 7785582 + `s/fix-weapon-anims` merged; master
ed718f8 merged mid-task). The merge of fix-weapon-anims into master held 3 tests that pass on
master; all three are green now, and every fix cites the game's code.

## The verdict on each failure

The branch's new timing is RIGHT — the game really does switch slower than the double-advance bug
made the old tree switch. The three test files' windows/settles were tuned to that 2x-fast bug and
are re-tuned here with the game's tick counts. On top of that, the work exposed **three real port
regressions/bugs** (each with game evidence), fixed in `crates/nf-view/src/scene.rs`'s view-script
block.

### 1. `gadget_select_live::the_camera_pickup_equips_through_the_gadget_cycle`

**Symptom**: "the weapon cycle must stay in 1..0x47" — the test read `gun.weapon == 0x52` (a
gadget id) right after the first weapon-cycle press.

**Diagnosis (port correct, test window wrong)**: the game writes the weapon byte `+0x62` at
**holster-done**, not at the draw's end — `Player_SetWeaponAnimObj` case 2: `if (+0x63 != +0x62)
{ +0x62 = +0x63 }` then queues the draw [M 0x1a2e38:262-263]; the draw then plays at 1.25
[case 3:287-288, `AnimScriptAddSpeed(0x3fa00000, …+0xC0)`]. The port does exactly this. The test's
gadget-equip loop broke out on `weapon == 0x52` alone — mid-draw (measured: the grapple→camera
switch is ~24-tick holster + ~86-tick draw; the loop's 60-tick settle was tuned to the 2x bug) —
and the weapon-cycle sweep's first read then counted the still-equipping camera as a "selected"
gadget. **Test fix**: the equip loop now waits for `weapon == 0x52 && switch_state == 0` (120-tick
per-press cap) and the weapon-cycle settle is 120 ticks (a full correct-rate switch: two
`ceil((len−1)/1.25)` runs + the stop-wait ticks). No assertion changed: the cycle-walk itself
(bounds 1..0x47 [M 0x1a7120]) was never the bug — `Gun::cycle` clamps into the weapon band and
`toggle_back` restores a remembered weapon id only.

### 2. `gadget_laser_live::the_beam_end_equals_the_crosshair_rays_hit`

**Symptom**: "the beam is parked" — `sim.laser_beam` None after the 30-tick aim+fire hold.

**Diagnosis (port correct, test window wrong)**: the Laser's draw script 0x060004ee is 76 frames
[measured off the level's scripts]; at the game's 1.25 it runs ~61 ticks, after the start weapon's
holster [M case 1:220-223] — and the fire gate blocks until the machine is idle (`can_fire =
!switching`, the game's state-3/4 fire block [M 0x1a2e38 case 4]). Measured: the draw settles at
hold-tick 48 and the first beam parks at hold-tick 49 (the other laser tests, with 60–300-tick
windows, passed all along). **Test fix**: the hold window 30 → 120 ticks. All three assertions
(beam start at the eye, the far end on the aimed face, the crosshair-ray direction) unchanged.

### 3. `fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds`

**Symptom on the merge**: five sweep cells never played their missing-on-05 ids — 0x32 [1057, 1059,
1479], 0x33 [1056, 1058, 1479], 0x41 [639], 0x42 [1051, 1481], 0x43 [1486, 1487, 1488] (the task
brief's three cells plus 0x42/0x43, which the brief's partial list had not yet captured).

**Diagnosis — one test-window issue and three real port bugs**:

- **Port bug A (the op-1 frame window is empty during a switch)** — `Sim::run` captured the
  `(prev_script, prev_time)` watermark *after* the switch machine's tail advance, so during a
  holster/draw the window came up `from == to` and **no op-1 fired while a switch played**. The
  game's `AnimProcessScriptCmds` runs per `AnimObjectUpdate` on the playing script — holster/draw
  included — over the same tick's (prev frame, new frame] [M 0x1c1b38]. This silenced every
  draw/holster/variant-script sound; the Samurai's draw whoosh 0x5c7 sits at frame 90 of its
  107-frame draw script 0x60007ef and never played on the merge (master only played it by accident:
  the double-advance bug kept the window alive). **Fix**: the watermark moved above the switch
  machine, so the window covers the whole tick's advance. This alone turned 0x32/0x33 green.
- **Port bug B (the idle fidget threshold 100x early)** — the +0xCC fidget queued at
  `aims_idle >= 20` from fix-weapon-anims's `[S]` guess "FRAME_RATE_INT 1". The game's threshold is
  `0x14 * FRAME_RATE_INT` [M 0x1a2e38:83]; **FRAME_RATE_INT = 60** [L docs/fields/bldata.md +0x93E
  "the weapon-raise cooldown (= n_INT·0x14 = 1200)" — the EE snapshot's own default is 1200 — and
  +0x95E "= (char)n (60)"; door.md's n·6 = 360-tick auto-close corroborates]. So the threshold is
  **1200 idle ticks** (20 s), not 20. With the 20-tick threshold the fidget kept choking the idle
  loop before its later frames. **Fix**: `aims_idle >= 1200`, cited in place. The fidget test in
  weapon_anims_live.rs (this branch's own test from the predecessor task) pinned the guessed 20 and
  is corrected to 1200 (`p2k_idle_fidget_plays_after_1200_idle_ticks`) — the assertion's meaning
  (the fidget queues once, plays at 1.0, the idle re-queues, the counter resets) is unchanged.
- **Port bug C (a stale fire state/latch survived a switch)** — the Gun's `fire_state`/`latch`
  stayed set when a switch displaced a still-playing fire script. In the game the switch REPLACES
  the view state: `Player_WeaponSelect` writes +0xF4 = 8/3/1 directly [M 0x1a2cc0:20/35/38] — the
  fire state 9 and its +0x95C latch are gone. In the port the stale `fire_state` then hit the
  state-9 script suppress (`was_fire_state && flags & 0x100400 && +0x44 != 0`) on the NEXT weapon's
  first pull and queued no fire script at all (probe at the 0x41 cell: `fs true, latch true,
  script = idle, clip 1` across the whole fire window). **Fix**: the switch-start block clears
  `gun.fire_state`/`gun.latch` (the fire script's loop sounds already stopped there).
- **Test window (the sweep's draw wait)** — `until_drawn(240)` was tuned to the 2x-fast draws and
  broke mid-switch/mid-reload: the twins' draw-done clip (0 — the equip fills the base's clip slot,
  the switch reads the id's, cf. the Samurai `base(0x32)=0x33`, `shares_base_clip`) starts the
  auto-reload on the first idle tick, and breaking inside its view states 5/6/7 [M cases 5/6/7]
  made the fire window's held trigger ride the reload branch — which consumes the press edge; the
  game's held trigger does not fire across a reload [M 0x1a5460:442-445's new-press edge test].
  **Test fix**: `until_drawn` now also requires `reload_state == 0` and waits up to 600 ticks
  (covers the chained previous-cell reload tail + holster + draw at the correct rates). The
  MISSING_ON_05 mapping, the owning-bank assertions and the `given >= 69` baseline are untouched.

## Files touched

- `crates/nf-view/src/scene.rs` — the view-script block: the op-1 watermark move (commented in
  place), the fidget threshold 1200, the switch-start fire-state/latch clear. No other Sim code.
- `crates/nf-view/tests/gadget_select_live.rs` — equip-wait + settles (assertions unchanged).
- `crates/nf-view/tests/gadget_laser_live.rs` — hold window 30→120 (assertions unchanged).
- `crates/nf-view/tests/fix_weapon_sounds_live.rs` — `until_drawn` reload-aware + 600 cap
  (assertions unchanged).
- `crates/nf-view/tests/weapon_anims_live.rs` — the fidget test's threshold 20→1200 (this branch's
  own predecessor test; the fidget-semantics assertions unchanged). Outside the brief's file list —
  justified: the [L] threshold fix in scene.rs is impossible to keep without correcting the test
  that pinned the wrong guessed constant.

**Shared files**: none touched by this task (the mid-task master merge brought in fix-weapon-fx-levels'
drone.rs/gas.rs/lib.rs/weapon_assets.rs changes; the WORKLOG merge conflict was resolved
append-only, both entries kept).

## Verification

- Reproduce-before-fix: all three tests shown failing on the pre-fix tree at the head of this
  session (gadget_select.rs:41, gadget_laser.rs:257, fix_weapon_sounds.rs:281), plus per-cause
  probes (the mid-draw equip break; the first beam at hold-tick 49; the 0x41 stale
  `fs/latch/clip 1` fire window; the master-baseline probe on a throwaway master worktree showing
  master's 1479 came from the idle-loop accident and 0x33 was equally refused there).
- Fail-first per fix: the sounds sweep failed with the five cells before, passes after (81 ids
  started vs 64 failing); the fidget test failed at the 1200 threshold before the scene.rs fix and
  passes after; gadget_select/gadget_laser failed before, pass after.
- Neighbourhood runs on the final tree (foreground chunks — the sandbox kills background jobs
  between tool calls, so `suite.py` could not run as a background job):
  - gadget_laser_live 7/7, gadget_select_live 2/2, fix_weapon_sounds_live 3/3 (+2 pre-existing
    ignores), weapon_anims_live 7/7.
  - fix_audio2_live 4/4, fix_auto_switch_live 2/2, fix_weapon_effects_live 6/6,
    fix_weapon_fire_modes_live 5/5, fix_weapon_modes_live 6/6, fix_weapons_all_live 4/4,
    gadget_grapple_live 1/1, gadgets_live 4/4.
  - fix_pickups_remainder_live 1/1, fix_pickups_trace_live 2/2, weapon_assets_live 5/5,
    weapon_fx_levels_live 2/2 (+2 ignores).
  - gadget_grapple_user_spot 2/2, qclaw_live 3/3, nf-view lib 8/8.
  - nf-game lib 150/150.
  - **Total this session: 224 passed, 0 failed, 6 ignored (pre-documented ones).**
- The full four-crate sweep (`tools/test/suite.py` / the cargo four-crate command) did NOT fit the
  window again: the background suite died within one tool call (this sandbox kills background
  jobs — the same failure the fix-hud-radar session logged), and the foreground window closes at
  18:44 UTC. The next session's first item is the full sweep on this tree. Risk assessment for the
  unrun families: the port changes are confined to the view-script block's switch/fidget/op-1
  paths + the four test files; every weapon/gadget/audio/pickup binary that exercises them ran
  green above.

## Live checks for the main agent

1. **The fidget threshold** (the [S]→[L] correction's last mile): in PCSX2 with a weapon idle
   (e.g. the P2K on 07000005 at the start), gdbwatch `BLData+0x93A` (the idle counter) and
   `+0x959` (the fidget substate) for ~25 s of standing still: expected — +0x93A counts up to
   1200, then +0x959 = 1 and the +0xCC script queues (the view model's fidget), then +0x93A
   restarts from 0. If the game fidgets after ~20 ticks instead, bldata.md's +0x93E note needs a
   second look (but its snapshot default 1200 is strong evidence).
2. **The draw-script op-1s** (port bug A): gdbwatch `Sound_Play3D`-adjacent or the SFX pool while
   switching weapons with the Samurai (0x32/0x33) — expected: the draw whoosh 0x5C7 (1479) plays
   mid-draw (~frame 90), and no 0x5C7 after the draw completed. nf-view: `--bond` at the 07000005
   start with `--give 0x32`, switch away and back — the whoosh should be audible mid-draw.
3. **The AIMS-20 side lights' thresholds** (pre-existing on master, NOT changed here — flagged for
   a follow-up): scene.rs's frame_state blinks bone 8 at `aims_idle > 15` and kills the lights at
   `> 45`, but the game reads `FRAME_RATE * 15.0 < +0x93A` and `FRAME_RATE * 45.0 < +0x93A`
   [M 0x1a5460:264/314] — with FRAME_RATE = 60.0 that is 900/2700 ticks (15 s/45 s). Expected live:
   the AIMS lights stay solid ~15 s of idle, blink ~15–45 s, go off after ~45 s. The port today
   blinks at 0.25 s and goes off at 0.75 s. A visual change — left to the main agent's render per
   the merge lessons.
4. nf-view render spots (the fidget/raise are the only view-visible changes on this branch):
   `--bond 54,2.4,-173.5` on 0700000c after equipping the camera — the draw ends in the idle pose;
   the fidget only after 20 s idle. The aim raise/lower tests still pin the +0xB8/+0xBC numbers.

## What's left

- The full four-crate suite re-run on this tree (the next session's first item; the neighbourhood
  chunks above are the risk cover).
- The AIMS lights' 900/2700-tick thresholds (master pre-existing; needs the main agent's render —
  live check 3).
- The clip-slot twin oddity (equip fills the base's slot, the switch reads the id's — the Samurai
  0x32/0x33 and the 0x41/0x42 pair both show it) is worth a dedicated look: the game's
  `Player_EquipWeapon` 0x1a8a58 vs the switch's `Player_SetWeaponAnimObj` case-2/4 reads — the port
  may mis-slot the twins' clips (this task worked around it in tests only).
- The PP7-family +0xD0/+0xD4/+0xD8 path and the 0x32 Samurai reachability (inherited from
  fix-weapon-anims's left tail; unchanged here).
