# fix-grenade-throw (102c) — report

Branch `s/fix-grenade-throw` (final tree = merged with `origin/master` e169958).
Task: the user's playtest (results 1013-1825, 07000015 and 07000005): "The grenade gets thrown out
before the pin animation finishes. It takes forever to throw another grenade, even after the
explosion it still takes a few seconds. The explosion is very small and not accurate to the game.
You can hear the bounce, but once it settles it makes a rapid sound as if it's rapidly contacting
the ground." And: "Smoke grenade didn't work."

## What the game does (the evidence)

- **The throw's release frame**: the round spawns when the fire script's frame reaches
  `weapon_data+0x44` = 26 for the grenade family [L records built from ACTION.ELF's own initialiser:
  0x34/0x35/0x36/0x37 all 26], latched at obj+0x95C [M `Player_SetWeaponAnimObj` case 9,
  0x1a2e38:416-420 — `AnimListBuild(...)+0x90 >= (float)*(ushort*)(weapon_data+0x44)`]. The frag's
  fire script 0x0600049d is the 70-frame wind-up whose op-1 plays the pin sound 0x1e4 at frame 16
  [D 07000026.bin (also 01-04, 09-0b, 11-16, 23-25, 26-28, 29, 41, 46, 4a, 4b)]; there is NO op-4.7
  (Bullet_init) event in the grenade's view scripts — the spawn is the +0x44 release frame, not an
  op-4 sub-op-7 event (that op exists [M `AnimProcessScriptCmds` 0x1c1b38 case 4 sub 7] but the
  grenade scripts don't use it). The refire interval is `weapon_data+0x40` = 63 [L record], stored
  at the shot [M `Player_WeaponFiring` 0x1a5460:652-654]; while the fire script plays, flag
  `+0x68 & 0x8000` blocks the refire until the script stops [M 0x1a5460:630-634], so the held
  cadence is the script length (~70) + the re-arm beat, and the clip refills through the
  auto-reload only once the view is idle again [M 0x1a5fe4 block: `anim+0xf4 != 0 → return`;
  `Player_ReloadAmmoType` 0x1a1988 moves 1 round for flag-0x100 records].
- **The blast scale**: `Bullet_update`'s fuse death calls `Explode_Create(f12 = weapon_data+0x08,
  f13 = f12, f14 = weapon_data+0x0C, script 0x6000052, attacker = BU+0x30)` [M disasm
  0x12ce1c-0x12ce2c]. `Explode_Create` stores param_1 into `SCRIPTINFO+0x94` [M 0x17cbd0] and
  `Script_GetInterp` multiplies every key scale by `SCRIPTINFO+0x94` [M 0x1da158:117, 128, 177];
  the entity matrix is then `Mat_Scale3f(key_scale × +0x94)` which scales ONLY the 3×3 — the
  translation is copied unscaled [M 0x1e35f0:0xc-0xe]. So the blast script plays at scale =
  radius (8.0 for the frag [L record: 'Fragmentation Grenade', radius 8 / damage 100]).
  `obj+0xE8 = radius·0.0667` is a different field (no visual reader found) — the port had used it
  as the visual scale: **15× too small**, the user's "very small and not accurate to the game".
- **The bounce sound**: `Bullet_CollisionHandler` 0x12ce80 (the block at LAB_0012d4bc): the flash
  (`+0x70 & 0x2000`) plays 0x15 and the smoke (`& 0x20000`) plays 0x13 on the FIRST bounce only
  (counter == 0); every other weapon plays 0x51c when the bounce counter is EVEN and < 0xF —
  counters 0,2,…,14, 8 sounds max. The counter is read BEFORE its own increment. The slow-bounce
  tail zeroes the speed below `FRAME_RATE_MUL·0.01` [M 0x12ce80 tail] and `Bullet_update`'s
  re-normalise (`Vec_NormaliseLen` stores the length back, disasm 0x12f020-0x12f030) keeps a
  zero-speed bullet's move at zero — a rested grenade never re-collides, so it never sounds again.
- **The smoke grenade (0x36)**: the first fuse expiry pops `Occlude_Create(pos, 1200, 1)` [M
  `Bullet_DoTrails` 0x12e114-0x12e1b4] — a static class-0x2A object, mesh 0x2000273 for kind 1
  [M 0x17ea80], whose only update is a 1200-tick countdown [M `Occlude_Update` 0x17eba0] — plus
  Sound 0x14, and re-arms the fuse +1200; the second expiry kills it. No damage anywhere (the
  record is radius 0 / damage 0 [L]; the smoke branch never reaches `Explode_Create`).

## The repro (headless NF_ISO, real input path)

`crates/nf-view/tests/fix_grenade_throw_live.rs` (4 tests, through `nf_view::load_weapons` +
`panel::apply` give + `Sim::step_bond` trigger), first run on the pre-fix tree:

- **07000005 + the panel-given frag: NO grenade ever spawned** (the first run's `spawns []`). The
  frag's view scripts (0x0600049c/d/e) are not in 07000005's archive [D; the archive scan], so the
  view script slot went to a hash with no entry, `view_time` pinned at 0 and the release frame never
  arrived. This is the give path's asset-merge rule (`Wanted::Ids` at load / the warm reload the
  windowed app answers, lib.rs `pending_weapon_assets`); 07000015 (the user's level) carries the
  scripts in its own archive, so its throws ran the release-frame path. The tests load pre-merged
  (`load_weapons`) to exercise the throw itself.
- The blast scale was 0.533 (radius·0.0667) — the test asserted 8.0 and failed.
- The bounce sounds: 9+ ticks of 0x51c across a settled grenade (the per-tick micro-collisions
  re-colliding through the re-accumulated gravity) — the user's "rapid sound".
- The smoke: popped at tick 235 with the fuse re-arm but NO cloud — "Smoke grenade didn't work".

## The fixes (each cited above)

1. **`spawn_blast` scale** (drone.rs): `spawn_once_scaled(…, radius)` instead of `radius·0.0667`
   [M 0x12ce1c-0x12ce2c + 0x17cbd0 + 0x1da158:117].
2. **The script player no longer scales key translations** (nis.rs `keys()`): the game scales only
   the scale output [M 0x1da158:117] and `Mat_Scale3f` keeps the translation [M 0x1e35f0]. With the
   blast now at scale 8 the old ×8 translation would have flung the blast script's sub-effects up
   to ~10 units off (the blast is the only player whose scale ≠ 1, so nothing else moves).
3. **The bounce-sound gate** (drone.rs): flash 0x15 / smoke 0x13 on the first bounce only; 0x51c on
   even counters < 0xF, reading the pre-increment counter [M 0x12ce80].
4. **The rest latch** (grenade.rs): the slow-bounce zeroing latches `resting`; a rested grenade
   ticks only its fuse (no movement, no collision, no sound) [M 0x12ce80 tail + 0x12f020-0x12f030].
5. **The smoke cloud** (grenade.rs `SmokeCloud` + drone.rs push/tick + scene.rs render pool):
   `Occlude_Create(pos, 1200, 1)` — mesh 0x2000273, a 1200-tick countdown, drawn at `obj+0x30`
   [M 0x17ea80/0x17eba0]. The mesh resolves in every level archive that has it (checked on
   07000005: by_hash → map 12 objects 4-7); the pool is built only where it resolves.

**Throw timing**: the release-frame path (spawn at +0x44 = 26) and the refire (+0x40 = 63) were
already the port's rule; the task's repro verified them against the game's numbers instead of
changing them: pin sound at tick 15 (frame 16), spawn at tick 25 (frame 26), held-trigger gap 71
(the game: the script 70 + the re-arm beat ≈ 72 [M 0x1a5460:630-634 + 0x1a6358 block]). The
press-release-press cycle: the second press while the fire script still plays and the clip is empty
does nothing (the game's own flow — the reload waits for the idle view [M 0x1a5fe4]); the next
press throws. The "takes forever" symptom matched the missing-script case on 07000005 (no throw at
all) and the game's own cadence once the scripts are present; no port-side deviation remained.

## Files

- Task files: `crates/nf-game/src/grenade.rs` (the rest latch, `SmokeCloud`), `crates/nf-game/src/explode.rs`
  (untouched in the end — the propagate rules were already faithful), `crates/nf-game/src/gun.rs`
  (untouched — the timing verified), `crates/nf-view/tests/fix_grenade_throw_live.rs` (new).
- Required adjacent touches (NOT in the task's list, each a small fix with the citation inline):
  `crates/nf-game/src/drone.rs` (the blast scale, the bounce gate, the smoke push + the
  `smoke_clouds` field/ctor/tick), `crates/nf-game/src/nis.rs` (`keys()` translation), 
  `crates/nf-view/src/scene.rs` (the cloud's Effect pool + draw arm). Shared-file hooks: the
  `smoke_clouds` field sits on its own lines (merge lesson 4); the Grenade constructors gained
  `resting: false` in the tests that build one by hand.
- Sister assertions: the pin sound precedes the spawn; the held gap is bounded [63, 130); the smoke
  never explodes; the cloud's lifetime is 1200; the blast radius/damage equal the record.

## Verification

- The new tests: `fix_grenade_throw_live` 4/4 on the final tree (they failed on the pre-fix tree:
  no spawn / scale 0.533 / rapid sounds / no cloud).
- Final-tree suite (merged with origin/master), foreground chunks with `--test-threads=1`
  (the sandbox kills background jobs between tool calls, so `suite.py`'s nohup run could not
  survive; the same binaries ran chunked instead):
  - `nf-game` + `nf-formats` + `nf-audio`: **352 passed, 3 failed, 2+ ignored** — the 3 failures
    are exactly the allowed `disc_golden` trio (bigf header/tables/directory on the slim ISO).
  - `nf-view` (all binaries incl. `levels_sweep_live` 3/3 and `weapon_matrix_live` 28/28 re-run
    explicitly): **326 passed, 0 failed, 7 ignored**.
- The grenade numbers on the final tree: pin 15 → spawn 25; held gap 71; bounce sounds [35, 141,
  166] then silence for 120 ticks after settle; blast 8.0/100.0 at script scale 8.0; smoke pop at
  235 with the cloud at 1200 ticks and no explosion.

## Live checks for the main agent

PCSX2 (spot, action, expected):
1. 07000015 Paris Prelude, the frag armed: press fire once — the pin click, the arm winds up, and
   the grenade leaves the hand about 1/3 s after the pin click (frame 26 of the 70-frame wind-up),
   NOT at the press. `--bond 105,25,108` facing the courtyard.
2. Hold fire: the next grenade releases ~1.2 s after the previous (refire 63 + the script's stop
   beat); not "forever", not machine-gun.
3. The frag explodes ~3.5 s after the throw with a LARGE blast (the fireball's scale 8.0) —
   compare against the retail game's explosion at the same spot.
4. The smoke grenade: pops a large grey cloud that persists ~20 s and damages no one.
5. A settled frag: one short rattle as the bounces die out, then SILENCE while it sits.

nf-view (`--bond x,y,z,yaw,pitch`, what each should show):
- `--bond 105,25,108` on 07000005 after giving the frag: the throw arc, the blast at scale 8.
- The smoke cloud drawn at the pop position (mesh 0x2000273) for 1200 ticks.

## What's left / notes

- The bounce sound's other branches (the satchel/belt rest-impact 0x4a7 at 0x12d0xx, the sticky
  0x80000 sound 0x446, the ricochet-prob effect) were NOT touched — they belong to the
  satchel/ricochet systems, not the grenade family's settle rule.
- The smoke re-arm for the non-grenade 0x69 family (3600 ticks) stays generic +1200 in the port
  (only 0x36 carries `+0x70 & 0x20000` among the grenades [L sweep]); no behavioural difference.
- The mine 0x37 keeps the guard-thrown fuse 420 (the player's trigger detonation 7777.0 needs the
  detonator gadget — a separate system).
