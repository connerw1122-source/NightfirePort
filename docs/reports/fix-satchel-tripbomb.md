# fix-satchel-tripbomb — the satchel charge (0x3b) and the laser tripbomb (0x3a)

## What was wrong

The port threw the satchel and the tripbomb as ordinary bouncing grenades: the `+0x100` stick arm
in `Bullet_CollisionHandler` was ported for Oddjob's hat (0x45) only (grenade.rs's old
`if self.weapon_type == 0x45`), `Bullet_init`'s family fuse arm (base 0x3b) was missing from both
spawn paths (the satchel got the default 210-tick fuse instead of the family's
`((id − base)·5 + 5)·60`), and the tripbomb's laser — the arm delay, the beam, the
guard/Bond-crossing trigger and the 6-placed cap — did not exist. The countdown beeps (0x447), the
`SatchelDigits` two-digit display and the stick/arm sounds were absent too.

## The game behaviour, with the addresses

All in ACTION.ELF; the tags follow the repo rules ([M] decompile/disasm, [L] disc/record dump,
[S] suggested).

- **The records** [L — a dump of all 115 `weapon_data` entries via `WeaponData::from_action_elf`]:
  0x3a 'Laser Tripbomb' = radius 8 / damage 100, `+0x70` = 0x080100 (sticky 0x100 + the laser bit
  0x80000, NO +0x200 fuse bit), base (+0x02) = 0x3a; 0x3b..0x40 'Satchel Charge' = 8/2000,
  `+0x70` = 0x300 (sticky + the fuse bit), each basing to ITSELF, only 0x3b selectable. The
  labels are the disc's own (`docs/reports/fix-explosions2.md`'s table [L USATxt.dat]; the atlas
  row `docs/atlas/weapon-data.md` 0x3A).
- **The fuse** [M `Bullet_init` 0x12c5a0:103,192-223]: 210 default, 300 with `+0x70 & 0x10000`,
  then the family arm — base 0x3b → `(id == 0x6c) ? 900 : ((id − base)·5 + 5)·60` and `BU+0x102 =
  100`; base 0x69 → 5; base 0x37 → owner-is-player ? 7777 : 420. With the disc bases only the
  fired 0x3b hits the family arm → **300 ticks (5 s)**.
- **The stick** [M `Bullet_CollisionHandler` 0x12ce80:232-394]: `+0x70 & 0x100` + a world hit →
  `obj+0xF4 = 3` (stuck), the matrix aligned to the NEGATED hit normal [:256-266], parked
  `hit + normal·offset`: the 0x3b arm offset 0.04 + Sound 0x4a7 [:257-258, 319-323]; the 0x80000
  tripbomb arm offset 0.002 + Sound 0x446 + `BU+0xFC = 120.0` [:325-334] — **the tripbomb's arm
  delay is 120 ticks written at the stick, not the 210 init fuse**. A char (types 2/3) hit
  bounces instead [:239-249]. The tripbomb's stick walks the objects and caps the placed set at 6
  per owner, marking the oldest dead [:335-370].
- **The stuck tick** [M `Bullet_update` case 3 0x12e7b0 + the call site disasm 0x12efe8]: the
  speed/velocity zero; the 0x80000 arm counts `BU+0xFC` down and runs the laser block at ≤ 0
  [0x12efa4-0x12efc8]; the hat's 0x45 countdown is hat-only [:363-371]. **`Bullet_DoTrails` runs
  in state 3**: the decompile's `if (param_1 == 0) return` tests the OBJ pointer (a0), not the
  dropped 4th float arg — disasm 0x12d890-0x12d8c8 + `mov.s $f12, $f21` with case-3 `$f21 = 0.0`.
  So the +0x200 fuse arm (0x12d890:262-285) keeps ticking the satchel's fuse while stuck, and the
  family-0x3b tail (:418-553) runs in every state.
- **The satchel tail** [M 0x12d890:418-553]: quarter-seconds = `BU+0xFC·(1/15)`, seconds =
  `BU+0xFC·(1/60)`; Sound 0x447 when the quarter-second CHANGES from `BU+0x100` and
  `((qs & 3) == 0) || ((qs & 1) == 0 && sec < 10) || (sec < 5)` [:441-443]; two kind-0x40 digit
  objects every tick, gfx `SatchelDigits[sec/10]` / `[sec%10]` [L SatchelDigits @0x2bed90 =
  0x2000791..0x200079a], linked to the satchel's cel. The fuse death is the generic blast:
  state 2 + `Explode_Create(record+0x08, record+0x08, record+0x0C, …, script 0x6000052, …)`
  [:391-394] — **8/2000 for the satchel** [L record].
