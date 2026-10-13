# fix-explosions2 — the frag's real damage path, and the death-branch routing

Branch `s/fix-explosions2` (from `s/fix-explosions` 88189da + a master merge, re-merged at the end).
fix-explosions held because it made the frag grenade harmless: it read weapon 0x35 as radius 10 /
damage 0 and re-locked the grenade test to "the frag blast damages nobody". The driver was right —
the real game's frag kills. The record reading wasn't wrong; **the weapon-id mapping was**.

## The answer: the frag is 0x34, not 0x35

**The disc name labels settle the mapping** [L]. `Pickup_Handler` 0x18e650 case 0 names a pickup via
`Txt_BindLabel(weapon_data[id]+0x38)` (SP branch, `MPSettings._384_4_ == 0`). `Txt_BindLabel`
0x1cad60 = `FixupTable[label>>24] + (label&0xffffff)` into the language bank's pointer array. The
bank is `USATxt.dat` (FNames[SelLang] 0x2c7498; psiFileLoad over FILES.BIN's FileList). Reimplementing
`Txt_LoadLanguage` 0x1caad0 exactly (bank copy → dword skip → the pointer loop stores `Bank[k] =
bank + offsets[k-2]`, so the game's index k maps to my offsets table entry k−2; the `-1` rewind makes
NumFixups read the last consumed offset = 7, giving FixupTable = [0, 1000, 1812, 1903, 1958, 2065,
2183] for hi bytes 0-6) resolves every weapon name:

| id | SP label | name [L USATxt.dat] | +0x08 radius / +0x0C damage | +0x70 |
|---|---|---|---|---|
| 0x34 | 0x05000031 | **'Fragmentation Grenade'** (MP 0x5b → 'Frag Grenade') | **8.0 / 100.0** | 0x280 |
| 0x35 | 0x0500003f | **'Stun Grenade'** | 10.0 / 0.0 | 0x2280 |
| 0x36 | 0x0500003c | **'Smoke Grenade'** | 0.0 / 0.0 | 0x20280 |
| 0x37/0x38/0x39 | 0x0500003e | **'Remote Mine'** (0x38/39 are 0x37's variants) | 8.0 / 100.0 | 0x18300 |
| 0x3a | 0x05000049 | **'Laser Tripbomb'** (MP 'Tripbomb') | 8.0 / 100.0 | 0x80100 |
| 0x3b-0x40 | 0x0500004c | **'Satchel Charge'** | 8.0 / 2000.0 | 0x300 |

So fix-explosions' "frag 0x35 = 10/0" was really the stun grenade, and everything it inferred from
that ("the game's own frag blast damages NOBODY") collapses. The frag is 0x34 with a real 8/100 record.

**The kill path** [M]: `Bullet_DoTrails` 0x12d890's timer-death branch — gate `weapon+0x70 & 0x200`
(asm 0x12dd2c-0x12dd44), then at expiry:
- `+0x70 & 0x2000` → the **flash branch** (asm 0x12dd60-0x12dd68: `andi 0x2000; beq 0x12e104`): the
  stun grenade only — a sweep of all 115 records' +0x70 shows only 0x35 (0x2280), 0x59, 0x5b carry
  the bit [L]. Gas_Init(0.5, 1.68, fx 0x200001f, 0xB), Camera_Shake(2.0), Sound 0x16,
  Light_Create(10, 5, white), the 30-unit sphere → Bond's whiteout (dist 5→90 × facing × LOS →
  `Player_SetFlashBang`) and drones within 20 u → msg 0x18 → StunGrenadeImpact 0x4D — broadcast only
  for a PLAYER thrower in SP (0x12e094: `MPSettings._388_4_ == 0 && thrower+0xFF != 3` skips).
  **No damage, no 0x6000052 script.**
- `+0x70 & 0x20000` → the **smoke branch** (asm 0x12e104-0x12e1bc): the smoke grenade only (0x36,
  0x20280): first expiry resets state 1, sets `BU+0xF6 = 1`, re-arms the fuse to 1200 ticks (3600 for
  the 0x69 family), `Occlude_Create(pos, 1200, 1)` is the cloud and Sound 0x14 pops it; the second
  expiry dies. No damage, no flash.
- else → **`Explode_Create(wd+0x08, wd+0x08, wd+0x0C, …, 0x6000052, …)`** (asm 0x12e1c0): the frag
  0x34 (8/100), the mine 0x37 (8/100), the satchel family 0x3b-0x40 (8/2000) → `Explode_CollisionHandler`
  0x17cde0 → `Explode_Propagate` 0x17c518: case 2 drones → `Drone_ExplosiveHit` 0x13aab8 → msg 9 →
  `NDrone2_HitDamage` 0x145558; case 3 → `Player_Hurt` 0x196060. **The frag kills a guard next to the
  blast with the game's (int)-truncated linear falloff over radius 8** — 100·(1 − (int)dist/8).

Candidates the driver listed, closed: the impact-explode site `Bullet_handle_object_destruction`
0x12cd98 has no second damage call (it is the chain-reaction/anim-state-10 route into the same
Explode_Create); `Bullet_update`'s tail 0x12efd8 zeroes direct-hit damage for the grenade ids; the
difficulty multipliers in `NDrone2_HitDamage` [L .data 0x30cab0: Easy 2.0 / Normal 1.5 / Hard 0.5]
scale but never zero; `Bullet_init` 0x12c5a0:214-223 gives the mine its 420-tick fuse (7777.0 for a
player owner — trigger-detonated). `DroneWeap_ThrowGrenade` 0x179558's sniper pick (0x35, ⅓ → 0x36)
and `NDrone2_DefaultInit` 0x14b300's +0xBCC map (sel 1→0x36, 2→0x35, 4→0x34, 5→count+0x3a, 6→0x6c;
the 0x7000014 clamp → 0x34) already matched the port's constants — only the names in the comments
were wrong. `OnInitDeath`'s captain drops (0x35 ×2 on 1-8; 0x34 on 9-b/11-16/4a) likewise.

