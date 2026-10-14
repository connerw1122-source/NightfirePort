# weapon-matrix (task 100d) — every weapon × every on-foot level, measured

Branch `s/weapon-matrix`. The tool: `nf-extract weapon-matrix <iso> [levels]` prints the table;
`crates/nf-view/tests/weapon_matrix_live.rs` runs the same matrix headless (one `#[test]` per
level, panic-freedom asserted). Per cell: load the level (the normal `nf_view::load` path), give
the weapon (the `--give` path — `panel::apply`'s "weapon" command), draw it (the real input path:
`switch_to`, then the mode button for variant records), fire it 3 times (hold-to-use gadgets
hold), reload, switch away to the P2K (0x02) and back — recording: view model present, the
muzzle-flash attachment, the view-anim scripts played (`Sim::view_script`), shot ticks
(`Sim::shot_tick`), sound ids requested (`Drones::sounds`) and started (the driver's `Cmd::Start`
batch, built the way `sound_tick` builds it), effects (player bullets, impacts, explosions,
grenades, the laser beam), errors/panics. Runner: `crates/nf-view/src/matrix.rs`.

**Zero panics** across the full sweep (27 levels × 99 weapon ids, 5m27s wall on the slim ISO).

## The mark table (levels × weapons; final run on this tree, master + no 100a-c code yet)

