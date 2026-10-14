# weapon-matrix2 (task 103b) — the matrix re-run on today's master, the runner upgraded to the real give path

Branch `s/weapon-matrix2` (from `origin/master` @ b65d6f7). The tool: `nf-extract weapon-matrix <iso>`;
`crates/nf-view/tests/weapon_matrix_live.rs` runs the same matrix headless. Files this task changed:
`crates/nf-view/src/matrix.rs` (the runner), `crates/nf-view/tests/weapon_matrix_live.rs` (the sister
test), `crates/nf-view/examples/probe_matrix_draw.rs` (the draw-machine probe used for the root cause
below). **No shared files touched** (`drone.rs`, `scene.rs`, `player.rs`, nf-view `lib.rs`: unchanged).

## What ran

- **Full re-run on master, 27 levels × 98 weapon ids, 0 panics** (the 10:29 UTC binary — master's
  code with the pre-upgrade runner; split across two invocations because one full sweep no longer
  fits a 10-minute tool window). The mark table below is that run, next to 100d's.
- **The runner upgraded mid-task** (the task's "check the new behaviours"): see "The runner upgrade".
  Its full 27-level sweep is the next window's first item (≈16 min); its 07000005 row is verified
  here and quoted below.
- Comparison parse: `scratch/parse_matrix.py` (this sandbox) parsed both tables with the SAME rules,
  positionally aligned. The old report's prose counts (274/235/94/6/6) counted a different subset —
  the numbers below are the apples-to-apples ones.

## The mark table (master re-run)

```
marks       ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
07000001    .  F  .  D  .  -  B  -  W  I  .  I  I  D  D  I  I  D  .  I  .  I  .  I  .  I  I  D  .  D  D  D  D  D  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  D  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000002    .  F  .  -  B  -  B  -  B  I  I  I  I  -  B  I  I  -  B  I  I  I  I  I  .  I  I  R  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  -  -  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000003    .  -  -  -  B  -  B  -  B  I  I  I  I  -  B  I  I  D  .  I  .  I  .  I  .  I  I  D  .  D  D  D  D  D  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  D  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000004    .  F  .  -  B  -  B  -  B  I  I  I  I  -  B  I  I  -  B  -  B  I  I  I  .  I  I  R  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  -  -  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  I  I  I  .  .  I  I  I  D  .  D  .  n  n  n  n  I  .  I  I  I  
07000005    .  -  -  -  B  -  B  -  B  -  B  -  -  I  I  I  I  I  I  -  B  -  B  D  .  I  I  I  I  I  I  I  I  I  I  -  B  -  B  -  B  I  I  I  I  I  .  I  .  I  .  I  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000006    .  -  -  -  B  -  B  W  D  D  .  D  D  I  I  I  I  I  .  D  .  D  .  D  .  I  I  I  .  I  I  I  I  I  .  D  .  D  .  D  .  I  .  I  .  I  .  I  .  I  .  I  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000007    .  R  .  -  B  -  B  D  .  D  .  D  D  I  I  I  I  I  .  I  .  I  .  I  .  I  I  I  .  I  I  I  I  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  D  I  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  I  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000008    .  R  .  -  B  -  B  -  B  -  B  -  -  I  I  I  I  I  I  -  B  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  R  .  R  .  I  .  I  .  I  -  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  I  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
07000009    .  -  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  -  I  I  I  F  n  I  I  I  .  .  .  .  I  I  D  D  I  D  .  .  D  .  .  D  .  D  I  I  I  .  .  D  .  D  .  .  .  n  n  n  n  I  .  I  I  I  
0700000a    .  -  .  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  I  .  I  .  I  .  D  I  I  I  D  n  I  I  I  .  .  .  .  I  I  D  D  I  D  .  .  D  .  .  D  .  D  I  I  I  .  .  D  .  D  .  .  .  n  n  n  n  I  .  I  I  I  
0700000b    .  -  .  -  B  -  B  -  B  -  B  -  -  I  I  I  I  I  I  I  I  I  I  D  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  W  D  D  .  I  .  I  .  D  I  I  I  F  n  I  I  I  .  .  .  .  I  I  D  D  I  D  .  .  D  .  .  D  .  D  I  I  I  .  .  D  .  D  .  .  .  n  n  n  n  I  .  D  I  I  
0700000c    .  -  .  -  B  -  B  -  B  -  B  -  -  I  I  I  R  -  B  I  I  I  I  I  .  I  I  I  I  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  .  I  .  I  .  I  I  D  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
0700000d    .  F  .  -  B  -  B  -  B  D  .  D  D  I  I  I  D  D  .  I  .  I  .  I  .  I  I  I  .  .  D  D  D  D  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  .  I  I  D  I  D  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  D  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000011    .  I  I  I  I  I  I  I  I  I  .  I  I  I  I  D  I  I  .  I  .  I  .  D  .  I  I  D  .  I  I  I  I  I  .  I  .  I  .  I  .  D  .  I  .  I  .  I  .  I  .  D  I  D  I  D  n  I  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000012    .  I  I  I  I  I  I  I  .  I  .  I  I  I  I  D  I  I  .  I  .  I  .  D  .  I  I  D  .  I  I  I  I  I  .  I  .  I  .  I  .  D  .  D  .  D  .  I  .  I  .  D  I  D  I  D  n  I  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
07000013    .  I  .  I  I  I  I  I  .  I  .  I  I  I  I  D  I  I  .  I  .  I  .  D  .  I  I  D  .  I  I  I  I  I  .  I  .  I  .  I  .  D  .  D  .  D  .  I  .  I  .  D  I  D  I  D  n  D  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
07000014    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  R  I  I  -  B  I  I  I  .  R  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  D  .  D  D  D  D  D  n  D  D  D  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  D  I  I  .  .  D  D  I  I  .  I  .  n  n  n  n  D  .  I  D  D  
07000015    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  R  I  I  -  B  I  I  I  .  R  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  D  .  -  -  -  -  F  n  D  D  D  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  D  I  I  .  .  D  D  I  I  .  I  .  n  n  n  n  D  .  I  D  D  
07000016    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  R  I  I  -  B  I  I  I  .  R  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  D  .  -  -  -  -  D  n  D  D  D  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  D  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  D  .  I  D  D  
0700001b    .  I  .  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  R  .  D  .  I  I  I  I  I  n  I  I  I  I  .  .  .  I  I  I  I  I  I  .  .  I  .  .  I  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  -  .  I  -  -  
07000041    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  R  -  B  -  B  -  B  D  .  R  B  R  .  -  B  -  B  -  B  -  B  -  B  -  B  -  R  -  R  R  .  R  .  D  .  -  -  -  -  F  n  D  D  D  .  .  .  .  D  D  D  D  D  D  .  .  D  .  .  D  .  D  D  D  .  .  D  D  D  D  .  D  .  n  n  n  n  D  .  D  D  D  
07000043    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000046    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  R  -  B  -  B  -  B  D  .  R  B  R  .  -  B  -  B  -  B  -  B  -  B  -  B  -  R  R  .  R  .  R  .  D  .  -  -  -  -  F  n  D  D  D  .  .  .  .  D  D  D  D  D  D  .  .  D  .  .  D  .  D  D  D  .  .  D  D  D  D  .  D  .  n  n  n  n  D  .  D  D  D  
07000048    .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  .  I  .  I  I  I  I  I  n  I  I  I  I  .  .  .  I  I  I  I  I  I  .  .  I  .  .  I  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
07000049    .  -  -  -  B  -  B  -  B  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  I  .  I  I  I  I  F  n  I  I  I  .  .  .  .  I  I  I  I  I  D  .  .  I  .  .  D  .  I  I  I  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  I  I  I  
0700004a    .  I  I  I  I  I  I  I  I  -  B  -  -  I  I  -  I  I  I  I  I  I  I  D  .  I  I  R  .  I  I  I  I  I  I  I  I  I  I  I  I  R  .  W  D  D  .  I  .  I  .  D  I  I  I  F  n  I  D  D  .  .  .  .  I  I  I  I  I  I  .  .  I  .  .  D  .  D  D  D  .  .  I  I  I  I  .  I  .  n  n  n  n  I  .  D  I  I  
0700004b    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  R  -  B  -  B  -  B  D  .  R  B  R  .  -  B  -  B  -  B  -  B  -  B  -  B  R  .  W  D  D  .  D  .  D  .  D  D  -  -  I  n  -  -  B  D  .  .  .  -  -  I  I  R  -  .  .  I  .  .  I  .  -  -  -  .  .  I  I  I  I  .  I  .  n  n  n  n  -  .  -  -  -  
```

marks: `-` clean | `!` panic | `.` give refused | `n` record has no fp model | `I` no view model |
`D` draw failed | `F` no observable fire | `R` reload broken | `W` switch-away failed | `B`
switch-back failed | `S` silent. (The upgraded runner adds `M` mode press failed, `G` guided camera
missing, `X` blast timing wrong — none of the 07000005 upgraded-row cells carry them.)

## Old vs new, same parser

OLD: `.` 630, `B` 126, `D` 286, `F` 15, `I` 1203, `R` 43, `W` 2 — 46 cells changed mark, 0 new
panics. The moves are small and mostly the harness's own randomness at cell edges (the mode chain's
press budget, reloads landing inside/outside the window): `I` ±4, `D` +6, `R` −8, `B` +3, `.` −13.
No cell regressed from clean to broken that the decompile round below does not explain. The full
diff list is reproducible with the runner (`nf-extract weapon-matrix <iso>`).

