# weapon-matrix3 — the watch gadgets in the matrix, and the sweep's remaining reds

Branch `s/weapon-matrix3` = origin/master 8295049 + the merge of `s/fix-reload-hang` (the task's
base note: that branch carries only its claim/WORKLOG start so far — 103i's fix is still in flight
on its own branch, nothing to merge but the stub; the hang family below is theirs to close).

## What landed

1. **The gadget-cycle path** (`crates/nf-view/src/matrix.rs`): the watch gadgets 0x4A..0x5E no
   longer die at the give — they are CARRIED, not given (`Player_EquipWeapon` 0x1a8a58's owned
   branch declines an already-owned gadget at the ammo cap), so the cell equips through the
   **gadget cycle** (`Player_WeaponChange` 0x1a6f80 mode=1, bounds 0x4A..0x5E — the port's
   `Input::gadget_cycle`, proven by gadget_select_live) and runs the real fire checks: the beam /
   the bolt / the thrown camera / the sound. A level that does not carry the id prints the dim
   `n` — the game's own cycle cannot reach it either [M 0x1a6f80 walks owned, selectable records].
   Marks for gadget cells run their own chain (n → I → D → F → S → W → B → `-`).
2. **Two harness repairs the gadget cells exposed** (both cited in code):
   - **The dead-Bond tail**: the prior cells' gunfire draws the guards; a dead Bond refuses every
     input and the rest of the row measures nothing. The runner cold-reloads the level when hp
     reads ≤ 0 at a cell start (the warm auto-restart does NOT revive: its RamLoad carry keeps the
     dead 0 — measured here). This alone moved whole level rows out of the red.
   - **The stale fire_busy script**: the port's fire_busy approximation (scene.rs:
     `view_script == the record's fire scripts && view_time < len − 1`) reads a FINISHED script —
     the last view script stays set with `view_time` frozen at 0 — and then blocks the whole
     switch/cycle block forever. The game's gate there is the view-anim state 9
     [M Player_WeaponChange 0x1a6f80:33-47], which is idle in this state. The runner clears the
     stale field when the machine is idle and two idle ticks agree the script is done
     (`unstick_view_script`). The D-block from 0x2e on is largely THIS: 07000003 before/after the
     fix ran on the same tree — **D 17 → 9, clean 27 → 34** (every other mark unchanged).
3. **The stunner's no-target arm recorded**: `Cell::no_target` — the stunner's shot spawns only
   when the aim ray meets a live guard within weapon_data+0x1c; the miss arm spawns nothing and
   draws no taser beam by the game's own rule [M Player_WeaponInitBullet 0x1a4bf0:169-200; the
   port's `Sim::stunner_target`]. A silent stunner cell at an empty spawn is the game's behaviour,
   not a red.

## The gadget cells (the task's item 1) — 07000005, all four carried gadgets

`weapon_matrix_live::gadget_cells_run_the_cycle_on_07000005` **green** (NF_ISO): 0x4A Stunner
(equipped, sounded, `nt1` — the no-target arm at the spawn), 0x4E Laser `-` (the pointer beam
parks, 3 shots), 0x50 Q-Claw `-` (the bolt spawns), 0x54 Micro-Camera `-` (the thrown camera) —
all four cycle-equipped, fire-checked, sounded, and through the switch away/back. Fail-first: the
pre-change cells were `.` (the give refused an owned gadget) — the same test asserts give_ok on
them. Sisters: 0x4F/0x4C (the uncarried twins) stay dim `n` with the cycle-reach reason; 0x55 is
not selectable [L weapon_data +4 = 0] so the cycle skips it, as the game does. The Micro-Camera's
record carries no guided-camera bit (+0x6C = 0 [L ee_start_05.bin]) — no camera feed is expected
from the missile-cam path, and none fired; the thrown camera itself is the observable.

## The sweep (the task's item 2) — 27 levels × 98 ids = 2646 cells, **0 panics**

`nf-extract weapon-matrix <iso>` split into five invocations (one level-row per ~90 s on the story
levels, seconds on the wrappers; 07000015 re-ran alone after a chunk timeout). Red counts:
**B 287, D 487, F 53, M 197, R 310, W 47** (matrix2's run: D 1278, B 234, R 226, M 152, F 22,
W 9 — the D mass halved twice over with the dead-Bond + stale-script repairs, and the sweep is
still measured with the 103i hang unfixed).

