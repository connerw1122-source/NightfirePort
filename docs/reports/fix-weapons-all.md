# fix-weapons-all — report

Branch `s/fix-weapons-all`, worked 2026-10-13 05:40–07:30 UTC (one sandbox session). Everything below
was reproduced first through the real input path (the dev panel's own `panel::apply` give commands +
`Sim::step_bond` weapon-switch and fire input), then fixed from the decompile, with NF_ISO tests in
`crates/nf-view/tests/fix_weapons_all_live.rs` (4 tests, all green) and the updated unit tests in
`crates/nf-game/src/grenade.rs`.

## Done

### 1. The frag "blows up immediately as soon as it hits the ground" — fixed [M]
`Bullet_CollisionHandler` (0x12ce80) NEVER explodes a bounced grenade:
- a reflected speed below `FRAME_RATE_MUL·0.01` only zeroes the speed (`*(float *)(BU+0xF8) = 0`) —
  the grenade sits, state stays 1, no `Explode_Create`;
- the 20th bounce (the `(ushort)(count+1) < 0x14` else-branch) likewise only aligns the matrix and
  zeroes the speed;
- only the tick fuse (`Bullet_init` 0x12c5a0:103 — 210 ticks for the frag [L 0x35], 300 with
  `+0x70 & 0x10000`, 420 for the mine) explodes it.
The port exploded on BOTH conditions. Fail-first: on 07000015 the thrown frag exploded at tick ~90
(decayed bounces on flat ground); after the fix it bounces (first bounce at tick 33) and pops at tick
208 of its 210-tick fuse (`frag_bounces_and_waits_for_the_fuse`, green).

### 2. The bounce feel ("does not throw well") — improved [M+L]
The bounce scale is the per-material coefficient table at `0x3037dc + material·0x88`
[M 0x12ce80 `fVar23 = speed * *(float *)lVar10`]. Measured from ACTION.ELF [L]: 0.5 for materials
0..12 and 15..21, 0.0 for 13/14/22 (the rest of the `& 0x3f` range is unrelated data). The port used a
flat 0.4 stand-in; `Grenade::update` now takes the world's material id (`segment_surface` already
returned it) and uses the measured coefficients.

### 3. "No clue what the rocket launcher is" — fixed
The dev panel named weapon 0x3A with the bare fallback "weapon 0x3a". It is now "AT-600 Scorpion
(rocket launcher)" — cross-referenced from the user's own playtest ("the Scorpion launcher's
heat-seeker mode"), the game text bank's `AT-600 Scorpion ` string [D USATXT.DAT in FILES.BIN], and
models-weapons-items' pin 0x3A → rocket mesh 0x0200074f. Also named 0x33 "Korsakov K5 (scoped, 100
rounds)" and 0x45 "Militek MGL (grenade launcher)" [S — see the live-check caveat below].
Fail-first: `panel_give_list_matches_weapon_data` asserted the catalog contains no "weapon 0x3a" and
failed on the old code; green now. The same test locks the panel give list: the quick buttons'
decimal ids 53/54 parse to 0x35/0x36 (correct — frag/stun), each give equips exactly that id
(15–29 ticks through the real `switch_to` path), each has its own view model, and the pinned
projectile meshes hold [L ee_start_05.bin @ 0x2bf150+w·0x10C+0x5C]: 0x35 → 0x02000726,
0x36 → 0x0200071f, 0x3A → 0x0200074f.

### 4. "The flashbang kills enemies" — locked out with tests
The stun's record carries explosion radius +0x08 = 0.0 and damage multiplier +0x0C = 0.0
[L ee_start_05.bin], so `Explode_Propagate` (0x17c518) can damage nobody. The throw path already
feeds the record's radius; `stun_grenade_never_kills_but_frag_does` throws a stun through the real
fire path (explodes, radius 0.0) and asserts `explode_propagate` at radius 0 damages nobody, with the
sister assertion that the frag's radius-10.0 blast at the same spot does reach a guard. I could not
reproduce an actual stun kill in the port (what I tried: panel-given stun thrown at a parked guard on
07000001 and 07000015 — no damage; the only damage path is `Explode_Propagate` with the record
radius). The user's kill was most plausibly the frag/stun confusion nades05 describes; the name/mesh
audit above addresses that.

### 5. "Every single weapon needs to have a first person model" — verified on the user's level
`every_fp_weapon_has_a_view_model_on_07000012`: after `load_dev` on 07000012 every selectable weapon
with a +0xDC skin has a view model; the test asserts the daily drivers (0x02 P2K, 0x35, 0x36, 0x3A)
and prints the gap list (empty for the selectable set on this level). The session merges
(`merge_weapon_models` + `merge_dev_view_models`) do the work; no code change was needed there.

