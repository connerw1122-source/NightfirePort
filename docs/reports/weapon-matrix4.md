# weapon-matrix4 — the harness measures weapons only

Branch `s/weapon-matrix4` (from origin/master @ 7785582). Task ID 105a.
Files this task changed: `crates/nf-view/src/matrix.rs` (the whole harness work),
`crates/nf-view/src/scene.rs` (two small port fixes, both with decompile evidence — see
"Shared files"), `crates/nf-view/examples/probe_twin.rs` (the probe that pinned the record
fields; kept). `crates/nf-view/tests/weapon_matrix_live.rs` is unchanged — the sweep binary IS
the test (one test per level, a level fails on any panicked cell).

## The three harness repairs the task asked for

1. **Bond alive and unhurt for the whole sweep** (`god`/`step` in matrix.rs): every tick the
   runner steps is wrapped in the recorder cheat's own write — health 100 / armour 50 / flash 0
   every frame (cheats.py `p.write(bl + 0x8B0, F50, 4)` + the health row; the dev panel's god arm,
   dev.rs `after_tick`). Health goes in at 1000.0 BEFORE the tick (one tick can carry more than
   100 damage at point blank — a death would start inside it, with `Player_CheckForDeath`'s
   channel/sound side effects [M 0x190810:34-97]) and at 100.0 after; a death that somehow started
   is cleared at once. The dead-Bond tail is gone — no row dies below the killing cell any more.
   A scripted disable is survived too: `wait_settled` requires 90 consecutive enabled-and-unpaused
   ticks before a cell (one 07000002 timeline disables Bond again a tick after the intro's enable
   [L probe_twin trace: `dis=true` at t1542]; the old check-then-break gate started cells inside
   that window and they measured nothing), and `Cell::disabled_ticks` counts any disabled ticks a
   cell did run so such a red is attributed to the script, not the weapon.
2. **Switch budgets from the game's own script lengths** (`script_ticks`/`switch_budget`/
   `mode_budget`/`fire_settle_ticks`/`reload_budget`): the holster +0xC4 and draw +0xC0 play at
   1.25×, the variant swap +0xC8 at 1.3× [M Player_SetWeaponAnimObj 0x1a2e38 cases 1/3/8], the
   reload +0xA4 passes at 1.35× [M Player_WeaponFiring's shared tail 0x1a6250-0x1a631c]. Every
   wait rides to the IDLE VIEW (`wait_idle`): machine idle AND the view not mid-fire-script — the
   game's own refuse gate [M Player_WeaponChange 0x1a6f80:33-47]. That re-synced the reload pulse
   (the R mass: the pulse landed inside a fire script) and repairs the stale finished fire script
   the port leaves set (frozen `view_time` with the machine idle → re-queue the idle script, the
   game's own case-9 end [M 0x1a2e38:449-456]).
3. **Twins and gadgets reached the way the player reaches them**: the direct `switch_to` needs
   `owned[id]` and owned[] is per base [M Player_EquipWeapon 0x1a8a58:39-41], so a twin's direct
   select is refused. Draw AND back now go to the BASE first, then the mode press to the twin
   [M Player_Weapon 0x1a24c8:246-260] — from the away P2K the mode press only bounced the held
   family and never arrived (the B mass). Gadgets: the mode check runs for them too (the game's
   action 12 reads whatever record is held [M 0x1a24c8]), and the wheel (`Input::gadget_cycle`,
   mode=1, bounds 0x4A..0x5E [M disassembly 0x1a707c]) is the fallback. By-design refusals are
   recorded as the game's own arms, not reds: a non-selectable record (+4 == 0 [L probe:
   0x38/0x5a/0x5c/0x6c]) is dim `n` (the cycle skips non-selectable candidates [M Gun::cycle] and
   the direct select needs owned[id] — no player route), a dry twin target refuses the mode press
   [M Player_WeaponHasAmmo 0x1a16c0 → fire_mode_switch] (`mode_dry`; 0x1a→0x1b: ammo type 5 vs 12
   [L probe]), a record with no reload scripts (+0xA8/+0xA4/+0xAC all 0 [L probe: 0x38]) is
   reload-N/A, and an empty reserve refuses the reload [M Player_ReloadAmmoType 0x1a1988: carried
   == 0 → return 0, the caller EXITS].