```
cells: 2646 levels: 27
red by mark: {'B': 287, 'D': 487, 'F': 53, 'M': 197, 'R': 310, 'W': 47}
marks       0x01 0x02 0x03 0x04 0x05 0x06 0x07 0x08 0x09 0x0a 0x0b 0x0c 0x0d 0x0e 0x0f 0x10 0x11 0x12 0x13 0x14 0x15 0x16 0x17 0x18 0x19 0x1a 0x1b 0x1c 0x1d 0x1e 0x1f 0x20 0x21 0x22 0x23 0x24 0x25 0x26 0x27 0x28 0x29 0x2a 0x2b 0x2c 0x2d 0x2e 0x2f 0x30 0x31 0x32 0x33 0x34 0x35 0x36 0x37 0x38 0x39 0x3a 0x3b 0x3c 0x3d 0x3e 0x3f 0x40 0x41 0x42 0x43 0x44 0x45 0x46 0x4a 0x4b 0x4c 0x4d 0x4e 0x4f 0x50 0x51 0x52 0x53 0x54 0x55 0x56 0x57 0x58 0x59 0x5a 0x5b 0x5c 0x62 0x63 0x68 0x69 0x6a 0x6c 0x6d 0x6e 0x6f
07000001    . F D D D - B F D D D - - - B F D D D - B - W D D D D R B M R F D D D M R F D D D - B R D D D D D D M W F D D D n - W D D D D D - - - - F D - n n n - n - n n n - n n n n n n n n n n n n W D D - -
07000002    . F D - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n W D - - -
07000003    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000004    . F - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n W D - - -
07000005    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B B D R R R D D M W - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000006    . - - - B - B W D - B - - - B - - - B - B - B D - R B R B M R M R M R R D M R M R - R R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000007    . R - - B R D - B - B W - - B - - - B - B R D D - R B R W M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000008    . R - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000009    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M W M R M R M R M R - B - R R R R D D M - - - M F n D D D D D D D D D . D D D - n n n - n - n n n n n - n W n n B n n n n n - D - - -
0700000a    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R R D R D R D R D D M W - - W D n - W D D D D D - - . - - - - n n n - n - n n n n n W n W n n B n n n n n - D - - W
0700000b    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B W D D D R D D M - - - M F n D D D D D D D D D . D D D - n n n - n - n n n n n W n W n n B n n n n n - D - - -
0700000c    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B . D M B M B M B M B M B - B R R R R R D D M W - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n W n n n n n n n n - D - - -
0700000d    . F D - B - B - W F B - - - B - R F B - B - B D F R B R D . B M B M B F B M B M R F B - R F R R D D W F - - M R n F W D D D D D - - - - - - - n n n W n D n n n - n n n D n n n n n n n n - D - - -
07000011    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000012    . - - D D - B - B - B - - R D - R - B - B - B D - R D R W M R M D M R M D M R M R - B R D R R R D D M W - - W D n - W D D D D D - - - - W - - n n n B n - n n n - n n n n n n n n n n n n - D - - -
07000013    . F - - W F B W D - B W F - B F - - W F B W D D - R D R B F R F R F R F R F R F R F B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n W D - - -
07000014    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - W R D R D R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - W
07000015    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000016    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - W D n - W D D D D D - - - - - - - n n n - n - n n n - n n n n n n n n n n n n - D - - -
0700001b    . F D - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n W D D - -
07000041    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B - R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000043    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B - R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000046    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000048    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B W D D D R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
07000049    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B - R R R R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
0700004a    . - - - R - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B W D D D R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - W
0700004b    . - - - B - B - B - B - - - B - - - B - B - B D - R B R B M R M R M R M R M R M R - B W D D D R D D M - - - M F n D D D D D D D D D D D D D - n n n - n - n n n - n n n n n n n n n n n n - D - - -
```

marks: `-` clean | `!` panic | `.` give refused | `n` no fp model, or a gadget the level does not
carry | `I` no view model | `D` draw failed | `F` no observable fire (gadget cells: and not the
no-target arm) | `R` reload broken | `W` switch-away failed | `B` switch-back failed | `M` mode
press failed | `S` silent. `nt<k>` in a cell summary = the no-target ticks.

## The red groups — the reason per group