## What landed

- **`drone.rs`** — the tick's grenade explode arm now routes on the weapon's own `+0x70` flags read
  from the table, exactly as the asm does: flash (0x35) / smoke (0x36) / explode. The flash branch
  lost its duplicate raw `play3d(0x16)` and no longer plays the 0x6000052 fireball script; its drone
  stun broadcast is gated on the player-thrown case (SP, [M 0x12e094]). The smoke branch: Sound 0x14,
  the fuse re-arms +1200 ticks once (`Grenade.smoke_popped`), then dies — no whiteout/stun/script
  (the Occlude cloud visual is not modelled; the port has no occluder system). The 0x6000052 script,
  the `explosions` entry and the radius shake now fire only on the explode branch. The mine's fuse
  override (0x37 → 420 [M 0x12c5a0:214-223]) added to `throw_grenade`. Comments corrected to the
  disc names throughout (+0xBCC, the sniper pick, the drops).
- **`grenade.rs`** — the `smoke_popped` field (new line), the fuse doc rewritten to `Bullet_init`'s
  real table (210 / 300 / satchel (id−0x3b)·5+5)·60 / 0x69 → 5 / mine 420·7777), the weapon_type doc
  carries the disc names. Unit-test constructors tagged 0x34.
- **`explode.rs`** — the header evidence block rewritten around the true mapping; the routing doc on
  `Explosion::weapon_type`.
- **`scene.rs`** — the player's throw gained the mine fuse override and the corrected comment (the
  frag = 0x34, 8/100; the old hardcode's sin was killing with EVERY type and on impact).
- **`panel.rs`** — the dev panel's weapon catalog: 0x34 "Frag Grenade", 0x35 "Stun Grenade", 0x36
  "Smoke Grenade" (it said 0x34 "(1 round)", 0x35 "Frag Grenade", 0x36 "Stun Grenade" — the user's
  "a frag grenade spawns what looks to be a stun grenade" was this, one id off).
- **Tests re-locked to the game** — `explode_live.rs`: the frag (0x34) thrown at a guard kills him
  and Bond keeps 100 (the blast script plays); the stun (0x35) at a guard touches nobody's health;
  the routing sweep asserts 0x34 has no flash bit and 0x35 does. `stun_grenade_stuns_without_damage…`
  (was the 0x36-based flashbang test): the stun grenade stuns into 0x4D/0x4E, no damage, the
  whiteout + the 2.0 shake, a shot on the stunned guard → 0x44. `fix_weapons_all_live`:
  `frag_bounces_and_waits_for_the_fuse` (0x34, radius 8.0, bounce-never-explodes kept),
  `stun_grenade_never_kills_but_frag_does` (0x35 produces NO explosions entry and the whiteout; the
  propagate sister uses radius 8), `panel_give_list_matches_weapon_data` (0x34/0x35 gives + the mesh
  0x0200_02af). `fix_player3_live`: the thrown frag (0x34) now asserts the guard ~3 u away is
  hurt/killed (100·(1−3/8) = 62.5), the sister is the stun (0x35) doing nothing. The walk/walkreal
  harnesses' obstacle grenades re-tagged 0x34 (+ `smoke_popped: false`).