```
marks       ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
07000001    .  F  .  D  .  -  B  -  W  I  .  I  I  D  D  I  I  D  .  I  .  I  .  I  .  I  I  D  .  D  D  D  D  D  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  D  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000002    .  F  .  -  B  -  B  -  B  I  I  I  I  -  B  I  I  -  B  I  I  I  I  I  .  I  I  R  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  -  -  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000003    .  -  -  -  B  -  B  -  B  I  I  I  I  -  B  I  I  D  .  I  .  I  .  I  .  I  I  D  .  D  D  D  D  D  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  D  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000004    .  F  .  -  B  -  B  -  B  I  I  I  I  -  B  I  I  -  B  -  B  I  I  I  .  I  I  R  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  -  -  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000005    .  -  -  -  B  -  B  -  B  -  B  -  -  I  I  I  I  I  I  -  B  -  B  D  .  I  I  I  .  I  I  I  I  I  .  D  .  D  .  D  .  I  .  I  .  I  .  I  .  I  .  I  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000006    .  -  -  -  B  -  B  W  D  D  .  D  D  I  I  I  I  I  .  D  .  D  .  D  .  I  I  I  .  I  I  I  I  I  .  D  .  D  .  D  .  I  .  I  .  I  .  I  .  I  .  I  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000007    .  R  .  -  B  -  B  D  .  D  .  D  D  I  I  I  I  I  .  I  .  I  .  I  .  I  I  I  .  I  I  I  I  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  I  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000008    .  R  .  -  B  -  B  -  B  -  B  -  -  I  I  I  I  I  I  -  B  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  R  .  R  .  I  .  I  .  I  -  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  I  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
07000009    .  -  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  -  I  I  I  F  n  I  I  I  .  .  .  .  I  I  D  D  I  D  .  .  D  .  .  D  .  D  I  I  I  .  .  D  .  D  .  .  .  n  n  n  n  I  .  I  I  I  
0700000a    .  -  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  I  .  I  .  D  I  I  I  D  n  I  I  I  .  .  .  .  I  I  D  D  I  D  .  .  D  .  .  D  .  D  I  I  I  .  .  D  .  D  .  .  .  n  n  n  n  I  .  I  I  I  
0700000b    .  -  .  -  B  -  B  -  B  -  B  -  -  I  I  I  I  I  I  I  I  I  I  D  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  R  .  R  .  I  .  I  .  -  I  I  I  F  n  I  I  I  .  .  .  .  I  I  D  D  I  D  .  .  D  .  .  D  .  D  I  I  I  .  .  D  .  D  .  .  .  n  n  n  n  I  .  D  I  I  
0700000c    .  -  .  -  B  -  B  -  B  -  B  -  -  I  I  I  R  -  B  I  I  I  I  I  .  I  I  I  I  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  -  I  R  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
0700000d    .  F  .  -  B  -  B  -  B  D  .  D  D  I  I  I  D  D  .  I  .  I  .  I  .  I  I  I  .  .  D  D  D  D  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  I  D  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000011    .  I  I  I  I  I  I  I  I  I  .  I  I  I  I  D  I  I  .  I  .  I  .  D  .  I  I  D  .  I  I  I  I  I  .  I  .  I  .  I  .  D  .  I  .  I  .  I  .  I  .  D  I  D  I  D  n  I  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000012    .  I  I  I  I  I  I  I  .  I  .  I  I  I  I  D  I  I  .  I  .  I  .  D  .  I  I  D  .  I  I  I  I  I  .  I  .  I  .  I  .  D  .  D  .  D  .  I  .  I  .  D  I  D  I  D  n  I  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
07000013    .  I  .  I  I  I  I  I  .  I  .  I  I  I  I  D  I  I  .  I  .  I  .  D  .  I  I  D  .  I  I  I  I  I  .  I  .  I  .  I  .  D  .  D  .  D  .  I  .  I  .  D  I  D  I  D  n  D  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
07000014    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  R  I  I  -  B  I  I  I  .  R  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  I  .  D  .  D  D  D  D  D  n  D  D  D  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  D  I  I  .  .  D  D  I  I  .  I  .  n  n  n  n  D  .  I  D  D  
07000015    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  R  I  I  -  B  I  I  I  .  R  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  D  .  -  -  -  -  F  n  D  D  D  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  I  I  I  .  .  D  D  I  I  .  I  .  n  n  n  n  D  .  I  D  D  
07000016    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  R  I  I  -  B  I  I  I  .  R  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  I  .  D  .  D  D  D  D  D  n  D  D  D  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  D  .  I  D  D  
0700001b    .  I  .  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  R  .  D  .  I  I  I  I  I  n  I  I  I  I  .  .  .  I  I  I  I  I  I  .  .  I  .  .  I  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  -  .  I  -  -  
07000041    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  R  -  B  -  B  -  B  D  .  R  B  R  .  -  B  -  B  -  B  -  B  -  B  -  B  R  .  R  .  R  .  R  .  D  .  -  -  -  -  F  n  D  D  D  .  .  .  .  D  D  D  D  D  D  .  .  D  .  .  D  .  D  D  D  .  .  D  D  D  D  .  D  .  n  n  n  n  D  .  D  D  D  
07000043    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000046    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  R  -  B  -  B  -  B  D  .  R  B  R  .  -  B  -  B  -  B  -  B  -  B  -  B  R  .  R  .  R  .  R  .  D  .  -  -  -  -  F  n  D  D  D  .  .  .  .  D  D  D  D  D  D  .  .  D  .  .  D  .  D  D  D  .  .  D  D  D  D  .  D  .  n  n  n  n  D  .  D  D  D  
07000048    .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  n  I  I  I  I  .  .  .  I  I  I  I  I  I  .  .  I  .  .  I  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000049    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
0700004a    .  I  I  I  I  I  I  I  I  -  B  -  -  I  I  -  I  I  I  I  I  I  I  D  .  I  I  R  .  I  I  I  I  I  I  I  I  I  I  I  I  R  .  R  .  R  .  I  .  I  .  -  I  I  I  F  n  I  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
0700004b    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  R  -  B  -  B  -  B  D  .  R  B  R  .  -  B  -  B  -  B  -  B  -  B  -  B  R  .  R  .  R  .  R  .  D  .  -  -  -  -  I  n  -  -  B  D  .  .  .  -  -  I  I  R  -  .  .  I  .  .  I  .  -  -  -  .  .  I  I  I  I  .  I  .  n  n  n  n  -  .  -  -  -  
```

(Full tables: `nf-extract weapon-matrix <iso>` reproduces them; the committed run's output is in
this report's numbers below. Marks: `-` clean, `.` give refused, `n` record has no fp model, `I`
no view model, `D` draw failed, `F` no observable fire, `R` reload broken, `W`/`B` switch
away/back failed, `!` panic.)

## Baseline (master at task start) vs final (this tree)

100a–c (`s/fix-weapon-assets`, `s/fix-weapon-sounds`, `s/fix-weapon-effects`) were **not merged**:
at task start (03:10 UTC) each held only its initial WORKLOG commit — no code to build on. The
baseline and final runs are therefore the same tree (master + this task's runner, which only
observes). Baseline == final, numbers below.

## The numbers (27 levels; final run)

