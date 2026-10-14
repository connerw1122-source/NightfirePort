# fix-aim-raise — report

Branch `s/fix-aim-raise` (task 104f). All facts tagged [M]/[L]/[D]/[S].

## The deviation

Two merged fixes left the same hole (fix-guard-misc2 "What's left"; fix-weapon-anims deviation 2):
the game's view state 0xF sets the **aiming bit** only when the aim-raise script finishes, and
`Player_AutoAim` keeps scanning until then — while the port turned the aim camera (the zoom), the
free-aim window and the auto-aim lock on the instant the button went down.

## The game mechanism (the evidence)

- **The press defers the bit** — `Player_Weapon` 0x1a24c8 [M]: the hold-aim branch (45-52) sets the
  aim bit (`BLData+0x60 & 1`) while the button is held; the state machine then sees the bit rose
  (113-119) and, **iff the record has an aim-raise script (`weapon_data+0xB8`)**, clears it again
  (146), sets view state 0xF and queues `+0xB8` at 1.25× (`AnimScriptAddSpeed 0x3fa00000`,
  148-150). A record without `+0xB8` aims at once (117-119) — the P2K.
- **The finish sets the bit** — `Player_SetWeaponAnimObj` 0x1a2e38 case 0xf (554-561) [M]: when
  `AnimScriptIsStopped` the bit is set (`|= 1`), the state returns to 0 and the idle script
  re-queues (the `+0x959 = 1` substate arm, 63-73). Case 0x10 (562-567) — the lower — only returns
  to state 0. The lower is queued by `Player_Weapon` 120-138 on the bit's fall: `+0xBC` at 1.25×,
  or `+0xB8` played REVERSED (`AnimScriptAddReverse`, disasm 0x1a26f0) when `+0xBC == 0`.
- **The zoom starts on the bit's rise** — `Player_Weapon` 154-187 [M]: when bit0 rises over bit1
  (the "was aiming" tracker), `+0x8D4` leaves 1.0 and takes the per-weapon aim-zoom table value
  (the player's `+0x1B4` table, filled by `Player_InitAmmoWeapons` 0x1a17f8:51 with
  `SQRT(weapon_data[w]+0x88)`, clamped to `+0x88`); when the bit falls, `+0x8D4 = 1.0` (161).
  `Player_Zoom` 0x1a1a70:18 reads the zoom button only while the bit is set.
- **The free-aim window is bit-gated** — `Player_Aiming` 0x1a1d28:31 [M]: the hip turn-speed path
  runs while the bit is clear; the free-aim accumulation (+0x118/+0x11C, the ±0.296/±0.4 window)
  only while set.
- **The auto-aim lock waits** — `Player_AutoAim` 0x1a7c98:50 [M]: while the bit is clear (and the
  weapon has auto-aim) the `Check_AutoAim` scan runs; once aimed, the scan stops and the lock
  offsets ease toward the manual cursor (the +0x120→+0x118 lerp at 0.5/tick).
- Tick order inside `Player_Update` 0x193600 [M]: `Player_Aiming` (282) → `AnimObjectUpdate`
  (318/358) → `Player_WeaponRecoil`→`Player_SetWeaponAnimObj` (411) → `Player_Weapon` (412) — the
  raise queued by `Player_Weapon` advances from the next tick.

