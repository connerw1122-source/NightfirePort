# scorpion-modes-12 — the AT-600 Scorpion's heat-seeker mode

Task 676 · branch `s/scorpion-modes-12` · 2026-10-16

## What's done

The Scorpion's fire-mode chain is **verified end-to-end on 07000012 through the real input path**
(`Sim::step_bond` only), and one fidelity gap against the decompile is fixed. The mode machinery
itself was already on master (the merged fix-weapon-modes / fire-modes3 work: `fire_mode_switch`
[M `Player_Weapon` 0x1a24c8:246-260] and the homing block [M `Bullet_homing` 0x12f358]); what this
task adds is the disc ground truth, the playtest-level test, and the LOS-trace correction.

### The disc record pins [L, from the ISO's parsed weapon_data]

| record | name label | twin +0x05 | +0x6C | +0x20 speed | +0x1C range | +0x90 ammo | +0x92 clip |
|---|---|---|---|---|---|---|---|
| 0x2E 'Heatseeker' | 0x05000048 | +1 | **0x0322200** (0x20000 homing, 0x200 quarter-speed, 0x2000, 0x10000) | 0.25 | 500 | 0x0e | 4 |
| 0x2F 'Unguided' | 0x05000048 | −1 | 0x01a200 (no homing bit) | 0.3333 | 500 | 0x0e | 4 |

- The ○ switch is the TWIN swap (both records' mode count +0x2E is 1 — the mode byte never cycles
  on this pair) [M 0x1a24c8's twin arm].
- **The HUD mode label is the same string in both modes**: both records carry name 0x05000048
  [L] — the launcher is named identically while either mode is held. The HUD reads the held
  record's label (panel.rs `wd.name_label(id)`), so the value the HUD reads is stable across the
  swap; pinned as the disc has it.
- The 0x200 quarter-speed bit explains the heat-seeker's 0.0625 u/tick flight (0.25 × 0.25
  [M `Bullet_init` 0x12c5a0:188-190]) — a slow, curving rocket, exactly the playtest's picture.

### The homing behaviour [M `Bullet_homing` 0x12f358, called from `Bullet_update` 0x12e7b0:483]

- **The gate**: only records whose `+0x6C` carries 0x20000.
- **Acquisition**: the nearest heat-seekable object [M `Bullet_heatseekable` 0x12f248] within
  1000 units of the round, inside the 0.66-dot (≈49°) cone of the round's facing, with a clear
  line of sight; the owner (`BU+0x30`) is skipped as a candidate; a dead target (`+0xfe & 1`)
  drops the lock and the scan re-runs.
- **The turn**: `dir += clamp((to_target − dir)·0.0333, ±0.01)` per component, then normalised —
  the measured curve closes a ~24° off-axis launch in a few dozen ticks (the test logs it).
- **Fixed here**: the homing LOS ran the port's flight `segment`; the game's homing LOS is its
  ONE bit-0x8 DROP trace (`Collide_LineOfSight(…, 8)` [M 0x12f358:92]) — pass-flagged hits
  (foliage) must not block the lock. The port now uses `segment_bullet` (the trace bullet.rs's
  comment already assigned to the homing LOS).

### The test (`crates/nf-view/tests/scorpion_modes_live.rs`, one 07000012 load)

1. **The record pins** — the table above, asserted against the ISO parse.
2. **The mode switch** — `Input::mode_switch` (the pad's ○) on the Unguided record requests 0x2E
   [M 0x1a24c8:246-260], the draw completes onto the Heatseeker, a second press toggles back.
3. **The heat-seeker** — fired off-axis (auto-aim off, the test measures the homing, not the
   assist) at an enabled guard: the round locks the guard (`BU+0x40`), its direction curves
   toward him, and the guard takes the hit.
4. **The unguided twin** — the same off-axis shot: the direction never changes, the round never
   locks, the guard is untouched.
5. **The never-locks-Bond sister** — with no seekable object alive, no round ever holds a target
   [M 0x12f3b4's scan finds nothing; the owner is not a candidate]. The candidates are the
   port's live guards, so Bond (the owner) can never be the target by construction; the scan's
   `drawn` gate maps the game's view-draw byte `obj+0xFC` [M `View_AddObjects` 0x1e62d8 sets 2
   on view entry], not an enable flag.

Playtest 1012-1723 ("the scorpion launcher heat seeker mode doesn't work") **does not reproduce**
on this branch: the switch, the mode pair and the homing all behave on 07000012. The pre-fix
master behaviour the playtest saw predates the fix-weapon-modes merge.

## What I ran

- `cargo test --offline --release -p nf-view --test scorpion_modes_live` — green.
- `cargo test --offline --release -p nf-game --lib` — 237 passed (the homing change's crate).
- `cargo test --offline --release -p nf-view --test fix_weapon_fire_modes_live --test
  fix_weapon_modes_live --test fire_modes3_live` — 5+5+6 passed (the weapon-mode regression set).
- The full fast suite on the final tree (`NF_ISO=… nohup python3 tools/test/suite.py`) — see the
  total line below.

## Live checks for the main agent

1. **The HUD mode label** [L]: hold the Scorpion, press ○ — the HUD name should read the same
   string in both modes (both records carry 0x05000048); screenshot both modes to confirm the
   game shows no separate "HEATSEEKER/UNGUIDED" line the port is missing.
2. **The heat-seeker turn rate** [L]: fire a heat-seeker ~25° off a standing guard and ride it in
   PCSX2 — the round should curve in over roughly a second (the port's ±0.01/tick clamp). If the
   real rocket visibly turns faster, gdbwatch `BU+0x00..0x0C` per frame for two ticks to read the
   real per-tick delta (predicted ≤ 0.01 per component [M 0x12f358's 0x3c23d70a clamp]).
3. **The heatseekable classes** [L]: does the heat-seeker lock the copter on 0700000a/0x12's
   flyover? The port's scan currently targets its live guards only (see What's left).

## What's left

- **Drone-fired homing**: the game runs `Bullet_homing` for ANY `+0x6C & 0x20000` round including
  guards' own rockets; the port's drone-fired `Bullet` (crates/nf-game/src/bullet.rs) carries no
  homing/target fields — a guard with a launcher fires dumb rockets. Not needed for the Scorpion
  playtest; a follow-up could share the homing block with the drone bullet path.
- **The heatseekable classes beyond guards** [M 0x12f248]: copter bodies (class 0, the CopterList
  walk), class-3 objects with the `+0x894` counter and class-0x28 objects with `+0xfc` are
  heat-seekable on the disc; the port's scan covers its live drones only.
- **The projectile-matrix realign** [M 0x12f358's tail `Mat_Align2Dir(obj+0x90, …,
  0x2d6210/0x2d6220)`]: the port's rocket facing IS its direction vector (no separate projectile
  matrix), so the third-person mesh's bank/align on the turn is not modelled — a visual matter
  for the main agent's render.

## Suite total

(to be filled from the final-tree run: `tail -1 suite.out`)
