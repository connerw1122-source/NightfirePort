# remote-mine-detonate (679) — the Remote Mine's detonator and throw cadence

**Status: the cadence measured + tabled and its real delay fixed; the multi-mine detonation is
the open item.** Branch `s/remote-mine-detonate` from origin/master (27e5d48a).

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

- `cargo test --release -p nf-view --test remote_mine_detonate` (NF_ISO): 2/3 —
  - `the_throw_release_frame_and_the_re_throw_interval_match_the_game` (the table, end to end);
  - `firing_the_detonator_with_no_mine_out_does_nothing` (the sister: no blast, no projectile
    [M the +0x20000 arm returns before Bullet_init], the hand returns to the mine [M 0x1a4c88-
    0x1a4cbc]);
  - `three_mines_at_the_games_cadence_then_one_detonation_explodes_all_three` — **FAILS at the
    detonation: one detonator fire over THREE placed mines yields 0 blasts** (the single-mine
    detonate on master passes). The 3 throws + the placements assert green; the gap is in the
    mark/detonate path under multiple mines — the next session's first step.
- What ran: `remote_mine_detonate` (above); the fast runner on this tree — see the total line
  below; the touched crates: nf-game (reload_one), nf-view (the 0xc arm).

## What's left

1. **The 3-mine detonation** — one fire must explode all three [M the walk has no break]. The
   single-mine path works; trace the third+ mine's mark (the walk's `g.state < 2` filter, the
   fuse tick under the stacked positions) and turn the third test green.
2. The cadence test's press→spawn leg pins the port's script clock (17 ticks); the game-side
   tick count for the +0x44 release is a live check (below).
3. The fast runner's total line on the final tree (the run was cut off by the sandbox window —
   the partial lines are in `suite.out` at the sandbox repo root).

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