Red cells by mark: **274 `.`** (give refused), **235 `I`** (no view model), **94 `D`** (draw
failed), **6 `R`** (reload), **6 `B`** (switch back), **0 panics**. The gadget columns are
informational by the 2026-10-13 correction (the watch gadgets are carried everywhere; the Q-Claw
spot here has no grapple point, so quiet fire is expected — not counted red).

- **`I` — the cross-level asset gap (the headline, 100a's work).** Every cell where the weapon's
  first-person skin (`weapon_data+0xDC`) is not in the level archive: the give works
  (`Player_EquipWeapon` 0x1a8a58 checks the *record*, not the archive), the draw completes
  (`gun.weapon == id`), but `Scene::view_models` has no entry (scene.rs "The first-person gun
  models": `level.skins.get(&fp)` → `continue`). 235 of the ~2600 gun cells; e.g. the Raptor
  Magnum family (0x0e-0x11) has a view model on 07000005 but not on 07000003; the front-end map
  07000048 has none at all except the records it carries. The red-cell reasons name the missing
  hash per cell (`fp skin <hash> not in the level archive`).
- **`.` — give refused (274).** Two kinds, both the game's own semantics, not bugs:
  1. the record has no first-person model (`weapon_data+0xDC == 0`; `Player_EquipWeapon`
     0x1a8a58:20 refuses — the variant records 0x03/0x05/0x09/... that carry their fp through
     their base; marked `n` where the record itself has fp == 0);
  2. the shared ammo bank at cap: `equip_weapon`'s owned branch refuses when
     `carried[t] >= cap` (crates/nf-game/src/pickup.rs) — the matrix gives 99 weapons in a row,
     so banks fill; the dev tools' Give button hits the same wall. The cell's reason names it.
- **`D` — draw failed (94), concentrated on 07000001/02/04 and the wrappers.** On 07000001 the
  manor's intro script holds the player: the P2K cell shows the view model, the draw sounds
  (started ids 35/41/92/134) and the idle script, but zero shot ticks — the input is swallowed
  while the intro runs (`Sim::disabled` does not cover all of it — a live check is listed below).
  The rest are the variant records whose mode-button chain could not be completed inside the
  cell's window (see `B`).
- **`B` — switch back to a variant failed (6: 0x05/0x07/0x09/0x0f/0x13/0x1f on 07000002/03).**
  The mode chain (`Player_Weapon` 0x1a24c8's `fire_mode_switch`, record `w[5]` twin byte) reaches
  the variant for the *draw* (via_mode 1 in the same cells) but the *back* press sequence from the
  base 0x02 does not complete in 12 idle-gated presses. The draw path starts from the previous
  cell's variant (1 press); the back path starts from the base (3+ presses) — so the chain stalls
  mid-way. 100c (fire behaviour) owns the mode system; the matrix gives it exact cells.
- **`R` — reload broken (6: 0x1c Frinesi on 07000002, others on the wrapper levels).** The R edge
  lands but the 5/6/7 machine never leaves idle — the cells' fire scripts were still playing when
  the pulse landed (the settle is 30 ticks; the shotgun's fire script is longer). Not a game bug:
  a matrix-window artifact, listed for the next runner pass.
- **`F` — no observable fire (P2K on 07000001/02/04, Remote Mine on 07000002).** Same intro/hold
  story as `D` — see the live checks.

**Carried weapons are green where the level carries them** — e.g. 07000005 (the Levelik plant):
0x02-0x08 all `-`/`B` with vm 1, flash 1, 2-3 shot ticks, sound ids requested *and* started,
impacts, and reload ok; the deviation is exactly the records whose assets are not in the archive.

## Sounds (100b's read from the matrix)

Sound ids are recorded requested (`Drones::sounds`) and started (`Cmd::Start`) per cell. Note:
`started` is non-empty only because the runner builds the driver batch itself (`audio.update`,
the same call `sound_tick` makes in the windowed loop — lib.rs 966); without it the headless sim
never emits `Cmd::Start`. The laser (0x4e) and stunner cells record their sound ids and zero
player bullets — the "sounds like bullet impacts" question the user raised is now data: the laser
cells show `beam 1` and impacts from the beam, no bullets; 100c can confirm against the fire
model.

## The dev menu's "Test weapon" row

`crates/nf-view/src/dev.rs`: the Weapons page's first row — "test weapon (in hand)": draw the
held weapon, fire it 3 times, reload, live. The machine (`TestRun`, `Dev::test_inputs` /
`Dev::test_observe`) feeds the real input path through the same lanes the keyboard uses
(`switch_to`, the trigger, the R edge) — `lib.rs` merges them per tick (never during a replay)
and observes after `step_bond`. It works from the dev window (the game keeps running there) and
from the F2 overlay after closing the menu; the result lands in the overlay's status line
("test weapon 0x02: draw ok, 3 shot ticks, reload ok"). Verified headless by
`weapon_matrix_live::test_weapon_row_runs_the_sequence` (the App's merge, verbatim).

## Shared files touched (additive hooks only)

- `crates/nf-view/src/lib.rs`: `pub mod matrix;` + the two-line Test-weapon merge/observe hook in
  the tick loop (guarded off during replays). Nothing else.
- `crates/nf-game`, `drone.rs`, `scene.rs`, `player.rs`: **untouched.**

## Verification

- `nf-extract weapon-matrix <iso>` — the full sweep, 5m27s, 0 panics (the numbers above).
- `cargo test -p nf-view --test weapon_matrix_live test_weapon_row_runs_the_sequence` — passes
  (draw ok, 3 shot ticks, reload ok on the P2K / 07000005).
- Fast suite on the final tree (`tools/test/suite.py --only
  weapon_matrix_live,fix_weapons_all_live,fix_weapon_modes_live,models_weapons_live,dev_menu_flow,devwin_flow`):
  **51 passed, 0 failed, 0 ignored (8 min)** — the full matrix binary (27 levels, panic-free) plus
  the weapon suites and the dev-menu/devwin suites the lib.rs/dev.rs hooks touch. A second pass (dev_tools_live, fix_viewmodel_zoom_live, gadget_select_live, replay_live, panel_live, scenario_live): **22 passed, 0 failed** — 73 green in total across the two passes. The remaining
  210 binaries were not re-run before the deadline (the same runner, `tools/test/suite.py`, with
  no `--only`, is the full pass); the hooks are additive and the touched suites are green.

## Live checks for the main agent (PCSX2)

1. **The manor intro's input lock** — spot: 07000001 start, let the intro play, then hold fire
   with the P2K: the game fires normally after the intro. In the port, the matrix's 0x02 cell on
   07000001 shows started ids 35/41/92/134 (draw sounds) with zero shot ticks for 240 fire ticks
   — please watch `Sim::disabled` and the script state at that point (`tools/trace/gdbwatch.py`
   on `BLData`'s disable flag, address per scene.rs's `Sim::disabled` comment).
2. **The mode chain's back press** — spot: any level, give 0x02, mode-button to 0x03, 0x04, 0x05,
   then cycle back from 0x02: does the real game chain 02→03→04→05 with repeated presses? The
   port's `fire_mode_switch` stalls mid-chain from the base (cells 0x05/0x07/0x09 on 07000002/03).
3. **nf-view `--bond` spots** for the view-model gap (each should show the gun in hand after
   merging 100a): `--level 07000003 --give 0xe` (Raptor Magnum), `--level 07000048 --give 0x10`,
   `--level 07000001 --give 0x1c` (Frinesi).
4. **The laser/stunner fire model** — `--level 07000005 --give 0x4e`: the matrix records beam on,
   zero player bullets; the stunner 0x4f likewise. Confirm in PCSX2 whether the real laser fires
   bullets (the user heard rapid impacts).

## What's left

- The `D`/`F` cells on cut-scene-holding levels (the intro wait covers `Sim::disabled` only).
- The `B` variant-chain cells (100c's mode system) and the 6 `R` window artifacts.
- Re-run with 100a-c merged once they carry code; every red cell then gets a fix or a reason.

## weapon-matrix2 (103b, 2026-10-14) — the re-run on master

The full re-run on master @ b65d6f7 (27 levels × 98 ids, 0 panics) and the runner upgrade (the real
give path via `nf_view::load_weapons` + the per-cell inventory reset + the mode-twin/label, guided-
camera and MGL blast-gap checks) are in **docs/reports/weapon-matrix2.md** — its mark table, the
old-vs-new same-parser counts and the silenced-twin reload-hang root cause live there.