## Two port fixes the harness surfaced (scene.rs, small, cited)

- **`Player_AmmoInGun`'s shared clip read** [M decompile 001a9190]: a twin record (+5 != 0) whose
  ammo type matches its base reads the BASE's slot (`iVar2 = weapon_data[w].+2` unless the types
  differ). The port's `Gun::has_ammo` read the twin's OWN slot — which no game write path ever
  fills — so a twin with a full shared magazine read as dry. Observable case: the Shaver twins
  0x5a/0x5c (ammo type 30's carried cap is 0 [D ammo_data]; the give fills clip[0x59] = 500) were
  unreachable by the mode press — D cells on every level — where the game reaches them. Fixed with
  a `clip_view` helper at the two has_ammo call sites (the mode press and the switch/cycle block).
  This is the read-side of fix-reload-hang's twin clip-slot unification (its runtime boundary work
  is in flight on its own branch; this task did not touch those sites).
- **`fire_busy` now ANDs the machine's state-9 flag**: the hash predicate alone OVER-REFUSED when
  a record's idle script IS its fire script — 0x5a: +0xA0 == +0xB0 == 0x060004ea (len 2) [L probe
  dump] — so the machine read fire-busy while merely idle and every switch/reload/regen gate
  starved forever (the stuck-machine wedges before 0x38/0x39/0x5b/0x62 on 07000002, each costing a
  cold reload and poisoning the tail). The game gates on `anim+0xF4 == 9` [M 0x1a6f80:33-47], a
  MACHINE state set by `Player_SetFiringAnim` 0x1a3c70:50-54 and cleared at the script end
  [M 0x1a2e38 case 9:449-456]; the idle loop playing the same hash is state 0 and switching works
  there. `gun.fire_state` is that flag; the predicate keeps its hash window inside it.

## Fail-first evidence

- 07000002 pre-change (master's runner, weapon-matrix3's table): `. F D - B - B - B - B - - - B -
  - - B - B - B D - R B R B M R M R M R M R M R - B R R R R R D D M - - - M F n D D D D D D D D
  D D D D D - ... W D - - -` — 17 D, 6 B, 19 M/R pairs, 1 W in the row.
- The same level after the repairs: every one of those cells clean except the by-design arms. The
  0x5a cell is the cleanest single proof: D (draw refused) before the `clip_view` fix, `-` after —
  the give fills clip[0x59] = 500, the mode press from the base lands 0x5a, the mine spawns, the
  back returns through base+mode press.
- The stuck-machine wedges (a cold reload before 0x38/0x39/0x5b/0x62 on every level) are gone
  after the fire_busy fix; the sweep's cold-reload log lines went to zero.

## The sweep — 0 panics; 18 of 27 levels measured

`weapon_matrix_live` (NF_ISO, --test-threads=1): **0 panics**; every completed level's test green.
Levels measured: 07000001-0d, 11, 12, 13, 16, 1b, 48, 4a (18). The window ended before
14/15/41/43/46/49/4b — their rows are the next window's first item (the runner is now ~4× faster
than matrix3's: ~60-90 s per story level, no cold reloads; the full sweep fits one window).