Records [L weapon_data + the disc script census, via the port's own loads]: the Winter Covert
Sniper 0x24 raises with 0x060007b9 (24 frames → ceil(23/1.25) = 19 advance ticks; no `+0xBC` — the
lower plays it reversed), the AIMS-20 0x1a with 0x060007bf (20 frames → 16 ticks), the watch pair
0x54/0x55 with 0x060005a4/0x060005a8 (+0xBC 0x060005a9, 71 frames); the Wolfram P2K 0x02 has no
raise script. Scope flags (`+0x68 & 0x40`) set on 0x24/0x1a/0x54.

## What the port got / what changed

`crates/nf-view/src/scene.rs` (the task's aim block) + `crates/nf-formats/src/weapon.rs` (two
accessors). fix-weapon-anims' raise/lower machine is **held unmerged**
(`s/103k2-fix-weapon-anims2`), so this branch carries the raise anims inside the gate — its model
is kept (same field names `view_aim_anim`/`prev_aiming`-equivalent edges, the 1.25× speeds, the
reverse lower) so the eventual merge is mechanical; the differences are listed under "Left".

1. **The gate** (`Sim::aim_gate`, called from `step_bond` and `step` — the raw button no longer
   reaches any consumer): the press on a `+0xB8` weapon clears the bit and starts the raise
   (+0xB8 @1.25, view time 0); each tick advances it (the hoisted `AnimObjectUpdate` + case 0xf
   finish, so the bit/zoom/free-aim begin ON the raise's last tick); on the finish the bit sets
   and the idle re-queues. The release clears the bit at once and plays the lower (+0xBC @1.25,
   else +0xB8 reversed at −1.25). No `+0xB8` → the bit is the button (unchanged). The raise/lower
   start only from the fully idle machine (switch/reload/fire quiet, view script = idle —
   fix-weapon-anims' condition; the game's lower arm checks view state 0 [M 0x1a24c8:121], the
   raise arm none [S: kept conservative so fire/reload/switch anims are never displaced on the
   press]).
2. **The consumers read the bit**: the free-aim window block (`Player_Aiming`'s port), the
   auto-aim scan/ease branch (`Player_AutoAim`'s port — during the raise it keeps scanning), the
   `drones.bond_aiming` AI flag, the crosshair and the mode-switch gate.
3. **The zoom gate** (`Player_Zoom`'s port): `zoom_target` leaves 1.0 only with the zoom input AND
   the bit set — during the raise the target stays 1.0 (the game's zoom target is written on the
   bit's rise).
4. **The view-script chain**: the aim anim advances in `aim_gate` (the chain skips it — one
   advance per tick, as the game's single anim object); another script displacing the raise/lower
   clears the marker (`AnimScriptAdd` 0x1c6d40 deletes the playing script) so a raise interrupted
   by fire never sets the bit; the `aims_idle` counter resets while the raise/lower play (the
   game's non-idle view states 0xF/0x10, 0x1a2e38:40-58).
5. **`weapon.rs`**: `raise_script()` (+0xB8) and `lower_script()` (+0xBC) accessors with the
   census in the doc comments. Additive only.

## Verification (NF_ISO = the retail onfoot ISO)

`crates/nf-view/tests/fix_aim_raise_live.rs`, 5 tests through the real input path
(`nf_view::load_weapons` + the panel give + `Sim::step_bond`), **all failed before the fix**
(the bit set on tick 1; "the AIMS-20 must not aim on the press tick" was the failing sister of the
P2K test) and pass after:

- `the_snipers_zoom_and_free_aim_begin_on_the_raises_last_tick` — the sniper: the record facts
  (+0xB8 = 0x060007b9, no +0xBC, 24 frames [L]); holding aim+zoom+turn, the bit stays clear, the
  zoom target stays 1.0 and the free-aim cursor stays at rest for the press tick + the 19 advance
  ticks, then **on the raise's last tick** the bit sets, the zoom target takes 10 and the zoom
  moves, and the free-aim window clamps the cursor to −0.296; sisters: the window holds while
  aimed.
- `the_aims20_zoom_and_free_aim_begin_on_the_raises_last_tick` — the same at the AIMS-20's numbers
  (0x060007bf, 20 frames, 16 advance ticks).
- `the_p2k_aims_immediately` — the P2K (no +0xB8) aims and opens the window on the press tick;
  sister: the AIMS-20 under the same single-tick input does not.
- `aim_release_drops_the_bit_then_plays_the_reverse_lower` — the release tick drops the bit and
  the zoom target at once and plays the raise script REVERSED (no +0xBC); after the symmetric 19
  ticks the idle returns and the bit never rose during the lower.
- `the_auto_aim_lock_eases_only_from_the_raises_last_tick` — with auto-aim on and a seeded lock
  pointer the scan can never re-locate, the scan clears it every raise tick (the game keeps
  scanning) and the ease preserves it from the raise's last tick on; sister: the lock survives
  while held.

Regression sweeps on the fixed tree: `fix_guard_misc_live` 6/6 (**the 0c sniper test passes** —
the P2K's instant aim is untouched), `fix_viewmodel_zoom_live` 3/3, `fix_zoom_overlays_live` 6/6
(the watch's manual `aiming = true` + held zoom is bit-compatible), `gadgets_live` 4/4 (the AIMS
side lights, whose idle counter the raise now resets), `gadget_laser_live` 7/7,
`fix_auto_switch_live` 2/2, `walkreal_0700000a/0c/0d_live`, `fix_progress_live` 3/3.

The full four-crate suite (`tools/test/suite.py`, 263 binaries, the final tree merged with
origin/master ed718f8): through its first 200 binaries — **643 passed, 3 failed, 27 ignored, 0
unexpected** — the run outlived the session window (the heavy ISO binaries trail; the totals here
are the log's sum at close). The 3 failed are the `fix_0c_pylon_live` pair +
`probe_pylon_render::probe_render_pylon_only`: `wgpu adapter: NotFound` — this sandbox has no GPU
adapter (documented sandbox artifact; the same panic hits any render test here, before any sim
code runs). `weapon_matrix_live` hit the runner's 600 s cap — the documented pre-existing artifact
(fix-0c-pylon.md records the identical TIMEOUT at matrix_0700001b; the binary's wall clock here is
~710 s and all 28 of its tests pass split through its own NF_MATRIX selector). No aiming-related
binary failed.

## Shared-file notes (minimal, additive)

- `crates/nf-view/src/scene.rs` (the task's file): the `aim_gate` method, the `view_aim_anim`/
  `view_aim_raising` fields (own lines), the zoom/bit/aims_idle gate edits, the chain's
  skip-advance + displaced-marker cleanup, the constructor's two field inits.
- `crates/nf-formats/src/weapon.rs`: the two accessors.
- `docs/WORKLOG.md`: the entry only (the merge conflict with fix-weapon-fx-levels resolved
  append-only, both entries kept).
- No changes to `drone.rs`/`player.rs`/`lib.rs` were needed (`lib.rs`'s noclip path routes through
  `step`, which is gated).

## What's left

- fix-weapon-anims' branch (`s/103k2-fix-weapon-anims2`) carries its own raise/lower machine +
  the `prev_aiming`/`view_alert` fields and the `!fire_state` half of the `aims_idle` gate. When
  it lands, the two machines should be unified (mine owns start+advance+finish; theirs started in
  run()'s chain) — the field names were kept identical to make that a small diff.
- The raise arm's from-idle-only condition (the game's raise branch has no view-state check — a
  press mid-fire replaces the fire script with the raise) is kept conservative; a mid-lower
  re-press likewise does not restart the raise (the game would).
- The 0x55 lower quirk (the taser twin rewrites itself to 0x54 on the aim-off edge,
  `Player_Weapon` 123-129) is not ported.
- The game's two-stage zoom shape (the bit rise targets `SQRT(+0x88)`, the held zoom button
  continues to `+0x88` at 2.5%/tick) — the port's merged mouse-first zoom (target = `+0x88`
  directly, 2.5%/15% rates) was left as merged; only its GATE is new here.

## Live checks for the main agent

- **PCSX2, 07000005, the sniper via the dev menu**: press-and-hold L1 with the scope out — the
  view model plays the raise and the FOV must stay wide until the raise's last frame, then zoom;
  release → the gun lowers (the raise anim backwards) and the FOV pops back at once. Watch
  `BLData+0x60` bit 0 (the aim bit) with gdbwatch: it must rise exactly on the tick
  `Player_SetWeaponAnimObj` case 0xf runs (0x1a2e38), not on the press.
- **PCSX2, the P2K**: L1 → the bit rises the same frame the button is read (no raise anim).
- **nf-view render** (the raise view model, the main agent's GPU): `nf-view <iso> 07000005
  --give 0x24` then hold aim — frames +2, +10, +19 (mid-raise: the scope still at the hip
  position, the FOV wide), +20 (the raise's last tick: the FOV starts closing). The exact
  `--bond` spot: the 07000005 start `[105.91, 25.32, 108.13]` facing the courtyard.
- **nf-view render** (the lower): hold aim to settled, release — frames +1, +10, +19: the gun
  travelling back down, the FOV wide from the first frame.
