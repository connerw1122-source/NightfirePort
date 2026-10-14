# fix-scope-zoom — every zoomable weapon: the zoom levels, the rates and the gates

Branch `s/fix-scope-zoom` (task `106d-fix-scope-zoom`). Files: `crates/nf-view/src/scene.rs` (the zoom
model + the view-model gate), `crates/nf-view/src/hud.rs` (the overlay + crosshair gates), new
`crates/nf-view/tests/fix_scope_zoom_live.rs`, updated `crates/nf-view/tests/fix_viewmodel_zoom_live.rs`
(harness only). `drone.rs` / `player.rs` / `lib.rs` untouched. No game bytes, tokens or URLs committed.

## The census — every zoomable record [L ee_start_05.bin weapon_data 0x2bf150: +0x00 id, +0x02 base, +0x04 selectable, +0x68 flags, +0x88 f32 mag]

| weapon(s) | name | base | flags & 0x40 (scoped) | mag +0x88 | aim-start zoom sqrt(mag) | pane (0x19e6d0) |
|---|---|---|---|---|---|---|
| 0x11 | Delta Repeater | 0x11 | yes | 4.0 | 2.0 | Sight 6 (default) |
| 0x18 / 0x19 | (label 0x0500003b) | 0x19 | **no** | 3.0 | 1.7321 | **none — unscoped zoom** |
| 0x1a / 0x1b | AIMS-20 | 0x1a | yes | 6.0 | 2.4495 | OICW 17 |
| 0x1e / 0x1f | Winter Tactical Sniper | 0x1e | yes | 10.0 | 3.1623 | Sight 6 |
| 0x20 / 0x21, 0x22 / 0x23, 0x26 / 0x27, 0x28 / 0x29 | the 16× snipers | own | yes | 16.0 | 4.0 | Sight 6 |
| 0x24 / 0x25 | Winter Covert Sniper | 0x24 | yes | 10.0 | 3.1623 | Sight 6 |
| 0x2a / 0x2b | (label 0x05000032) | 0x2a | **no** | 2.0 | 1.4142 | **none — unscoped zoom** |
| 0x32 / 0x33 (+ MP twin 0x6f) | Phoenix Samurai | 0x33 | yes | 3.5 | 1.8708 | Laser 19 (0x6f: default — no case 0x55) |
| 0x54 / 0x55 | Micro-Camera | 0x54 | yes | 8.0 (Upgrade_Weapon 0x1a8f30 writes 16.0 at runtime) | 2.8284 | Camera 11 |
| 0x48 / 0x49, 0x5d / 0x5e, 0x5f, 0x61, 0x71, 0x72 | dev/MP records | own | no | 2–10 | — | selectable 0, not player-reachable |