## The runner upgrade (the task's new behaviours)

`matrix.rs` now: (1) loads every level through `nf_view::load_weapons` with the whole catalog — the
REAL give path (100a's asset merge; the old runner gave through `panel::apply` on a plain load, so
it measured stubs the user can never see); (2) resets the inventory per cell from the load's own
state (the old `.` wall — 630 cells — was the shared ammo bank filling across 99 consecutive gives;
`Player_EquipWeapon` 0x1a8a58's owned branch refuses at cap, and no play session gives 99 weapons in
a row); (3) checks, per cell:

- **the fire-mode chain** — record twin byte +0x05 / mode count +0x2e [M `Player_Weapon`
  0x1a24c8:204-260, `fire_mode_switch`]: presses the mode button with the machine idle, records
  whether the twin got equipped or the mode byte cycled, and both mode labels (record +0x30 +
  mode·4; `HUD_UpdateAmmoPane` 0x19a7f0). New mark `M`.
- **the guided camera for 0x2C** — `Sim::missile_cam` while the missile lives (weapon +0x6C & 4 —
  only the AT-420 Sentinel 'Guided' 0x2C [L ee_start_05.bin]; `Bullet_init` 0x12c5a0:234-254). New
  mark `G`.
- **the MGL impact vs delay** — ticks from the first grenade spawn to the first explosion (a
  230-tick blast watch catches the fused twins that outlive the fire presses). The impact records
  (0x2A/0x1B, no +0x70 & 0x200 fuse bit) blast on first contact; the fused records tick 210/300/420
  [M `Bullet_DoTrails` 0x12d890:262-281]. New mark `X` (0x2a gap > 90, or 0x2b blasting inside 150).