- **The tripbomb's laser** [M case 3 + `PositionBeam` 0x1a9400]: at arm, the beam object is
  created once (kind 0, gfx **0x2000133**, linked to the tripbomb, `obj+0x1c` back-link) with
  Sound 0x27c [0x12ee30-0x12ee50]; every tick after, a ray from the tripbomb position along
  `pos − 200·Z` (Z = −normal from the stick, 0xc3480000 = −200.0 at 0x12efb8), flags 0x20a; a hit
  on char types 2/3/0x28 → `Bullet_handle_object_destruction` 0x12cd98 — gate
  `+0x68 & 0x4000` AND `+0x70 & 0x80204` [M] — → the 8/100 blast [L record 0x3a] and the beam
  object dies (the 0x80000 tail kills `BU+0x38`). The satchel fails the destruct gate (0x3b w68 =
  0x10080 lacks 0x4000) — **it cannot be shot into exploding**; timer-only. No satchel detonator
  record exists on this disc (no 0x20000 carrier in the family) — the task's "timer/remote" timer
  half is the live one on SP.
- **The blast script** 0x6000052 is the port's existing `spawn_blast` path (the one-shot script
  player + its Sound 0x3E) — reused unchanged for both gadgets.

## The port changes

- `crates/nf-game/src/grenade.rs` — `Grenade.phase: Option<Box<StuckGadget>>` (one boxed state:
  the surface normal, the tripbomb's arm countdown, the armed latch, the beep latch `BU+0x100`,
  the digit pair); the stick arm generalized to 0x3a/0x3b (the offsets 0.04/0.002, the arm 120);
  the stuck tick per the case-3 arms; the fuse arm reordered before the tail (the game's DoTrails
  order) and the tail (the beeps + digits) added; the `Arm`/`Beep` events.
- `crates/nf-game/src/drone.rs` — the Stick consumer plays 0x446 for the tripbomb (0x4a7
  otherwise), the Arm consumer plays 0x27c, the Beep consumer plays 0x447 (+ the AI sound list);
  after the grenade loop: the 6-placed cap trim and the tripbomb beam pass (`tripbomb_beams`,
  rebuilt per tick — the ray out along +normal clamped at the wall, the char-crossing trigger at
  1.0 u, the 8/100 blast into `pending_blasts`, the tripbomb removed); the guard throw's fuse
  gains the same family arm.
- `crates/nf-view/src/scene.rs` — the player throw's fuse gains the family arm (base 0x3b → 300;
  base 0x69 → 5); the projectile slot draws a stuck gadget aligned to −normal [M 0x12ce80:266];
  two beam slots (mesh 0x0200_0133, `TRACER_SLOTS+3/+4`) stretched off `tripbomb_beams` like the
  gadget beams; the digit pools (Effect kinds 4 = tens / 5 = units, 2 slots per digit mesh,
  `SatchelDigits` 0x2000791..0x200079a) drawn at the stuck satchel's matrix.

### Shared files touched (small, additive — per the task's shared-file rule)

`grenade.rs`, `drone.rs`, `scene.rs` are the task's named files. Beyond them, one line each
(`phase: None,`) was appended to the existing `Grenade { … }` struct literals so they compile
with the new field: `crates/nf-game/tests/ai_live.rs`, `crates/nf-view/tests/{explode_live,
fix_actors_live, fix_finale_live, fix_player3_live, fix_weapon_fire_modes_live, models_misc_live,
walk_07000001_live, walk_07000004_live, walk_07000014_live}.rs`. No behaviour change on those
paths (all non-gadget grenades).

## Verification

- **The failing-first probe** (run on `git checkout HEAD~1 -- crates`, then restored): a 0x3b
  grenade thrown at a wall through `Sim::step_bond` → `stuck=false, boom_at=300` — it bounced,
  never stuck. On the fixed tree the same scenario sticks and blows at the fuse.