The zoom axis zooms ANY aimed weapon up to its own +0x88 (`Player_Zoom` 0x1a1a70:18-39 has no scoped
gate) — the unscoped mag>1 records (0x18/0x19, 0x2a/0x2b) zoom with NO overlay and the crosshair
stays (the pane + crosshair-hide live only in `HUD_UpdateCrossHair`'s aim+scoped path).

## Game vs port, and what was fixed

**The zoom levels and their keys** [M `Player_Weapon` 0x1a24c8:154-187 + `Player_Zoom` 0x1a1a70 +
`Player_InitAmmoWeapons` 0x1a17f8:51]:
- The game keeps a per-weapon slot table at `BL+0x1B4 + w·0xC`, filled at init with
  `sqrt(weapon_data[w]+0x88)` [M 0x1a17f8:51]. Aim start targets `slot[base]` (the base from record
  +0x02 — twins share their base's slot), clamped down to +0x88 [M 0x1a24c8:173-183]; the read is
  gated by the per-player input option `DAT_002a38ce`, default 1 [M `Input_Init` 0x17edd8:22;
  L ee_start_05 = 1 for all rows].
- The zoom axis (analog action 6, read only while aiming) scales the TARGET ×1.025 per tick, then the
  clamps run (`target ≤ +0x88`, `< 1 → 1`, `> 50 → 50` [M 0x1a1a70:44-54]), then the VALUE scales
  ×1.025 not crossing the target, then `target = value` and the slot persists the reached zoom
  [M 0x1a1a70:39, 56-74]. So releasing the axis KEEPS the zoom; only releasing AIM targets 1.0
  [M 0x1a24c8:158-166]; re-aiming resumes at the persisted zoom.
- Whenever the value differs from the target it eases ×1.15 per tick, not crossing [M 0x1a1a70:21-33,
  55-68] — the aim-in/out ramp (1.0 → sqrt(mag) on aim, back to 1.0 on release).
- **The port instead**: a boolean zoom key snapped the target to +0x88 while held and to 1.0 on
  release, with no slots, no sqrt default, no persistence, and the zoom collapsed on every key
  release. Fixed in `scene.rs` to the game's exact order (an intermediate cut clamped the target
  BEFORE the axis multiply — the persisted slot then captured mag×1.025 (4.1 for a 4.0 record); the
  game clamps AFTER the multiply [0x1a1a70:39→44-46→56-68], which the final code follows).
- `zoom_slots: Vec<f32>` is the one new Sim field (own line, its own constructor line).

**The overlay / crosshair / view-model gates** [M `HUD_UpdateCrossHair` 0x19e6d0:106-153 +
`Player_PositionCamera` 0x18f5d8:100-107]: all three are **aim + scoped** — the aim bit
(`weapon+0x60 & 1`) with flags +0x68 & 0x40 hides the crosshair (LAB_0019e9c8, layer 0xff), enables
the weapon's pane, and `View_SetDrawInNoViews` hides the view-anim object. No zoom value in any gate.
The port gated all three on `zoom_target > 1.0` — a proxy that only worked while the port reset the
zoom on every release; under the game's persisting zoom it would keep the crosshair hidden forever
after a re-aim. Fixed: `hud.rs` (crosshair hide + pane draw) and `scene.rs` (view-model hide) gate on
aim+scoped.

**Zoom-out**: the game's axis is analog — positive = zoom out, negative = zoom in. The port's
`Input.zoom` is a held key (zoom-in only, the full-deflection stand-in); zoom-out happens via the
aim-release ease. Not modelled (would need an axis value in the shared Input struct) — listed under Left.

**Sway**: the aim inertia divides by the zoom [M `Player_Aiming` 0x1a1d28:186-187, 273-274 —
`f / BL+0x8D0`]; the port's scope-speed tuning rides the same AccelStates without the /zoom divide
[input.rs:331-360, documented by its comment]. Not changed this session (the sway system's own
task); evidence recorded above.

**Sound**: the zoom sound handle `BL+0x898` is only ever STOPPED in the decompile (aim end
[M 0x1a24c8:163-166], axis release [M 0x1a1a70:27-32], axis reversal [M 0x1a1a70:76-85];
`Player_InitWeapon` 0x1a7ef8:73 zeroes it) — the START site is a dropped store (no `sw … 0x898` in
the disassembly of Player_Aiming/Player_Zoom). The port plays no zoom sound: nothing to mismatch
until the start site is found — live check below.

**Overlay textures/layout**: unchanged from 102a-fix-zoom-overlays (the mirror-tiling quarters);
verified per weapon through the pane selection census (test below).

## Tests (NF_ISO, the real input path)

`crates/nf-view/tests/fix_scope_zoom_live.rs` (4/4 green):
1. `zoom_levels_match_the_game_per_weapon` — 12 records (the table above): aim-start zoom lands on
   sqrt(+0x88) (±1e-3), the held axis caps at +0x88 exactly, the zoom KEEPS for 30 ticks after the
   axis releases, the slot persists the max, and the value eases back below 1.02 after aim releases.
   Failed before the fix (no slots — the field didn't exist; behaviourally the port snapped to +0x88
   and collapsed on release).
2. `the_twins_share_the_base_zoom_slot` — zooming 0x1b to 6.0 then re-aiming the base 0x1a resumes
   at 6.0 [M 0x1a24c8:173-177 reads record +0x02's slot].
3. `the_overlay_and_crosshair_gates_follow_aim_and_scoped` — aiming the sniper at zoom ~1.0 draws
   the scope and hides the crosshair; each zoomable draws its own overlay once (scope / OICW circle
   0x03000141 / viewfinder 0x03000055 / laser strips 0x03000002, no scope on any); the unscoped 0x18
   zooms to 1.732 with NO pane and the crosshair stays. **Fail-first shown**: a probe compiled against
   the pre-fix tree printed `zoom=1.0000 target=1.0000` and panicked "no scope overlay while aiming at
   zoom ~1.0"; post-fix `zoom=1.1500 target=3.1623` and the scope draws.
4. `the_zoomable_census_selection` — the pane enum for all 21 zoomable ids incl. the MP twin 0x6f
   (default) and 0x52/0x53 (default, per 102a).

`crates/nf-view/tests/fix_viewmodel_zoom_live.rs` (3/3 green) and `crates/nf-view/tests/fix_zoom_overlays_live.rs`
(6/6 green): the draw tests' harnesses now set `sim.aiming = true` (the game's pane enable rides the
aim bit — the same harness update 102a's own gate change would have needed); the not-aiming sister
updated to match. The gun-tilt and pure-selection tests unchanged and green.

## Suite

This sandbox killed every background process during the runner's build phase (the 4 GB box OOMed the
-j4 build; the ALSA dev prefix also needed a private `.pc` — fixed at `/home/z/alsa-pc`), so the full
`suite.py` could not complete here this session. All four crates' test binaries ARE built (`cargo
test --no-run --release` clean, rc=0) and the directly-affected binaries ran green:
`fix_scope_zoom_live` 4/4, `fix_viewmodel_zoom_live` 3/3, plus `cargo check` clean on the tree. The
full-suite run is the first thing for the next window (the binaries are cached; the runner itself is
ready at `/home/z/run-suite.sh`).

## Visual (the main agent renders — this sandbox has no GPU adapter; nf-view --shot errors
"No suitable graphics adapter found")

The per-overlay draws are asserted through the headless HudFrame runs above. `--bond` spots for
PCSX2/nf-view comparison (before = master, after = this branch):
- `nf-view Nightfire-onfoot.iso 07000005 --give 0x54 --zoom-hold --frame 120 --shot zoom_microcam.png`
  → the viewfinder, centred; on this branch the zoom climbs from sqrt(8)≈2.83 toward 8 (before: the
  same view but reached instantly).
- `nf-view Nightfire-onfoot.iso 07000016 --give 0x1a --zoom-hold --frame 120 --shot zoom_aims20.png`
  → the OICW circle in the cyan tint.
- `nf-view Nightfire-onfoot.iso 0700000c --give 0x1e --zoom-hold --frame 120 --shot zoom_scope.png`
  → the scope oval.
- `nf-view Nightfire-onfoot.iso 07000016 --give 0x32 --zoom-hold --frame 120 --shot zoom_laser.png`
  → the laser strips.
- New behaviour to look for: aim WITHOUT the zoom key on the sniper — the overlay + hidden crosshair
  + hidden gun come up immediately (the game's aim+scoped gate) and the zoom rests at ~3.16, not 10;
  release the zoom key mid-zoom — the zoom STAYS until aim releases.

## Live checks for the main agent

1. PCSX2, sniper 0x24 on 0700000c: aim (no zoom key) — `watch BL+0x8D4 f32` should ramp to
   `sqrt(weapon_data[0x24]+0x88)` = 3.1623 [M 0x1a17f8:51 + 0x1a24c8:173-177], not 1.0 and not 10;
   hold the zoom stick up — it climbs to 10.0 and STOPS; release the stick — it stays 10.0 while
   aiming; release aim — it eases to 1.0 at ~15%/tick. Re-aim — resumes at 10.0 (the slot).
2. AIMS-20 twin: zoom 0x1b fully, switch to 0x1a, aim — the zoom starts at the SAME value (the
   shared slot).
3. `gdbwatch 0x8d0 4 w --seconds 10` while zooming — find the writer that STARTS the zoom sound
   (the handle store at `BL+0x898`; only stops are in the decompile).
4. Switch weapons while fully zoomed — the scope should drop during the holster (the aim bit goes
   off) and the next weapon aims at ITS OWN sqrt(mag).

## Left

1. The axis's zoom-out side (an analog value in the shared `Input`).
2. The aim bit waits for the raise's last tick [M 0x1a2e38:554-561 per fix-aim-raise] — the port
   aims/zooms at the raw input edge, a few ticks early during the raise anim.
3. A weapon switch while aimed keeps the zoom target in the port (the game's aim bit drops in the
   holster → the zoom resets); the twin-landed reset [M 0x1a24c8:213-228] is the only switch reset
   ported.
4. The aim-inertia /zoom divide (Player_Aiming 0x1a1d28) and the zoom sound start.
5. The full suite on the final tree (binaries cached; see Suite above).