### 6. Why a throw can do nothing on some levels [D scan, new knowledge]
The throw animation scripts live per level: 0x06000703/0x06000704 (grenade wind-up/throw) are in
07000004/06/0b..0d/11/12/13/15/16, the rocket's 0x0600073c only in 07000013/14/15/16/23/25-28. On
07000001 NONE of them exist, so the release frame (`fire_anim_len` 26 [L 0x35 +0x44]) never arrives
and a panel-given grenade cannot be thrown there — the first version of my tests hit exactly that
("the frag throw must spawn a grenade projectile"). The game behaves the same way (the bullet spawns
from the anim frame), so a throw attempt on 07000001 with a dev-given grenade is a data-shaped dead
end, not a port bug. The tests run on 07000015 (has all three).

## Verification

- `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1`: everything green except the 3 `disc_golden` tests (the known
  slim-ISO failures). One follow-up fix: `walk_07000014_live` dropped a zero-velocity grenade on Rook
  and relied on the removed contact explosion; the drop is now one tick from its fuse (travelled 59
  of range 60) — the same game timing, cited in the test. Both of its tests pass.
- The 4 new tests in `crates/nf-view/tests/fix_weapons_all_live.rs` drive the real paths: panel
  `apply` → `switch_to` input → `gun.weapon`, the fire-held throw (release at the fire script's
  `fire_anim_len`), and the sim's own grenade tick.

## Files touched

- `crates/nf-game/src/grenade.rs` — the bounce fix + coefficients + tests (task file).
- `crates/nf-view/src/panel.rs` — `weapon_name` entries (task file, weapon parts).
- `crates/nf-view/tests/fix_weapons_all_live.rs` — new (tests).
- `crates/nf-view/tests/walk_07000014_live.rs` — the Rook drop timing (minimal, shared-file note:
  test-only, cites 0x12ce80).
- `docs/WORKLOG.md` — my entry appended.

## What's left (next session)

- **Fire modes / alt-fire switch**: the Scorpion's heat-seeker mode. The record data is in place
  (0x3A fire-mode labels [L 0x05000049/0x02000004/…], `shots_per_pull[3]`, `+0x68 & 0x40000` alt
  scripts [M]); the mode-switch key (`Player_WeaponFiring`'s BLData+0x958 toggle reader
  [M Player_SetFiringAnim 0x1a3c70:40-49]) is not wired to an input yet.
- **Zoom/scope steps** (+0x88, `Player_Zoom`), burst/auto timing per mode, spread per mode
  (`+0x24` per-mode reads), reload kinds (clip vs shell, +0x68 & 0x100) — the per-weapon matrix from
  the task brief needs the same fail-first treatment.
- **Punch knock-out vs kill**, **guards dropping multiple objects**, **07000012 pistol sound**,
  **dev-panel level-switch weapon lock**, **07000012 unkillable NPCs** — untouched this session.
- **The gondola level's rocket launcher + ammo pickups** (fix-objects coordination): not investigated.

## Live checks for the main agent

1. **The text-bank label binding**: USATXT.DAT parses cleanly [size 135168, data 102250, 2801
   entries, fixup boundaries 0/0x3e8/0x714/0x76f/0x7a6/0x811/0x887/0xaf1], and the weapon-name
   clusters are at string indices ~2104-2143 (long: 'Wolfram P2K' 2117, 'Fragmentation Grenade' 2112,
   'AT-600 Scorpion ' 2135) and ~2154-2172 (short: 'P2K' 2159, 'Frag Grenade' 2154). But no single
   FixupTable[5] base reproduces sane bindings (`Txt_BindLabel` 0x1cad60: `FixupTable[h>>24] +
   (h&0xffffff)`); e.g. weapon 0x35's label 0x0500003f wants base 2073 for 'Fragmentation Grenade'
   while 0x02's 0x05000038 wants 2065 for 'Wolfram P2K'. Check in PCSX2: break `Txt_BindLabel`
   (0x1cad60), trigger a pickup message for the frag grenade, and record the returned char* — that
   settles the base and lets the panel use the game's own labels for every weapon.
2. **The Scorpion mode switch**: with 0x3A equipped, fire and watch BLData+0x958 (the alt-fire
   toggle) and which fire script (+0xB0 vs the alternate) plays. Expected: one of the d-pad /
   secondary-fire inputs flips 0x958 and the muzzle datum switches to bone 4.
3. **Stun explosion radius**: break `Explode_Create` (0x17cbd0) when a stun grenade (weapon 0x36)
   pops; the radius arg ($f12) should be 0.0 [L] — confirming a stun can never damage.

## Render spots for the main agent (visual verification)

- `--dev --bond 94.2,42.1,0.0,3.6,0.0` on 07000015 with a panel-given frag, mid-throw: the thrown
  body should be the frag mesh 0x02000726 (the oval frag), NOT the stun cylinder 0x0200071f.
- Same level, stun equipped and thrown: the 0x0200071f body in flight.
- 07000012 with the panel's "AT-600 Scorpion (rocket launcher)" given: the name in the dev panel's
  Weapons list, and a fired rocket showing mesh 0x0200074f.
- Grenade timing on flat ground (07000015, throw down a corridor): 2-4 visible bounces at half speed
  each, then the body SITS for up to ~2 s before the blast (210-tick fuse) — no contact explosion.
