## Your task: `remote-mine-detonate` — the Remote Mine's detonator and throw cadence

Playtests 1014-1725 and 1015-0046: "the detonator, for some reason, you have it set to the watch rather than the
alternate fire of the mine"; "there's a delay between when I click to throw ... and a heavy delay between me being
able to throw multiple". Port from the game:
- how detonation is triggered while the mine weapon is held (its alternate-fire/mode action vs a separate
  detonator item) [M Player_Weapon / the mine's weapon record +0x6C/+0x70, the detonate arm in `Bullet_update`];
- the throw → release frame and the re-throw interval [M the fire script, `Player_WeaponFiring` 0x1a5460].

Measure the frames and table them. Headless tests: throw 3 mines at the game's cadence, then detonate with the alt
fire and all three explode. Sister: firing while no mine is out does nothing.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/remote-mine-detonate`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T08:31Z

---
## Report (done, 2026-10-16T10:20Z)

Closed by the driver: the report is on `s/remote-mine-detonate` (docs/reports/remote-mine-detonate.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T10:22Z)

# remote-mine-detonate (679) — the Remote Mine's detonator and throw cadence

**Status: done** — the cadence measured + tabled, its real delay fixed, the three-mine
detonation flow green end to end. Branch `s/remote-mine-detonate` from origin/master (27e5d48a).

## What was already on master (this task verified it, no rework)

The detonator halves of the playtest are done and tested on master: the twin switch
(`Player_Weapon` 0x1a24c8's mode press — the mine 0x37's +0x05 twin = +1 → the detonator 0x38,
whose −1 twin returns; the shared ammo-22 slot), the detonate arm (`Player_WeaponInitBullet`
0x1a4bf0's `+0x68 & 0x20000`: the walk marks EVERY player-owned same-base bullet
`BU+0xFC = −10.0` [M disasm 0x1a4c54-0x1a4c88], `Bullet_DoTrails` 0x12d890:268-285 detonates at
< 0, `Player_WeaponHasAmmo(base)` keeps the hand [M 0x1a4cb8] or GetBestWeapon when dry), the
stick, and the cycle exclusion (+0x04 = 0 [L]). `fix_remote_mine_play.rs` covers them; its
`the_mode_press_detonates_the_placed_mine_and_returns_to_the_mine` passes.

## The cadence: measured and tabled (the task's deliverable)

Through the real input path (`nf_view::load_weapons` + the panel give + `Sim::step_bond`), the
records off the disc table ([L] ee_start_05.bin records; [M] the anchors):

| leg | frames | anchor |
|---|---|---|
| press → shot | 0 (the same tick) | `Player_WeaponFiring` 0x1a5460:618-660 — the press arms `+0x92C = 1.0`, the −FRAME_RATE_MUL and the `≤ 0` shot gate land in one call |
| shot → spawn | 17 | the fire script's release at the record's `+0x44` = 0x1a = 26 script frames [L] at the script clock (`Player_SetWeaponAnimObj` case 9 0x1a2e38:416-420) — the grenade family 0x35/0x36 carries the same +0x44 |
| shot → next shot | 90 | the 0xb/0xc chain (below), NOT the raw `+0x40` |
| spawn → next spawn | 90 | the sum |

The 0xb/0xc chain (the mine's `+0x6C` carries 0x400 [L 0x00008c08]): the fire script ends → the
`+0xB4` fire-last script (0x06000729, 34 frames) → at its stop **`Player_ReloadAmmoType(player,
1)` refills the clip BEFORE the draw** [M 0x1a2e38 case 0xc :521-523] → the draw at 1.25
[M case 3] → idle; the next press fires at once. The PCSX2 frag measurement in the port's own
note (the same chain): the next throw ~+125 with taps. The raw `+0x40 = 450.0` refire window
[L] rides the chain's refire zero [M case 4's end] — the live check L-2 pins the game's
`+0x92C` through two throws.

## The fix

- `crates/nf-game/src/gun.rs` — `Gun::reload_one` (`Player_ReloadAmmoType(_, 1)`'s single-round
  form [M :523]).
- `crates/nf-view/src/scene.rs` — the 0xb/0xc arm ran the draw WITHOUT the refill: the clip sat
  0 through the draw and the next press hit an empty clip and did nothing (the playtest's
  "heavy delay between me being able to throw multiple" — the trace showed the clip 0 for ~90
  ticks after the spawn). The arm now refills before the draw, exactly the game's order.

## Verification

- `cargo test --release -p nf-view --test remote_mine_detonate` (NF_ISO): **3/3** —
  - `the_throw_release_frame_and_the_re_throw_interval_match_the_game` (the table, end to end);
  - `firing_the_detonator_with_no_mine_out_does_nothing` (the sister: no blast, no projectile
    [M the +0x20000 arm returns before Bullet_init], the hand returns to the mine [M 0x1a4c88-
    0x1a4cbc]);
  - `three_mines_at_the_games_cadence_then_one_detonation_explodes_all_three` — three placed
    mines, ONE detonator fire, exactly three blasts (the walk has no break [M 0x1a4c88]), the
    mines gone, the hand back on the mine. (The Sim's explosion list is per-frame — the test
    accumulates the blasts as they fire.)
- The touched crates on the final tree: `nf-game` lib 237/237; `fix_remote_mine_play` 3/3 and
  `fix_weapon_fire_modes_live` 3/3 and `grenade_live` 2/2 (nf-game) — the mine family's
  neighbours untouched.
- **The fast runner** (tools/test/suite.py, the release tree at the branch point — the run
  overlapped this task's edits): `1762 passed, 7 failed, 72 ignored (56 min)`. The 7: the 3
  `disc_golden` slim-ISO fails (allowed), `fix_0c_pylon_live` ×2 + `probe_pylon_render` (the
  documented pylon Vulkan-ICD fails), and `walkreal_probe13` CRASH (exit -9 — a probe binary
  killed, not an assertion).

## What's left

- The game-side tick count for the +0x44 release (L-1) and the +0x92C window through two throws
  (L-2) — the live checks below; the port's numbers are pinned and the anchors are cited.
- The final-tree fast-runner re-run: the sandbox window fit only the branch-point run above
  plus the touched-crate runs on the final tree; the next session can re-run the 56-minute
  suite on the merged tree.

## Live checks for the main agent

1. **L-1 (the release frame):** gdbwatch the mine's `Bullet_init` call through one throw — the
   tick delta from the fire press to `Bullet_init` (the port: 17). Expected: 26 script frames at
   the game's script clock — which real-time tick count does PCSX2 show (17 at 1.5×, 26 at 1.0×)?
2. **L-2 (the +0x92C window):** gdbwatch `BLData+0x92C` through TWO mine throws. Expected per
   this task's reading: the throw arms 450.0 [L +0x40], the 0xb/0xc chain runs, and the window
   is cancelled (the case-4 end's refire zero [M 0x1a2e38 case 4]) — confirm the value the game
   holds at the second press (0 = the chain cancels it; 450-x = the game really waits 7.5 s).
3. **L-3 (the multi-mine boom):** place 3 mines (the give 0x37,0x38, throw ×3), one detonator
   fire, watch `Bullet_DoTrails`' explode arm per mine — three `Explode_Create` calls with
   `record+0x08/+0x0C` = radius 8 / damage 100 [L record 0x37], or one blast + a chain reaction?