- **Fail-first proof**: with the routing key reverted to fix-explosions' `weapon_type == 0x36`,
  `stun_grenade_stuns_without_damage_and_a_shot_kills_the_stunned` FAILED (the 0x35 stun was a dud —
  radius-10/damage-0 propagate, no flash, no stun state); restored, it passes. (The frag-kill maths
  through the else-branch already worked — what was broken upstream is that nothing ever threw a
  0x34: the tests and the labels called 0x35 the frag.)

## Verification

- `cargo test --offline --release -p nf-game --lib`: 121 passed (the sweep + the smoke/fuse units).
- NF_ISO: `explode_live` 2/2, `fix_player3_live` 6/6, `fix_weapons_all_live` 4/4, `models_misc_live`,
  `fix_actors_live`, `fix_finale_live`, `panel_live`, all 22 `walk_*`, all 19 `walkreal_*` (the
  ignored probes as before), plus the rest of the nf-view set from the two full-suite passes — green.
- Full four-crate suite (run in chunks on the 10-min tool limit, same commands): nf-formats, nf-audio,
  nf-game green; only the 3 `disc_golden` bigf tests fail (the slim-ISO allowance). No new failures.

## Live checks for the main agent

1. **The frag kill + falloff** — 07000005, `--bond 100,26,108,0,0`, wheel to the grenade, throw at a
   guard ~3 u from the landing point: the guard dies (or takes ~62 with the level's drone-side
   difficulty factor); a guard 6+ u away survives; Bond at 10 u keeps 100. The blast sound is 0x3E
   (the script), the fireball scale ≈ 8·0.0667. Old build (s/fix-explosions): no damage anywhere.
2. **Bond's own damage from his own frag** — stand 2/5/8 u from where your thrown frag lands: expect
   heavy pain at 2 u (≈ 75·level factor), light at 5 u, nothing at 8+ u (the truncation steps: 100·(1−(int)d/8)).
3. **The stun grenade** — 0x35 thrown within 20 u of a guard: the guard crumples (0x4D/0x4E) with
   full health, Bond whites out, sound 0x16, a white light, NO fireball script, NO kill. A
   guard-thrown stun (07000005's sel-2 throwers) stuns nobody but still whites Bond if close.
4. **The smoke grenade** — 0x36 (MP or the dev panel give): at the fuse one `Sound(0x14)` pop and a
   re-arm ~20 s later with a second pop, then it vanishes; no whiteout, no stun, no fireball. The
   Occlude cloud VISUAL is not ported (the port has no occluder system) — the pop/re-arm are.
5. **The dev panel names** — give 0x34/0x35/0x36: the catalog should read Frag/Stun/Smoke; 0x34's
   first-person model is skin 0x05000041.

## What's left

- The smoke grenade's `Occlude_Create` cloud visual (the port has no occluder system) and the whiteout
  falloff (the port sets flashbang = 1.0 flat; the game's dist 5→90 × facing × LOS curve is unmodelled).
- The mine's player-side 7777.0 trigger fuse and the detonator input; the tripbomb 0x3a's laser-cross
  detonation; the satchel 0x6c's 900-tick timer — the port keeps timed fuses (420) for all of these.
- The guards' stun-weapon drop on stun, the barrels (Propagate's break/destroy targets), the
  SimpleScript 0xE2 lane — unchanged from fix-explosions' list.
- The `walkreal_probe_fb6.rs` WIP from the interrupted fix-final-blockers2 session is preserved on its
  own branch (committed there before this branch was cut).

## Commit map

- `crates/nf-game/src/{grenade,explode,drone}.rs` — the routing, the smoke branch, the fuse, the names.
- `crates/nf-view/src/scene.rs` — the player throw's fuse override + comments (additive).
- `crates/nf-view/src/panel.rs` — the catalog names (shared file, 1 match arm + comment).
- `crates/nf-view/tests/{explode_live,fix_player3_live,fix_weapons_all_live,walk_0700000{1,4,14}_live,fix_actors_live,fix_finale_live,models_misc_live}.rs` — re-locked.
- `docs/port-map/guard-ai.md` — the grenade rows (below).