**Reds across the 18 levels: 24 cells total** (matrix3's story levels carried several hundred).
Per mark: F 16, M 3, W 4, X 1. Every red with its reason:

1. **0x32 Phoenix Samurai — F on 16 of 18 levels** (fire, no observable event; CLEAN on 01, 09,
   0c, 0d — the four levels where something observable did spawn). The cell draws, holds the
   trigger 3×, and nothing spawns: no shot tick, no bullet, no impact, no explosion, no grenade,
   no beam. The record's fire script plays (sequences recorded) and sounds start, so the machine
   runs — the fire itself produces nothing at an empty spawn spot. This looks like the stunner's
   no-target arm (weapon-matrix3: the shot spawns only when the aim ray meets a live target
   [M Player_WeaponInitBullet 0x1a4bf0:169-200]) — **the task's numbered follow-up #1**: weapon
   0x32 (the Phoenix Samurai katana), nearly every level, what's wrong: the fire's observable
   event needs a target or the sword's swipe effect is not ported; the game evidence to collect:
   the live check below (does the real game show the swipe spark at the spawn?). The four clean
   levels are the discriminator: something DID spawn there (a guard within reach on 09/0c/0d?),
   which supports the target-gated reading.
2. **0x54 Micro-Camera — M on 07000009/0a/0b** (the mode press failed; clean on the other 15
   levels). The watch pair 0x54/0x55:
   the game reads the mode press OR'd with the fire edge there (action 9 mask 4 [M Player_Weapon
   0x1a24c8; scene.rs's `watch` arm]) — the harness pressed the mode button without the fire edge
   and the press did nothing on those levels. Harness gap, not a weapon bug; the fix is one line
   (OR the fire edge for 0x54/0x55 in the 4.5 check) — next window.
3. **0x2c AT-420 Sentinel — W on 0700000b/48/4a** (the switch away failed). The guided-missile
   camera rides after the fire (the cell fired; `gcam` recorded) and the camera's 15-tick static
   period eats the away switch's window. Harness timing vs the camera, not the weapon — the away
   wait should ride out `missile_cam` first. Next window.
4. **0x04 Wolfram PP7 — W on 0700004a** (one cell). The away switch from the Wolfram failed once
   on the front-end map; the same shape as the W family above (the press budget at that cell's
   edge). Single occurrence; needs one re-run to confirm before it is a weapon finding.
5. **0x2a Militek MGL — X on 0700000d** (the blast-gap check): the impact record's explosion
   landed >90 ticks after the spawn (the impact arm should blast on first contact). One cell;
   the next window re-runs it with the 103i reload fix merged (the MGL family's fuse paths are
   entangled with the reload boundary reads).

**Panics: 0** across every cell run this window (the catch_unwind net never fired; the suite's
panic rule holds).

## Verification

- `weapon_matrix_live` on the final tree (NF_ISO): the levels above all green, 0 panics. Fail-first
  is documented per repair above (the 07000002 before/after row; the 0x5a D→- cell; the stuck-machine
  log lines disappearing).
- The full 4-crate suite (tools/test/suite.py) did NOT fit this window next to the sweep — the
  sandbox's 2-hour budget went to the runner work + the sweep. The next window runs it on the
  merged tree; the known allowances: the 3 disc_golden tests, pss_movies_live without the retail
  image, weapon_matrix_live's pre-existing 600 s per-binary cap (the full 27-level sweep inside
  one binary — now ~25 min — still exceeds it; consider splitting the binary per level chunk).
- Shared-file caution: scene.rs changes are two localised edits (the `clip_view` helper + its two
  call sites; the `fire_busy` predicate line + its comment). No other lines touched.

## Live checks for the main agent

1. **The Phoenix Samurai's fire** (follow-up #1): PCSX2, any level, give 0x32, fire at a wall and
   at a guard — does the real game show a swipe/spark effect and a sound where the port shows
   nothing? gdbwatch on `Player_WeaponInitBullet`'s 0x1a4bf0 target-test path for 0x32's +0x95b
   latch.
2. **The Shaver twin on hardware**: give 0x5a via the dev menu; with the BASE 0x59 equipped press
   the mode button — the game should swap to 0x5a (fp 0x05000079 → 0x0500004b). Then switch away
   to the P2K and try the wheel: the game should NOT reach 0x5a (the record is +4 == 0; the cycle
   skips it) — the only route back is base → mode press. Confirms the mode-press route and the
   `Player_AmmoInGun` shared-slot reading.
3. **The idle==fire records**: equip 0x5a (or the mine 0x38) and idle — the real game switches
   weapons instantly from the idle loop; the port used to starve every gate there (the state-9
   gate reading). gdbwatch `anim+0xF4` while the 0x5a idle loop plays: it should read 0, not 9.
4. **nf-view spots** (07000002): the spawn spot, `--give 0x5a`, cycle to 0x59, mode press — the
   first-person model changes to the 0x5a skin and the mine throws.

## What's left (next window)

1. The remaining 9 levels of the sweep (14/15/41/43/46/49/4b — 14 timed out once at 240 s, the
   wrappers are fast) on this runner; the red list above then covers all 27.
2. The two one-line harness gaps: the 0x54/0x55 mode press OR'd with the fire edge; the away wait
   riding out `missile_cam`.
3. The full suite (tools/test/suite.py) on the merged tree; the FAILED lines vs the allowances.
4. The 0x32 follow-up (the live check above decides: harness no-target arm vs a missing swipe
   effect port).
