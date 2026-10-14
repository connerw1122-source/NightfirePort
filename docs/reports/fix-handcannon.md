# fix-handcannon — the Remote Mine's detonator (0x38) fires and detonates

Branch `s/fix-handcannon` (from `origin/master` b9ed8b8), task 105b. Status: **done and tested** —
`fix_handcannon_live` 3/3 (all three shown failing pre-fix), the neighbour weapon/gadget tests green,
the full suite on this tree: see "Verification".

## What 0x38 is

The task name said "handcannon"; the disc says otherwise. Resolved through the language bank
(USATxt.dat via the game's own fixup table, `Txt_BindLabel` 0x1cad60's `fixups[id>>24] + (id&0xffffff)`):

- **0x37** = the **Remote Mine** (thrower): selectable (+4 = 1), class 4 explosive, damage 100,
  refire 450, clip cap 1, ammo type 22, fire script 0x06000728 — mode label 0x0500000b "Grenade",
  SP/MP names 0x0500003e / 0x05000067 "Remote Mine" [L ee_start_05.bin record 0x38's lane + USATxt].
- **0x38** = the **detonator**: NOT selectable (+4 = 0), base 0x37, twin −1, ammo type 22 with
  **clip-add +0x91 = 0**, clip cap 500, refire 60, **fire script +0xB0 == the idle script
  0x060004ea**, and **+0x68 = 0x00020010** — the 0x20000 bit is the whole task [L]. Its mode label
  0x0500004d is "Detonate". Holding the mine shows the game's own hint [M Player_WeaponFiring
  0x1a5460:385-390]: 0x10002dc "Use alternate fire to switch to detonator". 0x39 is an inert sibling
  (no scripts), +0x68 = 0x00020010 likewise.
- The +0x6C bullet flags are **0** and +0x70 = 0 for 0x38 — no bullet path of its own; the rounds/
  shot byte +0x18 is 0 (so `Player_WeaponInitBullet`'s generic bullet loop would spawn nothing even
  without the flag arm — the loop count is the raw +0x18 byte [M 0x1a4bf0:141 `lbu 24(s2)`]).

## The game's fire path for the detonator

`Player_WeaponInitBullet` 0x1a4bf0's **first** branch, before the 0x53 case and the bullet loop
[M disasm 0x1a4c30-0x1a4cd4, `lw v0,104(s2); and v0,v0,v1` with v1 = 0x20000]:

1. Walk every object; for each **type-5** object (`lbu 255(a2)` vs 5) whose weapon record's base
   (+0x02 of `BD+0x44`) equals the fired record's base (0x37) and whose `BD+0x30` owner is the firing
   player (`beql v0,s1`): **`BD+0xFC = 10.0`** (`lui at,c120; mtc1 at,f0; swc1 f0,252(a1)`) — the
   mine bullet's detonate fuse, 10 ticks. `Bullet_DoTrails` 0x12d890:268-281 decrements it per tick
   (the 7777.0 skip does not apply) and at < 0 blasts with `Explode_Create(radius = record+0x08,
   damage = record+0x0C, script 0x6000052, …)` [M 0x12d890:388-393] — 8.0 / 100.0 [L record 0x37].
2. `Player_WeaponHasAmmo(BL, 0x37)` (0x1a16c0: carried[type] or the shared clip): armed →
   **`BL+0x63 = 0x37`** (`sb v0,99(v1)`) — the switch request back to the mine. Dry →
   `owned[0x37] = 0` (`sb zero,442(v0)` — the +0x1BA owned byte, bldata.md line 81) and
   `Player_GetBestWeapon` 0x1a9680 writes the request (its tail writes BL+0x63 directly).
3. Return — **no bullet, no muzzle latch (+0x95b never sets in this arm → no muzzle flash), no
   casing, no shot tick.** The fire itself still runs `Player_SetFiringAnim` (the fire script IS the
   idle script, so the view just restarts its loop) and the refire interval +0x40 = 60.

The mines themselves: `Bullet_init` 0x12c5a0:196-223 sets BU+0xFC = **7777.0 for a player owner**
(obj+0xff == 3) and 420.0 otherwise, and `Bullet_DoTrails` 0x12d890:268-273 skips the decrement while
the fuse is 7777.0 — the player's mine waits forever for the detonator; a guard-thrown mine
self-destructs after 7 s.

## The port's three gaps (all shown failing first)

1. **The fire was refused outright** — the F family's "sounds but no shot tick": the idle==fire
   script's loop sounds recorded, nothing else. `GunParams::from_record` forced
   `ammo_per_shot = max(1, +0x91)` = 1 while the held detonator's clip slot read 0 (the give fills
   the BASE's slot per `Inventory::clip_slot` [M Player_EquipWeapon 0x1a8a58]) →
   `update_with`'s `clip >= ammo_per_shot` gate failed every tick. The game checks and spends the
   **raw** +0x91 byte (`(short)(param_2 & 0xff)` [M 0x1a18e0:9-25]) — a 0 clip-add is always
   fireable and spends nothing. Fixed: `ammo_per_shot = w[0x91] as u32`. Same zero-spend family:
   0x01 (melee anyway), 0x39, the Stunner twins 0x4b/0x4d, the Shaver twins 0x5a/0x5c [L sweep].
