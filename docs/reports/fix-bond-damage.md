# fix-bond-damage — Bond taking damage: armour, flash, hit direction, pain sounds, hit-push, difficulty factors

Branch `s/fix-bond-damage` (from origin/master eb52fb56, merged origin/master 4ea550b7 mid-task).

## What the port already had (checked, unchanged — each pinned by an earlier merged task)

- **Armour absorbing first, the difficulty factor, the flash write, the health floor**: `PlayerHealth`
  (`crates/nf-game/src/bullet.rs`) already carried `Player_HandlePain` 0x1902c8's armour gate
  (types 1..7 bypass armour, :84-104), the absorb math (:100-104), the <1 → 0 floor and the
  `flash += min(d·21.333, 128)` (:126-130). The per-difficulty `Plr_DMod_*` factor was wired with
  the live [L] 07000005 = 0.6/0.7/1.0 (`progress_live`, `fields_player_live`).
- **The red flash and its fade**: `HUD_UpdateHealthPane` 0x19b9c8's full-screen red glow at alpha
  `BL+0x968` with the −1/tick fade was already drawn (`hud.rs` sprites 20-23, tex 0x0300018D/E;
  fix-hud), and the port fades `health.flash` by 1/tick (`drone.rs`, the pane's fade).
- **The hit-direction indicator**: exists — the four edge-glow sprites driven by `BL+0x967` bits,
  the direction rule of `Player_DealWithObjHit` 0x194458:201-226 ported in `PlayerHealth::pain_from`
  (the both-sides arm included). Pinned by `hud_playtest_live`/`fix_hud_live` (unchanged).
- **The health pane's colour bands / armour ring**: fix-armour-ring's pane (green 0x52a88b /
  yellow 0xc6984e / red 0x9d1912 bands, the 8-segment armour arc) — unchanged, its tests green.
- **Death (death camera, fail flow)**: fully ported earlier (`crates/nf-game/src/death.rs`:
  `Player_CheckForDeath` 0x190810 → the blood pane → ch 0x62 → `Mission_Update` 0x182c60 → the
  0x40000042 menu; `fix_death_restart_live` 8/8 green on the final tree). Nothing differed.

## What differed and is now ported (each cited)

1. **The bullet-hit hurt sound 0x412 was never played** [M `Player_DealWithObjHit`
   0x194458:229-236]: every bullet hit on Bond plays `Sound_Play(100, 0x412)` while the weapon
   record's `+0x70 & 0x180 == 0` (the 0x600 arm is the MP assassin special, not ported — SP is
   always 0x412). [L weapon_data] probe: the guns (0x2a/0x2c/0x2e/0x30/0x33/0x41, +0x70 = 0x2/0x4)
   pass the gate; the grenades (0x35 = 0x2280, 0x37 = 0x18300, 0x3a = 0x80100) and Oddjob's hat
   (0x45 = 0x100) carry 0x80/0x100 and stay silent. Ported in the `BulletEvent::Player` arm
   (`drone.rs`, additive hook).
2. **The 1-in-4 pain grunt was never played or rolled** [M `Player_HandlePain` 0x1902c8:110-119]:
   after the armour/health math the game draws `Rand_Rand(4)` on the SHARED RNG and plays
   `Sound_Play3D(100, −1, −1, 0x88, BL+0x30)` on 0. The draw now sits exactly where the game puts
   it (inside the shared pain body, `PlayerHealth::apply`) so the bit-exact stream moves with it —
   every pain path (bullets, falls, drowning, explosions, the live wire) rolls and plays through
   the callers. The game skips the roll only for pain type 7 with `CheatInfo != 0` (never in SP).
3. **The melee hit-push `BL+0x910` was a documented gap** (docs/fields/bldata.md:158 "gap")
   [M `Player_DealWithObjHit` 0x194458:237-256]: the melee records (u16 `+0x10 & 1` set
   [L 0x63/0x64/0x65/0x66: 0x1; 0x67: 0x10 — clear]) play `Sound_Play3D(100, −1, −1, 0x171, Bond)`
   and write `BL+0x910` — record id +2: 0x65 → −0.1, 0x66 → +0.1, 99 (0x63, the guard kick) →
   +0.1. `Player_Update` decays it ×0.9/tick unconditionally [M disasm 0x1937a4-0x1937d8: the
   `lwc1 0x910/mul.s 0.9/swc1` chain] and adds it to the move vec's Y before `obj+0x50 += move`
   [M disasm 0x193ccc-0x193cf0: `lqc2/sqc2 0x30`, `lwc1 0x34 + lwc1 0x910 + add.s`, `obj+0x50`].
   The port stores it as `PlayerHealth.hit_push`, decays + applies it as the per-tick vertical
   displacement it is (the move vec's Y lane is up — the same lane the jump velocity uses
   [M `Player_HandleJump` 0x1916b0: obj+0xc4 += 0.95]) in `scene.rs` before `Bond::tick_ctx`
   (shared file, additive hook), floored by the existing ground clamp.
4. **Five `ReadTuningVars` damage-factor level groups were missing** [M 0x1d3a80]: the port only
   had 0x7000001-4 and 0x5-8, falling to the ELF boot defaults for 0x7000009-b (0.6/0.8/1.0,
   :97-99), 0xc-d (0.4/0.7/1.2, :126-127+143), 0x11-13+0x4a (0.5/0.6/1.0, :168-173), 0x14-16
   (0.4/0.6/1.0, :180-185) and 0x1b (0.6/0.7/1.0, :190-197) — every mid/late level's difficulty
   scaling was wrong (e.g. 09 at Agent dealt 0.75× instead of 0.6×). Also difficulty 0 reads
   `Plr_DMod_Normal`, not 1.0 [M 0x1902c8:29-48]; the unmapped levels keep the boot defaults
   0.75/1.0/1.25 [L ACTION.ELF 0x30cc50-58, read from work/elf].