- **Unit tests** (`nf-game --lib grenade`, all green): `satchel_sticks_at_the_wall_and_blows_at_tick_301`
  (the stick at the wall, the normal, the 0.04 parking, the 300-tick fuse from the throw, the
  blast 8/2000, the first beep on the first stuck tail tick, the digits (0,0) by the blast);
  `satchel_fuse_is_not_the_default_210`; `tripbomb_arms_120_ticks_after_the_stick_and_never_fuse_explodes`
  (the 120.0 stick write, the 0.002 parking, the Arm at stuck+121, 2000 further ticks silent).
- **Live tests through the real Sim path** (`nf-view --test fix_satchel_tripbomb_live`, 07000005,
  all green): the satchel stick + the 300-tick blast + the 0x6000052 script; the countdown digits
  (0,4) + the 0x447 beeps; the tripbomb arm at 120, the beam out along +normal, a guard parked on
  the line takes the 8/100 blast and the tripbomb + beam die, a guard 2 m off the line triggers
  nothing; the 7th placed tripbomb trims to 6; and `the_fire_pipeline_spawns_the_family_fuse` —
  the dev panel's give → the switch machine → the held fire → the spawned grenade's fuse is the
  family arm's 300 for the satchel and the `+0x1C/speed` range-out arm for the tripbomb (the two
  spawn-path formula arms, both exercised end-to-end).
- **The full suite** on the final tree: see the total line below.

### Deviations / notes

- The beam DRAW ends at the first wall hit; the game passes the full 200 units and lets the
  renderer depth-clip [S — prevents through-wall beams in the port's renderer].
- The laser's char test is a 1.0 u segment-sphere test on live drones + Bond [S — the game's
  `Collide_RayIntersect` flags 0x20a tests the collision cels; type-0x28 breakables are not in the
  port's ray].
- The digit placement offsets (±0.03, 0.032, 0.06 along the stuck matrix) are cosmetic [S — the
  game's own small rotation/translation pair at 0x12d890:486-511 was not transcribed exactly].
- The mine 0x37's own `+0x100` stick branch (0x12ce80:326-370) stays unported — the mine task's
  scope per the WORKLOG; the mine keeps its bounce-and-rest behaviour.
- The 0x4e/0x4f-bullet walk that drains `BU+0x102` (the digit-lifetime counter some other weapon
  decrements near the satchel) is not ported — no SP weapon fires those records at a satchel in
  the ported content.
- The in-flight beep latch: `phase` exists from the spawn for 0x3a/0x3b so the tail ticks from the
  throw exactly like the game's `BU+0x100` (init 0) [M].

## Live checks for the main agent

1. **The satchel stick + timer**: 07000005, give Bond the satchel (pickup or `fieldprobe`), throw
   at a wall. Expected: it sticks flush with the clang (0x4a7), the two-digit display counts
   04 → 00, the beeps accelerate, it blows 5 s after the throw with a large blast (2000 damage —
   lethal well beyond the frag's radius). `gdbwatch` on the bullet's `BU+0xFC` (the obj+0xe0
   deref + 0xfc): written 300.0 at `Bullet_init`, −1.0 per tick including stuck, blast at < 0.
2. **The tripbomb arm + beam**: throw the tripbomb (0x3a) at a wall. Expected: stick flush (0x446),
   silent 2 s, then one arm beep (0x27c) and a red beam across the corridor; walking Bond or a
   guard through it detonates (8/100). `gdbwatch` on `BU+0xFC`: overwritten to 120.0 at the stick
   (0x12ce80:330), the beam object created at ≤ 0.
3. **The 6-cap**: place 7 tripbombs. Expected: the oldest disappears (the game's walk at
   0x12ce80:335-370 marks `obj+0xFE |= 1`).
4. **The satchel can't be detonated**: shoot a stuck satchel. Expected: nothing (the destruct gate
   needs `+0x68 & 0x4000`, the satchel lacks it [L record]).

### nf-view render spots (the main agent renders before merging)

On 07000005 with the sim stepped until a gadget is placed (or `dev::scenario`-style placement):

- `--bond <near the thrown satchel>` mid-countdown: the satchel model flush on the wall, the two
  digit meshes (SatchelDigits) beside it showing the remaining seconds.
- `--bond <near the armed tripbomb>`: the tripbomb model flush on the wall + the red beam
  (0x2000133) stretching across the corridor to the far wall.
- The stuck models' orientation: aligned to the wall (−normal), not upright.

## What's left

- The exact digit-object offsets from 0x12d890:486-511 (cosmetic [S]).
- The mine 0x37 stick branch (another task's scope).
- The type-0x28 breakable trigger on the tripbomb ray.
