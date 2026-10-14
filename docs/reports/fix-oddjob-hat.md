# fix-oddjob-hat — Oddjob's hat (0x45): a thrown one-shot kill, not a grenade

Playtest 1013-2313: "Oddjob's hat should be a one-shot kill but it acts like a grenade."
Branch `s/fix-oddjob-hat` (from origin/master 7785582; merged origin/master b9ed8b8 mid-task).

## What the disc says (the evidence chain)

**The hat's weapon record** — weapon 0x45, EE 0x2c398c in `work/ee_start_05.bin` (table 0x2bf150,
record stride 0x10C) [L, cross-checked against the port's own disc parse]:
- +0x0C damage **100.0** — one touch kills any guard (the guards' health is below it).
- +0x1C range 1000.0; +0x20 speed **0.6667** units/tick [M `Bullet_init` 0x12c5a0:187-191 reads
  +0x20 into BU+0xF8]; refire +0x40 = 26; the release frame +0x44 = 13.
- **+0x6C = 0x20C08**: gravity (0x8), no flesh pass-through (0x800 — `Bullet_CollisionHandler`
  0x12ce80:136-141's material-0x10 pass-through is gated OFF), and the **homing bit 0x20000** —
  the ONLY 0x20000 carrier among the thrown projectiles [L the record sweep: 0x2a/0x2b/0x34/
  0x35/0x36/0x37/0x3a-0x40 have none].
- **+0x70 = 0x100**: the stick flag. No 0x4 (no contact blast), no 0x200 (no fuse — `Bullet_init`
  0x12c5a0:192-198's 300-tick arm needs 0x10000; the fuse gate `Bullet_DoTrails` 0x12d890:262-281
  ticks 0x200 records only) — **the hat never explodes**. The mine 0x37 / tripbomb 0x3a / satchel
  0x3b-0x40 also carry 0x100 [L sweep] — they stick in the game too; their branches stay the
  satchel task's scope.

**The flight: `Bullet_homing` 0x12f358** [M], run from `Bullet_update` 0x12e7b0:476-478 after every
flying tick for `+0x6C & 0x20000` bullets:
- Target acquisition (:64-104): the nearest **heat-seekable** object within 1000 units inside the
  0.66-dot cone of the flight direction, with line of sight (`Collide_LineOfSight(…, 8)`), stored
  at BU+0x40; kept until it dies. `Bullet_heatseekable` 0x12f248 [M]: live guards (class 2:
  +0xFC set, health +0xAC > 0, anim state ≠ 1), Bond (class 3), copter bodies (class 0). The
  owner is skipped (:72) — the hat never homes on its thrower.
- Steering (:119-166): `dir += clamp((to_target − dir)·0.0333, ±0.01)` per axis, re-normalised;
  the speed (BU+0xF8) is preserved; the matrix re-aligns to the dir. No RNG draws anywhere.
- **There is no return-to-thrower anywhere** — the previous report's "unmeasured" item resolves
  to this: the hat homes on TARGETS, and after a world stick it just sits (below). `Bullet_update`
  case 3 and `Bullet_CollisionHandler` have no thrower-seeking arm; the state-3 tail only counts
  the hat's lifetime down.

**The world contact: the stick** — `Bullet_CollisionHandler` 0x12ce80's `+0x70 & 0x100` block
(:232-397) [M]:
- A solid world hit sets `obj+0xF4 = 3` (stuck) at :255, aligns the hat's matrix to the negated
  hit normal (`Mat_Align2Dir`, :256-266), and the **hat alone** gets the 1500.0 post-stick
  lifetime (0x44160000, :315) + the **0x4a7 contact clang** (`Sound_Play3D(96.0, −1.0, −1.0,
  0x4a7, …)`, :317). The satchel 0x3b gets the clang but not the lifetime; other +0x100 records
  get neither.
- The bounce block (LAB_0012d4bc) is UNREACHABLE for a +0x100 record's world hit: its gate
  (:398-407) needs `+0x70 & 0x80` (the grenades' bounce bit — the frag 0x34 carries 0x280, the
  hat's 0x100 has no 0x80) or the character-hit flag iStack_100 — the world path sets neither.
- The stuck hat's handler never runs again (`if (obj+0xF4 != 3)` :102) — **a stuck hat kills
  nobody**; `Bullet_update` case 3 (0x12e7b0:285-371) zeroes the speed/velocity, keeps the
  position, and counts the 1500 down **for the 0x45 only** (:363-371), raising the delete flag at
  ≤ 0 — a SILENT death (`Bullet_Delete`'s effect kind = +0x70 & 7 = 0 spawns nothing).
- Versus a character (kinds 2/3 → iStack_100, :239-249) the hat takes the standard bullet hit
  (the damage → msg 8 — the 100.0 one-shot) and BOUNCES (state 1) — fix-gadgets-live's
  contact-kill pass on master already covers the kill; it keeps flying after the kill, matching.

## What was wrong in the port

The hat rode the grenade path (`Grenade`, the `+0x6C & 0x8` gravity spawn in `scene.rs`):
1. **No homing** — the hat flew dead straight and sagged into the floor short of the guard
   (gravity −9.8/3600 per tick² over a 0.6667/tick flight digs ~1.9 units by 25 units), then
   bounced and hopped like a grenade. The kill only landed on a dead-on close throw or a lucky
   ricochet — "it acts like a grenade".
2. **The grenade bounce at every world contact** — the 0x51c bounce gate, a reflected velocity, a
   rest; the game's hat STOPS at its first world contact with the clang. fix-gadgets-live had
   silenced the fuse blast and added the contact kill but left the bounce, the clang and the
   stick lifetime open ("Left: the hat's return-to-thrower and its 0x4a7 contact clang").

## The fixes (all additive; every change cites the game address)

- `crates/nf-game/src/grenade.rs`:
  - `Grenade.stuck` + `Grenade.seek_target` fields (own lines, at the end of the struct).
  - `GrenadeEvent::Stick(V3)`; const `HAT_STICK_LIFE = 1500.0` [M 0x12ce80:315].
  - `update()`: the stuck tick (the silent countdown, the 0x45-only gate [M 0x12e7b0:363-371]);
    the world-hit branch: `weapon_type == 0x45` → freeze + Stick instead of the reflect (the
    mine/satchel +0x100 stick branches are deliberately NOT touched — see the `stuck` doc).
- `crates/nf-game/src/drone.rs`:
  - The homing pass in the tick's grenade loop (flag-driven `+0x6C & 0x20000`, the same shape as
    the port's existing bullet homing: keep/refresh the target, the 1000-unit 0.66-cone LOS
    acquisition, the ±0.01/axis steer, the speed preserved; no RNG draws). A stuck hat does
    nothing [M 0x12ce80:102].
  - `hat_out`/`hat_kills` exclude stuck hats (the state-3 gate).
  - The `Stick` event arm: the 0x4a7 clang once, no bounce sound, no explosive broadcast, no
    blast [M 0x12ce80:316-317].
  - `throw_grenade`'s constructor gains the two new fields.
- `crates/nf-view/src/scene.rs`: the player-throw constructor gains the two new fields (the
  existing fuse/range model unchanged — the hat is a no-0x200 record, so its "fuse" stays the
  range-out equivalent +0x1C/speed = 1500 ticks [M 0x12e7b0:158-162], and the Sim's Explode arm
  breaks for it as before).

## Tests (`crates/nf-view/tests/oddjob_hat_live.rs`, both green on the merged tree)

Through the real input path (`nf_view::load_weapons` + the panel give + `Sim::step_bond`), on
07000005 with `--give 0x45`:

1. `the_hat_sticks_where_it_lands_clangs_once_and_kills_no_more` — **the failing-first
   discriminator**: a floor throw must STOP at the first contact (a frozen position, exactly one
   0x4a7 clang, ZERO 0x51c bounce sounds, no explosion), a guard moved onto the stuck hat keeps
   his health (the state-3 handler gate), the hat silently expires after the 1500-tick lifetime,
   and — sister — the frag 0x34 at the same floor still plays its 0x51c (the per-record routing,
   not a global change). **Shown failing on the pre-fix tree** (`git stash` of the three source
   files): FAILED at "the hat never stopped moving — it bounced like a grenade".
2. `the_thrown_hat_kills_the_guard_in_one_hit_and_never_bounces` — a 20-unit aimed throw at a
   parked guard (a verified segment-clear corridor, the other guards parked behind): the guard
   dies in one hit, `explosions` stays empty, ZERO 0x51c in the whole run, and a guard beside the
   victim is untouched (no blast radius). Honest note: on the pre-fix tree this test PASSES — at
   20 units the unsteered flight's dig point lands at the 1.05 contact-sphere edge, so the
   grenade path's dig-bounce still clips the guard (the pre-fix hat kills at close-to-medium
   range by direct hit or hop-luck, which IS the "acts like a grenade" feel). The homing's
   failing-first was attempted with four geometries (a 0.17-rad off-aim throw, a mid-flight
   2.5-unit guard shift, a lob-over) — each is either closed by the game's own gentle steer
   (0.9667^t closure cannot recover a 0.17-rad error inside a 20-unit flight) or made flaky by
   the throw-spread noise; the details are in the session log. The homing's user-visible value
   is at range and on moving guards — see the live checks.

## Verification

- Failing-first: test 1 (the stick) shown FAILED pre-fix (above). Post-fix both green.
- The merged tree (origin/master b9ed8b8 + the merge commit 6b59a52):
  - `nf-game` + `nf-formats` + `nf-audio`, `--release -j2 -- --test-threads=1 --no-fail-fast`:
    **46 binaries — 362 passed / 3 failed / 0 ignored**; the only failures are the ALLOWED
    `disc_golden` trio (bigf_header_fields, bigf_tables, directory_tree — the slim ISO).
  - The 16 nf-view neighbour binaries touched by the Grenade/scene/drone changes
    (oddjob_hat_live, gadgets_live, gadget_laser_live, gadget_grapple_live, explode_live,
    fix_grenade_throw_live, fix_weapon_fire_modes_live, fix_weapon_effects_live,
    fix_weapon_sounds_live, models_misc_live, fix_actors_live, walk_07000004/1/14_live,
    fix_player3_live, fix_finale_live): **65 passed / 0 failed**.
  - The Grenade struct grew two fields — every initializer across the tree was updated
    (ai_live, explode_live, fix_actors/finale/player3, walk_01/04/14, models_misc,
    fix_weapon_fire_modes + the 4 in grenade.rs's own tests).
  - `tools/test/suite.py` cannot survive this sandbox's background kill (dies between tool
    calls — the same as sessions 102c/100f); the same binaries ran in foreground
    `--test-threads=1` chunks as above (the grenade-throw session's precedent).

## Shared files touched (the driver's list: drone.rs, scene.rs, player.rs, lib.rs)

- `drone.rs`: the homing block, the two `!g.stuck` exclusions, the Stick arm, the throw
  constructor fields — all additive; master's projectile-trails block kept untouched beside them
  (the merge resolved both in the same loop).
- `scene.rs`: two lines in the player-throw constructor.
- `grenade.rs`: the struct fields, the event variant, the const, the two update branches.
- `player.rs` / `lib.rs` / `gun.rs`: untouched.

## What's left

1. The mine 0x37 / tripbomb 0x3a / satchel 0x3b-0x40 +0x100 stick branches (no 1500 re-arm, no
   countdown, trigger-detonated) — the satchel/mine task's scope, documented on the `stuck` field.
2. The hat's stuck ORIENTATION is not modelled (the game aligns the matrix to the hit normal,
   0x12ce80:256-266) — the port's grenade mesh draw doesn't take an orientation; invisible in the
   current renderer, flag for the render pass.
3. The hat vs a kind-0x15 object: the game bounces off it (iStack_100) — the port's contact pass
   keeps flying straight; unmeasured on the disc which objects are kind 0x15.
4. The homing vs Bond as a target (the game's heatseekable includes Bond for a drone-thrown hat —
   the port's hats are player-thrown and the scan is drones-only; if the MP Oddjob bot ever
   throws, revisit).

## Live checks for the main agent

PCSX2 (07000008 or any MP-ish level, `--give 0x45`):
1. Throw at a guard ~15-25 units out, slightly off-aim: the hat should visibly CURVE toward him
   (the homing) and kill on the touch — one hit, no explosion, no bounce sounds (0x51c) — and
   after the kill it flies on, then sticks to the first wall/floor with a metal clang (0x4a7) and
   lies there ~25 s (1500 ticks), then vanishes silently.
2. Throw at a wall: it STICKS where it hits (no ricochet hop), clangs once, kills nobody who
   walks into it afterwards, and disappears after ~25 s.
3. Throw at the floor mid-range: the same stick on the floor (the game's hat does NOT roll or
   rest like the frag).
4. gdbwatch on the stuck hat's `obj+0xF4` (the state): expect 3 after the wall contact, and the
   BU+0xFC slot 1500.0 counting down 1/tick.

nf-view renders (`--bond` spots, on this branch):
- Any spot with a guard in front: `--give 0x45`, hold fire — the thrown hat should arrive at the
  guard and he drops with no blast sprite.
- A wall-facing spot: the hat parked on the wall/floor at the contact point (the mesh draw keeps
  its flight orientation — the normal alignment is not modelled, see What's left #2).