2. **The +0x20000 arm did not exist.** Added at the top of the shot block (`Sim::run`'s `if release`,
   scene.rs, a `'shot:` labeled block so the existing lines are untouched): mark every player-owned
   grenade of this base with `range = travelled + 10.0` (the port's tick-fuse stand-in for the
   absolute BU+0xFC = 10.0; the grenade explodes on the 11th tick after the mark, as the game's
   DoTrails countdown does), then the has-ammo switch-back to the base, or the dry arm (clear
   clip[base] + owned[base] → `Gun::get_best_weapon` writes the request). No shot tick, no bullet,
   no light for this arm — the generic path is skipped exactly as the game returns early.
3. **The player's mine kept the guard's 420-tick self-destruct fuse** (`let fuse = if wt == 0x37 {
   420.0 }`): the user's mine blew up on its own 7 s after the throw. Fixed to 7777.0 for the player
   spawn [M 0x12c5a0:214-223] plus the DoTrails tick skip in `Grenade::update`
   (`weapon_type == 0x37 && range >= 7777.0` → the fuse does not tick; movement continues — the
   guard-thrown 420 fuse is unchanged) [M 0x12d890:268-273].

## Tests

`crates/nf-view/tests/fix_handcannon_live.rs` — through the real input path (`nf_view::load_weapons`
with both family records pre-merged + `panel::apply` give of 0x37 + `Sim::step_bond`):

- `the_detonator_detonates_the_placed_mine_and_switches_back_to_it` — throw, mode-press to the
  detonator (the game's own route [M Player_Weapon 0x1a24c8:246-260]), fire: the switch request is
  the mine 0x37 on the press tick [M 0x1a4cb8], the blast lands within the 11-tick window with
  radius 8.0 / damage 100.0 / record 0x37, and the mine is consumed. **Fail-first**: pre-fix the
  switch request stayed 0x38 and nothing exploded (the refused fire).
- `the_armed_mine_never_self_detonates` — 600 idle ticks after the throw (past the old 420 fuse):
  no explosion, the mine still armed. **Fail-first** at the old self-destruct tick.
- `the_detonator_fire_spawns_no_bullet_and_still_switches_back` — no mines placed: no shot tick
  [M the 0x20000 arm returns before the bullet loop], and the has-ammo tail still requests the mine.
  **Fail-first**: the request stayed 0x38.

Neighbours green on the fixed tree: fix_grenade_throw_live 4/4, fix_weapons_all_live 4/4,
fix_weapon_sounds_live 6/6, explode_live 2/2, fix_weapon_effects_live 3/3 (+2 ignored),
gadgets_live 4/4.

## Verification

- The suite (`tools/test/suite.py`, NF_ISO, the 4 crates): running on this exact tree; the total
  line and the FAILED/TIMEOUT lines were checked before the push — see the WORKLOG entry for the
  number. Allowances on the slim ISO: the 3 `disc_golden` tests, `pss_movies_live` (no retail image).
- Fail-first: committed as 54afd3d (the tests) before 30e69fa (the fix).

## Shared files

`crates/nf-view/src/scene.rs` (shared): the `'shot:` labeled block opens/closes with two brace lines
and inserts the detonator arm before the existing shot code (no existing line re-indented), plus the
one-line mine-fuse change inside the grenade spawn. `crates/nf-game/src/gun.rs` (+0x91 line) and
`crates/nf-game/src/grenade.rs` (the armed-mine skip) are not on the shared list. No renames, no
reformatting; the added `Gun`/table reads are additive.

## Live checks for the main agent

1. **PCSX2, 07000005 spawn**: give the Remote Mine (0x37), throw one at the floor, press ○ (the mode
   button) — the game hands Bond the detonator (a small black box, fp skin 0x0500004b); press fire —
   **the placed mine explodes immediately (≤ 0.2 s)** with the standard blast, and Bond is holding
   the MINE again without touching the wheel (the auto switch-back [M 0x1a4cb8]). Expected: blast
   radius 8 / damage 100, sound per the 0x6000052 blast script.
2. **The armed mine never self-detonates**: throw a mine, wait 30 s without pressing anything —
   the game's mine sits armed indefinitely (the port used to blow it up at 7 s [M 0x12c5a0:214-223]).
   gdbwatch `BU+0xFC` on the mine object: 7777.0, not decremented.
3. **The dry arm**: empty the mine's ammo (throw 6), hold the detonator, fire → the game takes the
   mine away (owned cleared [M 0x1a4cd0]) and switches to the best remaining weapon.
4. **nf-view spots** (07000005, `--bond` at the spawn, `--give 0x37`): throw (fire), mode-press, fire
   — the first-person model cycles mine → detonator → mine, and the blast sprite/gas/light appears at
   the resting mine's position ~11 ticks after the detonator press. (The renderer's give cannot
   script the throw+press sequence headlessly, so the visual evidence is the live check above; the
   headless numbers are the tests' radius/damage/timing asserts.)

## What's left

- The 0x37 stick behaviour: the game's mine STICKS on first contact (Bullet_CollisionHandler's 0x37
  arm counts the placed mines and pins the bullet [M 0x12ce80:271-330]); the port's mine bounces
  like a frag until it rests. Cosmetic for the detonation (the event, numbers and timing are
  game-accurate wherever the mine is); filed as the follow-up.
- The detonator's auto fire in the game may also route through a fire-script EVENT on some levels'
  merged scripts (the fire script is the idle 0x060004ea everywhere [L]); nothing observed.
- matrix4's in-flight `clip_view` read-side work (s/weapon-matrix4) touches the mode press /
  switch-cycle has_ammo sites; this fix's fire-path sites are disjoint and merge clean (checked: no
  overlapping lines).