## Files touched

- `crates/nf-game/src/bullet.rs` — `PlayerHealth.hit_push` (own field/line), the pain grunt roll
  inside `apply`, the `damage_factor` groups. **Note**: the task named player.rs/hud.rs/scene.rs,
  but the port's BL health struct and pain body live in bullet.rs and the hit dispatch in
  drone.rs — those two had to carry the change instead of player.rs/hud.rs (neither needed a line).
- `crates/nf-game/src/drone.rs` (shared) — additive hooks only: the `BulletEvent::Player` arm's
  0x412/0x171/hit-push lines, the two `pain_generic` call sites' grunt plays, and the new
  `play_pain_grunt` helper.
- `crates/nf-view/src/scene.rs` (shared) — additive hooks only: the hit-push decay+apply before
  `Bond::tick_ctx`, the drown/fall grunt plays.
- `crates/nf-view/tests/fix_bond_damage_live.rs` — new. Call-site signature updates (Rng in,
  grunt out) in `fields_player_live`, `fix_death_restart_live`, `fix_guest_reactions_live`,
  `fix_restart_target_live`, `hud_playtest_live`, `fix_hud_live` (behaviour unchanged).

## Verification

- **Reproduce-before-fix**: `fix_bond_damage_live` fails on the pre-fix tree (the tests do not
  compile against it — no `hit_push`/`damage_factor` groups — and the behavioural gaps are the
  four items above); 6/6 green post-fix on the merged tree:
  - a staged guard bullet (0x2c, the record's 100.0 [L]) on 07000005 at difficulty 1 costs
    exactly 40 health (100 × 0.6), flash 127 end-of-tick (128 written, the pane's −1/tick fade
    [M 0x19b9c8] runs the same tick), sets direction bits, and **starts 0x412**;
  - with armour 50 the same hit empties the armour (first 50 [M 0x1902c8:100-104]) and costs 10;
  - sister: a 0x37 hit (flags & 0x180 = 0x100) lands but starts **no** 0x412;
  - the pain grunt: 24 staged hits grunt some but not all (the deterministic `Rand_Rand(4)`);
  - the melee 0x65 hit (the record's 15.0 unmodded [M 0x139f50:19-21] → 9 at 0.6) starts 0x171
    AND 0x412 (0x65's +0x70 = 0x400), writes `hit_push = −0.1`, decays to −0.09 the next tick;
    sister: 0x67 (flag bit 0 clear [L]) — no 0x171, no push;
  - `damage_factor` matches `ReadTuningVars` for all seven groups + the boot defaults + difficulty
    0/4, and the already-pinned 07000005 values stay.
- The touched suites re-ran green on the merged tree: `fix_death_restart_live` 8/8,
  `fix_restart_target_live` 3/3, `fix_guest_reactions_live` 3/3 (+1 ignored),
  `hud_playtest_live` 6/6, `fields_player_live` 2/2, `fix_hud_live` 5/5, `nf-game` bullet units 6/6.
- Full suite on the final tree: see the total line at the end.

## Not modelled / left

- `Input_RumbleStart(playerIdx, 5, remaining)` [M 0x1902c8:119] — the port has no rumble output.
- The MP arms of 0x194458 (the +0x25/+0x5 assassin one-hit kills, the friendly-fire menu text) and
  `Player_HandlePain`'s MPSettings branch — the port is single-player only.
- `Player_SetHealth` 0x1960b8's `PlrStat_OkToUpdate` gate (the stats-recording window) is not
  ported; the port writes health directly (pre-existing, unchanged here).
- The melee push's ±0.1 lands as a vertical displacement, exactly the game's move-vec-Y lane; in
  play it reads as a small jolt, and the floor clamp absorbs the −0.1 (punch) case, as in the game.

## Live checks for the main agent

PCSX2 (07000005, difficulty 1, a guard with a rifle — the courtyard pair past the gate):
1. **Spot**: stand in a guard's line of fire, get hit. **Watch**: BL+0x968 (write, `Player_HandlePain`
   0x1902c8:126-130) jumps to ≥ 128 then fades ~1/frame; `Sound_Play` 0x412 fires on EVERY hit
   (0x1902c8's grunt 0x88 roughly every 4th); with the armour pickup taken, BL+0x8b0 drains before
   BL+0x894 moves. Expected: red edge glow on the hit frame, hurt sound each hit, grunt ~25%.
2. **Spot**: let a melee guard (the kick/punch pair) connect. **Watch**: BL+0x910 writes −0.1
   (punch 0x65) or +0.1 (kick 0x63/0x66) [M 0x194458:239/246/253], sound 0x171 plays, and the flash
   +0x968 still starts; the next frames show a small vertical jolt decaying over ~0.5 s.
3. **Spot**: difficulty switch (Operative vs 007) on 07000009. **Watch**: the same guard hit costs
   0.6× vs 1.0× the record damage ([M 0x1d3a80:97-99] — the port matched the old 0.75/1.0/1.25
   defaults pre-fix).

nf-view renders (no new draw code — the pane/glow/direction art is unchanged; these show the
ported numbers live): `--bond 128,2,128,0,0` during a staged firefight shows the red edge glow at
the hit alpha and the armour arc draining; the death flow (health → 0) shows the blood pane + the
fail menu as before.

## Suite total (final tree, merged with origin/master)

(filled after the run)