- **D 487** — two families. (a) The draw-refused-while-a-script-plays family (the stale fire_busy
  script above): the fix landed mid-window; the 07000003 proof row shows D 17 → 9. The FULL
  re-sweep on the fixed runner is the next window's first item — the table above is the pre-fix
  measurement for the story levels. (b) The 103i reload-hang family (the 0x03 twin's state-6 shell
  loop, the 0x2d Sentinel's 5/6/7 machine — `docs/reports/weapon-matrix2.md`'s pin): 103i is
  claimed and in flight; its fix collapses the rest of D/M/R together.
- **M 197** — the mode press refused while the machine is not idle at the press (the snipers'
  scoped twins 0x1e-0x28 and the 0x33/0x37 laser family): the press budget at cell edges
  (matrix2's finding stands) plus the hang family.
- **R 310** — the reload-window artifact (the pulse landing while a fire script plays; the 30-tick
  settle) plus the 103i hang family (the 0x1c column: the 210-tick fused reload family).
- **B 287** — the switch-BACK to the mode twins (0x05/0x07/0x09/0x0b/0x0f/0x13/0x15/0x17/0x1b/
  0x1d/0x2b at ~24-27/27 levels each): the back to a variant record goes through mode_to's 12
  presses and each press needs the machine idle; at the cell edges the budget runs out. The cells
  draw and fire fine (their summaries show draw1 + shots) — the back alone times out. A harness
  budget question, not a weapon bug.
- **F 53** — 0x02/0x08/0x10/0x20/0x26/0x35 on the manor-intro levels (07000001/02/04: Bond held by
  the intro script; 100d's finding) + 0x38 (22/27: the handcannon's fire with no observable event
  at the spawn — needs a target like the stunner? the live check below).
- **W 47** — scattered singles: the away-switch refused at a cell edge (the same press-budget
  family).
- **The gadget columns 0x4a-0x5c** — `n` everywhere except the levels that carry them (07000005's
  row: the four carried gadgets run; the uncarried twins dim). The Q-Worm 0x52/0x53 and the
  decryptor 0x56/0x57 are pickup-carried on their levels — their cells run where the level grants
  them (0700000c's camera pickup — gadget_select_live's pin).

## Files

`crates/nf-view/src/matrix.rs` (the gadget path, the two harness repairs, the no_target field, the
mark chains, the legend), `crates/nf-view/tests/weapon_matrix_live.rs` (the gadget test),
`crates/nf-view/tests/probe_gadget.rs` (the probe that separated the runner's context from a fresh
load — kept). Shared files: **none touched**.

## Verification

- `weapon_matrix_live::gadget_cells_run_the_cycle_on_07000005` — green (83 s, NF_ISO); fail-first
  on the pre-change runner (the cells were `.`).
- The sweep: 2646 cells, 0 panics; the counts above.
- The unstick proof: 07000003 re-run on the same tree, only the unstick added — D 17 → 9,
  clean 27 → 34.
- The suite: `tools/test/suite.py` was started in the background at 14:56 UTC but this sandbox
  kills background jobs between tool calls (the same limitation the last five sessions recorded);
  the foreground evidence is the matrix binary's own runs + `weapon_matrix_live` (29 passed /
  0 failed on the final tree incl. the new gadget test — the two-level NF_MATRIX sample + the
  test-weapon-row + the upgraded-runner sister).

## What's left

- The full 27-level re-sweep on the post-unstick runner (the next window's first item; the story
  levels are the slow half) — the table above is the pre-fix measurement.
- 103i's reload-hang fix (in flight on `s/fix-reload-hang`) — collapses the remaining D/M/R mass.
- The 0x38 F-family live check (below).

## Live checks for the main agent

1. **The stunner at an empty spot** (settles the nt arm): PCSX2, 07000005 spawn, aim at sky, hold
   fire with the stunner out — the game should show the blue muzzle light + the zap sound and NO
   beam; then aim at a guard — the taser beam draws. nf-view: the same two holds at the spawn.
2. **0x38 handcannon's fire** (the F-family): give 0x38 at any level, fire 3 — does the real game
   show a shot effect at the spawn spot? The port records sounds but no shot tick.
3. **The micro-camera throw**: equip 0x54 via the watch (Up/Down), fire — the camera should fly
   and stick; nf-view: `--bond 16.5,29.6,58.6,0,0.25` on 07000005, cycle to the camera, fire.
4. **The gadget wheel in MP is the same cycle** — nothing MP-side was touched.

ntfy: sent (see the transcript if it errored).