- **the gadgets' beam draws** — the beam flag per gadget cell already recorded (`beam`); with the
  give-path fix the gadget cells now carry beam/shot data where the level carries them.

07000005, upgraded row (the full-level proof, `weapon_matrix_live::upgraded_runner_checks_the_new_behaviours_on_07000005` **green**):
0x2a MGL impact `-` vm1 (old `.`/`I`), gap **2-4 ticks** (contact blast; the 0x2b delay twin's gap
**210** where it fired); 0x2c Sentinel `-` vm1 with **gcam 1** (the camera rides); 0x34 frag vm1
(old `I`); 0x02's mode press lands the silenced twin 0x03 with label `0x0500002b` base → the twin's
own label after. The mode twin, the labels and the guided camera were **unmeasured** before this
task; all three check green through the real input path.

## The remaining red cells — reasons and the one real bug found

- **`I` (1207)** — dominated by the levels whose archives lack the fp skins (the cross-level gap
  100a closed *for the give path*). The upgraded runner resolves them on 07000005 (0x2a/0x2c/0x2d/
  0x34 vm1 where the old runner printed `I`); the full-27 sweep with the upgraded runner is the next
  window's first item — the runner change is committed (56d05be), the sweep is 16 minutes.
- **The `D`-block from 0x2e on (THE REAL BUG this re-run found).** From the AT-600 Scorpion onward
  every draw fails, and the probe (`probe_matrix_draw.rs`) pins it: the previous cell's away switch
  lands on the P2K's **silenced twin 0x03** (twin_skip[02]=1 — the twin step offers it first
  [M `Player_WeaponChange` 0x1a6f80:179-198]); the per-cell reset reverts 0x03's clip to 0; the next
  draw tick starts the **auto-reload, and the 0x03 shell loop hangs in state 6 with clip 0** —
  `rel=6` forever, `gun.clip` never refills although carried=4 — and every later switch is refused
  while the view anim is not idle [M 0x1a6f80:33-47]. A fresh load draws 0x34 in 20 ticks (probe,
  `t20: weapon=34`), so the hang is sequence-state, not the weapon. **This is a port bug candidate
  in the reload shell loop (nf-game gun.rs / the view-script block in scene.rs)** — the decompile
  round it needs (the reload restart gate at `reload_loop_restarted`, and `Player_WeaponFiring`
  0x1a5460's shell refill) did not fit the window; harness mitigations (300-tick unstick + cold
  reload, held-clip repair) are in the runner and documented. Live check below.
- **`R` (35)** — the reload-window artifact from 100d persists (the pulse landing while a fire
  script plays; the settle is 30 ticks); plus 0x2d Sentinel's own reload hang (same family as the
  D-block root — the 5/6/7 machine never returns on the merged load). Live check below.
- **`.` (617)** — (1) records with **no fp model** (`weapon_data+0xDC == 0`; `Player_EquipWeapon`
  0x1a8a58:20 refuses — the watch gadgets 0x4a-0x55 are among them, and they are *carried*, not
  given: the runner should reach them with the **gadget cycle** (`Player_WeaponChange` mode=1,
  bounds 0x4A..0x5E) — listed as next-window work with the beam checks it unlocks); (2) records
  outside the cycle with fp==0 (0x62/0x63/0x6c/0x6d — the game's own refusal, by design).
- **`B`/`W` (134)** — the mode-chain press budget at cell edges (100d's finding stands).

## Verification

- `cargo test --offline --release -p nf-view --test weapon_matrix_live upgraded_runner_checks` —
  **1 passed** (57.6 s, NF_ISO): the give path dresses 0x2a/0x2c/0x34, the 0x2a gap ≤ 90, the 0x2c
  camera rides, the 0x02 mode press lands with a label.
- `nf-extract weapon-matrix` full sweep: 0 panics across 2646 cells (27 × 98).
- The upgraded-runner row on 07000005: 0x02/0x03 clean, 0x2a `-` gap 2-4, 0x2c `-` gcam1, 0x34 vm1.
- The changed file's blast radius is the matrix binary + `weapon_matrix_live` (no shared files):
  **weapon_matrix_live on the final tree: 29 passed / 0 failed** (213 s — the two-level sample
  07000005+0700000c via NF_MATRIX, the test-weapon-row test and the upgraded-runner sister test).
  The full fast suite (all 233 binaries) did not fit the window after the sweep + the probe rounds;
  the next window runs `tools/test/suite.py` on the final tree first.

## Live checks for the main agent (PCSX2)

1. **The silenced-twin reload hang (the port bug)** — any level: give the P2K, mode-button to the
   silenced 0x03, empty its clip (hold fire), then switch away and back: does the real game's 0x03
   shell loop refill and return to idle? In the port the state-6 loop sticks with clip 0 while
   carried > 0 (`gdbwatch.py` on the reload state byte and `reload_ticks`; the view-script block's
   +0xA4 replay gate is the suspect).
2. **The Sentinel reload** — give 0x2c, fire, reload: the port's 5/6/7 machine never returns on the
   merged load (R). Watch the +0xA8/+0xA4/+0xAC script queue in PCSX2.
3. **The mode chain's twin label** — with the P2K silenced twin equipped, the HUD's mode label: the
   port resolves record +0x30 + mode·4 (`HUD_UpdateAmmoPane` 0x19a7f0) — confirm the label text on
   screen matches the disc's language bank entry.
4. **nf-view spots** (after the next window's full sweep): `--level 07000005 --give 0x2a` (the MGL
   impact blast), `--level 07000005 --give 0x2c --frame 40` (the guided-camera view), `--level
   07000005 --give 0x34` (the frag in hand — the old runner's `I`).



## The upgraded runner's full sweep (appended 11:59 UTC, same session)

The upgraded runner (real give path + per-cell reset + the M/G/X checks) swept all 27 levels × 98
ids, **0 panics**. Red counts: **D 1278, B 234, R 226, M 152, F 22, W 9** — the red mass is ONE bug:
the silenced-twin reload hang (above) now reaches the draw because the give no longer stops the cell
first. Where the old runner printed `.`/`I` and quit, the upgraded cell draws — and hangs the machine
for the rest of the level's row. Per-weapon signals the new checks add:

- **0x2a MGL impact: 19/27 clean** — the contact blast gap measured green where the cell ran (7 D
  are hang victims, 1 R); the delay twin 0x2b shows the fused behaviour (19 B are the cell-edge cost
  of the 230-tick blast watch + the hang, not blast-timing wrongs — no X marks fired).
- **0x2c Sentinel 'Guided': the camera rode on the 4 clean cells** (mark `-` requires gcam); 10 R +
  8 D are the reload hang family.
- **`M` 152** — mode presses refused while the inherited machine hangs; they clear with the reload
  fix (the 07000005 upgraded row's mode checks are green where the machine was fresh).
- **0x02 P2K: 19/27 clean** through the real give path (6 F are the manor-intro hold levels, 100d's
  finding).

```
marks       ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
07000001    .  F  D  D  D  -  B  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000002    .  F  D  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000003    .  -  -  -  B  -  B  -  B  -  B  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000004    .  F  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000005    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  B  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000006    .  -  -  -  B  -  B  W  W  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000007    .  R  -  -  B  R  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000008    .  R  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000009    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  -  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  .  D  D  D  .  .  D  D  .  D  .  D  D  D  D  D  .  D  .  D  D  .  .  n  n  n  n  D  D  D  D  D  
0700000a    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  R  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  .  D  D  D  .  .  D  D  .  D  .  D  D  D  D  D  .  D  .  D  D  .  .  n  n  n  n  D  D  D  D  D  
0700000b    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  W  D  D  D  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  .  D  D  D  .  .  D  D  .  D  .  D  D  D  D  D  .  D  .  D  D  .  .  n  n  n  n  D  D  D  D  D  
0700000c    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  .  D  M  B  M  B  M  B  M  B  M  B  -  B  R  R  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  .  D  D  D  D  n  n  n  n  D  D  D  D  D  
0700000d    .  F  D  -  B  -  B  -  W  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  .  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  .  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000011    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000012    .  -  -  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000013    .  F  -  -  W  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000014    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000015    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000016    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  W  D  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
0700001b    .  F  D  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000041    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  -  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000043    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  -  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000046    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  R  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000048    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  W  D  D  D  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
07000049    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  -  R  R  R  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
0700004a    .  -  -  -  R  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  W  D  D  D  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  
0700004b    .  -  -  -  B  -  B  -  B  -  B  -  -  -  B  -  -  -  B  -  B  -  B  D  -  R  B  R  B  M  R  M  R  M  R  M  R  M  R  M  R  -  B  W  D  D  D  R  D  D  M  -  -  -  M  F  n  D  D  D  D  D  D  D  D  D  D  D  D  D  .  .  D  D  .  D  .  D  D  D  .  .  D  D  D  D  D  D  D  n  n  n  n  D  D  D  D  D  

red by mark: B:234 D:1278 F:22 M:152 R:226 W:9
```

The old-runner table above stays the methodology-stable comparison; the next window's reload fix
unlocks this table's real signal (the D/M/R mass collapses, the M/G/X checks measure every cell).

## What's left

- The silenced-twin reload-hang fix (the decompile round + an NF_ISO test that fails first) — it
  holds most of the upgraded table's D/M/R mass.
- The 0x03 shell-loop hang fix (the decompile round + an NF_ISO test that fails first).
- The gadget-cycle fallback for the watch gadgets (their fire/beam cells are `.` today).
- The fast suite on the final tree.
