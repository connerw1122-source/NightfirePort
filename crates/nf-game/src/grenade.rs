//! Grenades: the throw (`DroneWeap_ThrowGrenade` 0x179558, `DroneWeap_CanThrowGrenade` 0x17a798), the projectile's
//! flight (`Bullet_update` 0x12e7b0 for a gravity-affected bullet), the bounce (`Bullet_CollisionHandler` 0x12ce80),
//! the explosion (`Explode_Create` 0x17cbd0, `Explode_Propagate` 0x17c518, `Drone_ExplosiveHit` 0x13aab8). Spec:
//! docs/systems/drone-ai.md "Grenades", docs/systems/drone-cover.md (the throw state 0x2D).
//!
//! The game has no separate grenade entity — grenades are bullets (`obj+0xFF = 5`) with `weapon_data+0x6C & 0x8`
//! (gravity), `+0x70 & 0x4` (explode on impact) and `+0x70 & 0x100` (bounce). The port models them as a `Grenade`
//! struct (a projectile with velocity, gravity and a bounce counter) plus an `Explosion` (radius + damage + the
//! propagate step). The throw state (0x2D) and the `DroneAnim_EventFunc` datum 1 live in `drone.rs`.

use crate::world::{CollisionWorld, V3};
use crate::drone::Rng;

/// One grenade in flight (`Bullet_init` with a gravity-affected weapon_definition). Fields carry the game's `BU+0x…`
/// offsets where they match.
#[derive(Clone, Debug)]
pub struct Grenade {
    /// `obj+0x30`: position.
    pub pos: V3,
    /// `BU+0x00`: velocity (units per tick); gravity accumulates here.
    pub velocity: V3,
    /// `BU+0xF8`: speed (`weapon_data+0x20`).
    pub speed: f32,
    /// `BU+0xF4`: distance travelled; ranged-out grenades die.
    pub travelled: f32,
    /// `weapon_data+0x1C` / the per-type range (`Bullet_init`: 210 default, 420 for type 0x37, etc.).
    pub range: f32,
    /// `weapon_data+0x0C` × the explosion damage (`Explode_Propagate`'s `param_2`).
    pub damage: f32,
    /// `weapon_data+0x08`: the explosion radius (`Explode_Propagate`'s `param_1`).
    pub radius: f32,
    /// The weapon's type id (`weapon_data+0x02`). The disc name labels (Txt_BindLabel of
    /// weapon_data+0x38, resolved against USATxt.dat with the game's own fixup table) name them:
    /// 0x34 frag (8/100), 0x35 stun (10/0, the flash branch), 0x36 smoke (0/0, the occlude/re-arm
    /// branch), 0x37-0x39 remote mine (8/100), 0x3a laser tripbomb, 0x3b-0x40 satchel (8/2000) [L].
    pub weapon_type: u16,
    /// The drawn mesh (`weapon_data[w]+0x5C` of the weapon the throw used, `Bullet_update` state 0's
    /// `hashtable_set_object_to_entity_gfx(obj, weapon+0x5C)` disasm 0x12e8f0-0x12e900 [M]; 0 = none). The
    /// renderer poses it at the flight matrix every tick (`View_SetDrawInAllViews` in the game).
    pub mesh: u32,
    pub owner: usize,
    /// `BU+0xF4` state: 0 first tick, 1 flying, 2 dead (exploded).
    pub state: u8,
    /// `BU+0x10`: bounce counter (`Bullet_CollisionHandler`: max 0x14 = 20 bounces; 3 for ricochets).
    pub bounces: u8,
    /// `BU+0xF6` (the death-branch sub-state): the smoke grenade (0x36, `+0x70 & 0x20000` [L record
    /// 0x20280]) popped its `Occlude_Create` cloud on the first fuse expiry — the second expiry kills it
    /// [M Bullet_DoTrails 0x12e104-0x12e118: `lh +0xF6; bnel → the death end`].
    pub smoke_popped: bool,
    /// fix-grenade-throw: the slow-bounce latch — the game's zero-speed bullet never moves again
    /// [M 0x12ce80 tail zeroes the speed; `Bullet_update`'s `Vec_NormaliseLen` re-normalise (disasm
    /// 0x12f020-0x12f030) keeps the velocity at zero], so it never re-collides and never sounds again.
    pub resting: bool,
    /// fix-oddjob-hat: the +0x100 stick latch — Oddjob's hat (0x45, `+0x70 & 0x100` [L record
    /// 0x100]) STOPS at its first world contact: `Bullet_CollisionHandler` 0x12ce80:255 sets
    /// `obj+0xF4 = 3` (stuck), aligns the matrix to the negated hit normal [:256-266] and the
    /// handler's top skips every later tick (`if (obj+0xF4 != 3)` :102). The stuck countdown is
    /// hat-only [M `Bullet_update` case 3 0x12e7b0:363-371 — `weapon+2 != 0x45` skips it]. The
    /// mine 0x37 / tripbomb 0x3a / satchel 0x3b-0x40 records also carry +0x100 [L ee_start_05.bin
    /// sweep] — their stick branches (no 1500 re-arm, no countdown, trigger-detonated) are the
    /// satchel task's scope; the port sticks the hat only.
    pub stuck: bool,
    /// fix-oddjob-hat: `BU+0x40` — the hat's heat-seek target (`Bullet_homing` 0x12f358 stores the
    /// acquired object there; the collision handler's stick path reuses the slot for the object it
    /// stuck to [M 0x12ce80:244]). An index into `Drones::list`, `None` until acquired.
    pub seek_target: Option<usize>,
    /// fix-satchel-tripbomb: the stuck gadget's surface state (the satchel 0x3b / the tripbomb 0x3a;
    /// the hat keeps its normal-free path). `Bullet_CollisionHandler` 0x12ce80:256-266 aligns the
    /// bullet's matrix to the NEGATED hit normal and parks it `hit + normal·offset` along it (the
    /// offset: 0.04 generic / 0x3b [:257-258, 323], 0.002 for the 0x80000 tripbomb [:334, 369-381]).
    /// The renderer aligns the stuck model to `-normal` and the tripbomb's laser fires along `+normal`
    /// (the stick matrix Z = −normal [M :266]; the beam end = pos − 200·Z [M 0x12efb8-0x12efc8]).
    pub phase: Option<Box<StuckGadget>>,
}

/// fix-satchel-tripbomb: the stuck state of a +0x100 satchel/tripbomb (`Grenade.phase` — one box, the
/// shared-constructor merge lesson 4; only the satchel 0x3b and the tripbomb 0x3a carry it).
#[derive(Clone, Debug)]
pub struct StuckGadget {
    /// The surface normal the stick aligned to (HITDATA+0x10 [M 0x12ce80:256-266]).
    pub normal: V3,
    /// The tripbomb's post-stick arm countdown (`BU+0xFC` overwritten with 120.0 at the stick
    /// [M 0x12ce80:330]; `Bullet_update` case 3 counts it down 1/tick and runs the laser block at
    /// ≤ 0 [M 0x12e7b0 disasm 0x12efa4-0x12efc8 — the `+0x70 & 0x80000` arm]). The satchel never
    /// sets it (its fuse is `Grenade.travelled`, ticked by `Bullet_DoTrails`' +0x200 arm, which runs
    /// in state 3 too [M 0x12d890:262-285 — the decompile's `param_1 == 0` early-out tests the OBJ
    /// pointer (a0), not the 4th float arg: disasm 0x12efe8 `mov.s $f12, $f21` with $f21 = 0.0]).
    pub arm: f32,
    /// The laser armed (`Bullet_update` case 3: the beam object created once + Sound 0x27c
    /// [M 0x12ee30-0x12ee50]; `PositionBeam` 0x1a9400 stretches it every tick after).
    pub armed: bool,
    /// BU+0x100 — the displayed quarter-second latch (the 0x3b tail beeps only when it CHANGES
    /// [M `Bullet_DoTrails` 0x12d890:435-444]).
    pub last_qs: i16,
    /// The countdown display the 0x3b tail spawns every tick: two kind-0x40 objects gfx
    /// `SatchelDigits[sec/10]` / `SatchelDigits[sec%10]`, sec = `BU+0xFC`/60 [M 0x12d890:418-422,
    /// 485-553; L SatchelDigits @0x2bed90 = 0x2000791..0x200079a]. (tens, units), `None` before the
    /// first tail tick.
    pub digits: Option<(u8, u8)>,
}

/// What a grenade did this tick.
#[derive(Clone, Debug, PartialEq)]
pub enum GrenadeEvent {
    /// Bounced off the world at `pos` (sound 0x51C).
    Bounce(V3),
    /// fix-oddjob-hat: the hat (0x45) STUCK to the world at `pos` — the +0x100 arm's state 3
    /// [M 0x12ce80:255]; the caller plays the 0x4a7 contact clang [:317]. The object keeps
    /// existing (no bounce sound, no explosive broadcast).
    Stick(V3),
    /// Exploded at `pos` — the caller applies the explosion (radius, damage, owner) to drones and Bond.
    Explode { pos: V3, radius: f32, damage: f32, owner: usize, weapon_type: u16 },
    /// fix-satchel-tripbomb: the tripbomb's laser armed — the beam object was created and Sound 0x27c
    /// plays once [M `Bullet_update` case 3 0x12ee30-0x12ee50; the 0x27c sits inside the beam-create
    /// `if (*(int *)(pauVar1[3] + 8) == 0)` block at 0x12ee48].
    Arm(V3),
    /// fix-satchel-tripbomb: the satchel's countdown beep (`Bullet_DoTrails` 0x12d890:441-443 — Sound
    /// 0x447 when the quarter-second count CHANGES and `((qs & 3) == 0) || ((qs & 1) == 0 && sec < 10)
    /// || (sec < 5)`; the 300-tick satchel sits at sec ≤ 5, so every quarter-second beeps).
    Beep(V3),
}

/// The smoke grenade's cloud (`Occlude_Create(pos, 1200, kind)` 0x17ea80): a static class-0x2A object
/// whose glist is one mesh by kind — 0 = 0x200026b, 1 = 0x2000273, 2 = 0x2000274, 3 = 0x2000275 — and
/// whose only update is the lifetime countdown (`Occlude_Update` 0x17eba0: obj+0xF4 − 1 per tick; 0 →
/// obj+0xFE |= 1, dead). The smoke grenade 0x36 pops one with lifetime 1200, kind 1 at its first fuse
/// expiry [M Bullet_DoTrails 0x12e114-0x12e1b4 calls Occlude_Create(pos, 1200, 1)].
#[derive(Clone, Debug)]
pub struct SmokeCloud {
    /// `obj+0x30`: the cloud's position.
    pub pos: V3,
    /// The mesh the kind selects (`hashtable_set_object_to_entity_gfx` 0x17ea80).
    pub mesh: u32,
    /// `obj+0xF4`: the lifetime countdown (ticks; `Occlude_Update` 0x17eba0).
    pub ticks: i16,
}

/// `WldGravity` (VECTOR at 0x2d8890): `(0, −9.8, 0)`. [L] docs/systems/pickups.md, player.md.
pub(crate) const WLD_GRAVITY: V3 = [0.0, -9.8, 0.0];
/// `REC_FRAME_RATE` (0x30d0cc) = 1/60.
pub(crate) const REC_FRAME_RATE: f32 = 1.0 / 60.0;
/// `Bullet_CollisionHandler` bounce max: 0x14 (20) bounces.
const BOUNCE_MAX: u8 = 20;
/// `Bullet_CollisionHandler` min speed to keep bouncing: `FRAME_RATE_MUL · 0.01` = 0.01.
const BOUNCE_MIN_SPEED: f32 = 0.01;
/// fix-oddjob-hat: the hat's post-stick lifetime — the +0x100 arm writes 1500.0 (0x44160000) into
/// the fuse slot at the stick [M `Bullet_CollisionHandler` 0x12ce80:315]; `Bullet_update` case 3
/// counts it down for the 0x45 only and raises the delete flag at ≤ 0 [M 0x12e7b0:363-371].
const HAT_STICK_LIFE: f32 = 1500.0;

/// The per-material bounce coefficient table the bounce reads at `0x3037dc + material·0x88`
/// [M Bullet_CollisionHandler 0x12ce80: `fVar23 = *(float *)(BU+0xF8) * *(float *)lVar10`], measured
/// from ACTION.ELF [L]: 0.5 for materials 0..12 and 15..21, 0.0 for 13, 14 and 22 (the rest of the
/// 0x3f-masked id range holds unrelated data — denormals/pointers, not coefficients).
fn bounce_coefficient(material: u8) -> f32 {
    matches!(material, 13 | 14 | 22).then_some(0.0).unwrap_or(0.5)
}

impl Grenade {
    /// `Bullet_update` (0x12e7b0) for a gravity-affected grenade (`+0x6C & 0x8`): the first tick steps
    /// (0.5 + FRand 0.5) · speed [M the default case 0x12e7c0-0x12e828]; subsequent ticks apply
    /// `velocity += WldGravity · REC_FRAME_RATE²` and **`pos += velocity · FRAME_RATE_MUL`** — the game's
    /// velocity is the per-tick displacement (unit direction · speed, re-normalised after the gravity add
    /// with `Vec_NormaliseLen` storing the length back into BU+0xF8), and FRAME_RATE_MUL = 1 at the fixed
    /// 60 Hz [M the movement tail 0x12f040-0x12f058: `pos += fVar16 · velocity` with
    /// fVar16 = speed · FRAME_RATE_MUL = speed; docs/systems/input.md:126]. fix-player3: the port moved
    /// `pos += velocity · REC_FRAME_RATE` — 60× too slow, so a thrown grenade crawled at ~0.17 u/s and the
    /// 210-tick fuse expired at Bond's feet ("the throw doesn't really happen, no explosion").
    /// On a world hit: bounce (reflect the velocity by the surface normal, × the per-material
    /// coefficient 0x3037dc [L], up to 20 bounces) — a bounce NEVER explodes: a reflected speed below
    /// `FRAME_RATE_MUL·0.01` or the 20th bounce only zeroes the speed and the grenade sits until its
    /// tick fuse [M Bullet_CollisionHandler 0x12ce80 — the slow-bounce tail sets `BU+0xF8 = 0` with no
    /// `Explode_Create`, and the `(ushort)(count+1) < 0x14` else-branch likewise].
    /// Ranged-out grenades die (the fuse).
    pub fn update(&mut self, world: &CollisionWorld, rng: &mut Rng) -> Option<GrenadeEvent> {
        if self.state >= 2 { return None; }
        // fix-oddjob-hat: the stuck hat's tick [M `Bullet_update` case 3 0x12e7b0:285-371] — the
        // speed and the velocity are zeroed at the stick, no collision runs (the handler's
        // `obj+0xF4 != 3` gate 0x12ce80:102), the fuse slot counts down (hat-only :363) and the
        // delete flag rises at ≤ 0 (:369-370) — a SILENT death (`Bullet_Delete`'s effect kind
        // +0x70 & 7 = 0 spawns nothing [L the hat's +0x70 = 0x100]). The port re-arms `travelled`
        // to 0 at the stick (the game writes 1500.0 over the fuse slot, 0x12ce80:315) and counts
        // to `HAT_STICK_LIFE`.
        if self.stuck {
            match self.weapon_type {
                // The hat's silent post-stick lifetime (unchanged — fix-oddjob-hat).
                0x45 => {
                    self.travelled += 1.0;
                    if self.travelled >= HAT_STICK_LIFE { self.state = 2; }
                    return None;
                }
                // fix-satchel-tripbomb: the tripbomb's arm delay [M `Bullet_update` case 3, the
                // `+0x70 & 0x80000` arm 0x12efa4-0x12efc8 — `BU+0xFC` −= FRAME_RATE_MUL per stuck
                // tick, the laser block runs at ≤ 0]. 120.0 written at the stick [M 0x12ce80:330].
                0x3a => {
                    let Some(p) = self.phase.as_mut() else { return None };
                    if p.arm > 0.0 { p.arm -= 1.0; return None; }
                    if !p.armed {
                        p.armed = true;
                        return Some(GrenadeEvent::Arm(self.pos));
                    }
                    return None; // armed: the beam + the char trigger run in Drones::step (the drone list + Bond)
                }
                // fix-satchel-tripbomb: the satchel's fuse KEEPS TICKING while stuck —
                // `Bullet_DoTrails` runs in state 3 (Bullet_update 0x12efd8's call passes the step
                // 0.0, but its first gate tests the OBJ pointer a0 [M disasm 0x12d890-0x12d8c8], so
                // the +0x200 fuse arm [M 0x12d890:262-285] still counts BU+0xFC down) — fall through
                // to the shared fuse arm below with the movement skipped.
                0x3b => {}
                // fix-remote-mine-play: the mine's stuck fuse follows `Bullet_DoTrails` 0x12d890:268-273
                // [M]: the 0x37 arm skips the decrement ONLY while BU+0xFC == 7777.0 (the player-armed
                // wait-for-the-detonator state); ANY other fuse — a guard-thrown 420 [M 0x12c5a0:214-223]
                // or the detonator's mark (BU+0xFC = −10.0 [M disasm 0x1a4c88]) — keeps counting in
                // state 3 and detonates at < 0. The port's fuse model is `range − travelled`, so
                // "not 7777.0" falls through to the shared fuse arm below.
                0x37 => { if self.range >= 7777.0 { return None; } }
                _ => return None,
            }
        }
        // `Bullet_DoTrails` 0x12d890:268-273 [M]: weapon 0x37's fuse does NOT tick while it is the
        // player-armed 7777.0 — the mine waits for the detonator (`Player_WeaponInitBullet`'s
        // +0x68 & 0x20000 arm re-arms it to 10.0 [M 0x1a4bf0 disasm 0x1a4c88]); guard-thrown mines
        // keep the 420 fuse [M 0x12c5a0:196-223]. Only the fuse count freezes — the movement below
        // continues, exactly as the game's DoTrails skip sits beside the flight update.
        let armed_mine = self.weapon_type == 0x37 && self.range >= 7777.0;
        // A rested grenade skips everything but the fuse — the game's zero-speed bullet never moves
        // (the re-normalise pins the move at zero, disasm 0x12f020-0x12f030) so it never collides and
        // never sounds again [M 0x12ce80 tail + 0x12f040].
        if self.resting {
            if armed_mine { return None; }
            self.travelled += 1.0;
            if self.travelled > self.range {
                self.state = 2;
                return Some(GrenadeEvent::Explode { pos: self.pos, radius: self.radius, damage: self.damage, owner: self.owner, weapon_type: self.weapon_type });
            }
            return None;
        }
        // The fuse: BU+0xFC (`Bullet_init` 0x12c5a0): 210 default, 300 with `+0x70 & 0x10000`, the satchel
        // family (base 0x3b) ((id−0x3b)·5+5)·60 (0x6c → 900), 0x69 → 5, and the mine 0x37 → 420
        // (7777.0 when the owner is the player — trigger-detonated [M 0x12c5a0:214-223]; the port has no
        // detonator, so it keeps the guard-thrown 420). Count ticks, not distance —
        // `weapon_data+0x1C` (5000) is the max-travelled range, a SILENT state-4 death, not the fuse
        // [M Bullet_update case 1: travelled > +0x1C → state 4, no explosion].
        // fix-satchel-tripbomb: the fuse arm runs BEFORE the 0x3b tail (the game's `Bullet_DoTrails`
        // order: the +0x200 decrement 0x12d890:262-285, then the family tail :418+ reads the
        // DECREMENTED `BU+0xFC`).
        if !armed_mine {
            self.travelled += 1.0;
            if self.travelled > self.range {
                self.state = 2;
                // fix-weapon-fire-modes [M Bullet_update 0x12e7b0:158-162]: for a no-fuse record this
                // expiry is the RANGE-OUT arm — a SILENT state-4 death. The Sim's Explode arm gates on
                // `+0x70 & 0x200` (the `Bullet_DoTrails` fuse gate 0x12d890:262-281) and removes the
                // grenade quietly; a fuse grenade (0x2B, 0x34…) explodes here as before.
                return Some(GrenadeEvent::Explode { pos: self.pos, radius: self.radius, damage: self.damage, owner: self.owner, weapon_type: self.weapon_type });
            }
        }
        // fix-satchel-tripbomb: the satchel's countdown tail (`Bullet_DoTrails` 0x12d890:418-553) —
        // it runs for family 0x3b in EVERY state (the check sits behind the fuse block's merge label
        // 0x12e1f8, before the function tail), so the beeps start in flight. The quarter-second count
        // is `BU+0xFC · 0.06666667` (= /15), the seconds `· 0.016666668` (= /60) [M :421-422]; a beep
        // only when the quarter-second CHANGES from BU+0x100 and `((qs & 3) == 0) || (((qs & 1) == 0
        // && sec < 10)) || (sec < 5)` [M :441]; the two digit meshes re-spawn every tick [M :485-553].
        if self.weapon_type == 0x3b && self.state < 2 {
            let fuse = self.range - self.travelled;
            let qs = (fuse * 0.066_666_67) as i16;
            let sec = (fuse * 0.016_666_668) as i32;
            if let Some(p) = self.phase.as_mut() {
                if qs != p.last_qs {
                    p.last_qs = qs;
                    if (qs & 3) == 0 || ((qs & 1) == 0 && sec < 10) || sec < 5 {
                        return Some(GrenadeEvent::Beep(self.pos));
                    }
                }
                let sec = sec.max(0) as u8;
                p.digits = Some((sec / 10, sec % 10));
            }
        }
        // The stuck satchel stops here: `Bullet_update` case 3 zeroes the speed and the velocity
        // [M 0x12e7b0 case 3 — `*(pauVar1[0xf] + 8) = 0; Vec_Zero(pauVar1)`] and only the fuse tail
        // above runs (no movement, no collision).
        if self.stuck { return None; }
        // First tick: half-speed (the game's `Rand_FRand(0.5) + 0.5` step [M 0x12e7c0]).
        let step = if self.state == 0 { rng.frand(0.5) + 0.5 } else { 1.0 };
        self.state = 1;
        // Gravity: `velocity += WldGravity · REC_FRAME_RATE²` (the game's `FRAME_RATE_DIV · REC_FRAME_RATE²` with
        // `FRAME_RATE_DIV = REC_FRAME_RATE = 1/60`).
        self.velocity[0] += WLD_GRAVITY[0] * REC_FRAME_RATE * REC_FRAME_RATE;
        self.velocity[1] += WLD_GRAVITY[1] * REC_FRAME_RATE * REC_FRAME_RATE;
        self.velocity[2] += WLD_GRAVITY[2] * REC_FRAME_RATE * REC_FRAME_RATE;
        // `pos += velocity · step` (the game's per-tick displacement — see the doc comment above).
        let dt = step;
        let end = [self.pos[0] + self.velocity[0] * dt, self.pos[1] + self.velocity[1] * dt, self.pos[2] + self.velocity[2] * dt];
        // Collision: the segment from pos to end against the world.
        // A sitting grenade (speed zeroed by a slow bounce) skips the collision — the game's zero-length
        // move hits nothing and the fuse keeps counting [M 0x12ce80 tail].
        let speed2 = self.velocity[0] * self.velocity[0] + self.velocity[1] * self.velocity[1] + self.velocity[2] * self.velocity[2];
        // satchel-stick2: `Bullet_update`'s world hittest has TWO forms [M 0x12e7b0:425-465 — the
        // HITTEST build over the pre-move obj+0x30], picked by the move length against the probe
        // radius `max(0.25, obj+0x8C)` (the grenade family keeps the 0.25 floor, 0x3e800000; only
        // the MGL family 0x2c..0x31 / 0x62 / 0x6d zeroes it, :427-429):
        // - move < radius·0.5 → version 0x101: a one-sided sphere of that radius AT THE CURRENT
        //   position (0x1e92d8 `Intersect_PointGeom` → `Dist2Tri` 0x1dfca8: the closest point on
        //   the triangle wins, faces the centre is behind are skipped);
        // - move ≥ radius·0.5 → version 0x201: a ray of `max(1.0, move)` [M :443-465 —
        //   `uVar19 = max(1.0, fVar16)`], i.e. up to 1 u of look-ahead.
        // The satchel's speed is 0.0667 u/tick [L ee_start_05.bin weapon_data 0x3b +0x20], so its
        // whole 1-2 m approach runs in the 0x101 sphere form: the game PARKS the satchel the
        // moment a face comes within 0.25 u — grazing the wall top, hugging the wall, or a
        // close-range throw all stick where the old port's 0.067 u ray sailed past (the playtest's
        // "even standing directly at a wall and throwing it at it, it still bounced": no ray
        // crossing, no stick, the satchel flies on). Scoped to the +0x100 sticky family this arm
        // serves (0x37/0x3a/0x3b — the hat 0x45 keeps its verified path); the bouncing family
        // (0x34…) keeps the old move-length ray — the game gives it the same two forms [S backlog:
        // a 1.0 u look-ahead would shift every pinned frag bounce point].
        // grenade-rattle: the two forms ARE every bullet's in the game [M 0x12e7b0:427-465 — the
        // family test only zeroes the probe radius (the MGL 0x2c..0x31/0x62/0x6d → always the ray);
        // the 0x201 ray's length is `max(1.0, move)` [M :443-465]]. NOT landed here: the look-ahead
        // shifts the pinned blast geometry (fix_grenade_types_live's stun whiteout pins the old
        // landing; the re-pin needs the game's own landing measured live — see the report's live
        // checks). The bouncing family keeps the move-length ray for now; the settle-sound mechanics
        // below (the dot gate, the cap, the park order) are the rattle's fix.
        let sticky = matches!(self.weapon_type, 0x37 | 0x3a | 0x3b);
        const PROBE_RADIUS: f32 = 0.25;
        let world_hit = if !sticky {
            world.segment_surface(self.pos, end, false)
                .map(|(t, normal, mat)| (t, [self.pos[0] + (end[0] - self.pos[0]) * t, self.pos[1] + (end[1] - self.pos[1]) * t, self.pos[2] + (end[2] - self.pos[2]) * t], normal, mat))
        } else {
            let speed = speed2.sqrt();
            let move_len = speed * dt;
            if move_len < PROBE_RADIUS * 0.5 {
                world.sphere_surface(self.pos, PROBE_RADIUS, false)
                    .map(|(q, normal, mat, _)| (0.0f32, q, normal, mat))
            } else {
                let dir = [self.velocity[0] / speed, self.velocity[1] / speed, self.velocity[2] / speed];
                let len = 1.0f32.max(move_len);
                let far = [self.pos[0] + dir[0] * len, self.pos[1] + dir[1] * len, self.pos[2] + dir[2] * len];
                world.segment_surface(self.pos, far, false)
                    .map(|(t, normal, mat)| (t, [self.pos[0] + (far[0] - self.pos[0]) * t, self.pos[1] + (far[1] - self.pos[1]) * t, self.pos[2] + (far[2] - self.pos[2]) * t], normal, mat))
            }
        };
        if speed2 > 1e-12 {
            if let Some((_t, hit, normal, mat)) = world_hit {
                // fix-oddjob-hat: Oddjob's hat (0x45, `+0x70 & 0x100` [L record]) never bounces off
                // the world — the +0x100 arm STICKS it: `obj+0xF4 = 3` [M 0x12ce80:255], the matrix
                // aligns to the negated hit normal [:256-266] and the hat alone gets the 1500.0
                // post-stick lifetime + the 0x4a7 clang [:315-317]. The bounce block (LAB_0012d4bc)
                // is unreachable for a +0x100 record's world hit: its gate needs `+0x70 & 0x80` or
                // the kind-2/3/0x15 hit flag (iStack_100) [:398-407] and the world path sets
                // neither. The port freezes the velocity and reports Stick; the countdown runs in
                // the stuck branch above. (The mine/satchel records also carry +0x100 — their
                // branches are the satchel task's scope, see the `stuck` field.)
                if self.weapon_type == 0x45 {
                    self.pos = hit;
                    self.velocity = [0.0; 3];
                    self.stuck = true;
                    self.travelled = 0.0;
                    return Some(GrenadeEvent::Stick(hit));
                }
                // fix-satchel-tripbomb: the +0x100 stick for the satchel 0x3b and the tripbomb 0x3a
                // [M `Bullet_CollisionHandler` 0x12ce80 — the sticky arm :232-394 runs on a world hit
                // (bVar7 == 0; a char hit bounces :239-249), sets `obj+0xF4 = 3` :255, aligns the
                // matrix to the NEGATED hit normal :256-266 and parks at `hit + normal·offset`:
                // the 0x3b arm offset uVar12 = 0.04 + Sound 0x4a7 [:257-258, 319-323]; the 0x80000
                // tripbomb arm offset 0.002 + Sound 0x446 + `BU+0xFC = 120.0` [:325-334]]. The tripbomb's
                // 6-placed cap walk [:335-370] runs in the Sim's Stick consumer.
                // fix-remote-mine-play: the mine 0x37's own stick branch [M :296-330] — the record's
                // +0x70 = 0x18300 carries 0x100 [L ee_snapshot.bin @ 0x2bf150 + 0x37·0x10C + 0x70], the
                // walk matches by the record's base id (+0x02 == 0x37): count the same-weapdef
                // same-owner type-5 bullets (in flight AND stuck — no state filter in the walk),
                // `bVar4 = count < 6`, past the cap mark the OLDEST (min obj+0xEC [*suggested: the
                // port's spawn order]) `+0xFE |= 1` and still park the new one; uVar13 stays 0.04
                // (the mine never rewrites it) so the park is `hit + normal·0.04` [:345-352]. The
                // arm writes NO fuse (BU+0xFC keeps Bullet_init's 7777.0 player / 420 guard [M
                // 0x12c5a0:214-223]) and plays NO sound (no Sound_Play3D in the 0x37 arm — the
                // Sim's Stick consumer mutes it; 0x3b/0x45 play 0x4a7, 0x3a plays 0x446).
                if matches!(self.weapon_type, 0x37 | 0x3a | 0x3b) {
                    let off = if self.weapon_type == 0x3a { 0.002f32 } else { 0.04 };
                    let stuck_at = [hit[0] + normal[0] * off, hit[1] + normal[1] * off, hit[2] + normal[2] * off];
                    self.pos = stuck_at;
                    self.velocity = [0.0; 3];
                    self.stuck = true;
                    self.resting = false;
                    // satchel-stick2: the countdown latch (BU+0x100) and the digit meshes live in
                    // BU, not the stick — `Bullet_CollisionHandler` writes only obj+0xF4/0x90/0x30
                    // [M 0x12ce80:255-266 + the 0x12d478 park]; `Bullet_DoTrails`' 0x3b tail keeps
                    // reading the same BU fields through state 3. The old reset to (0, None) opened
                    // a one-tick digit gap at the stick (digits None → the kinds-4/5 slots FAR →
                    // the numbers blink off exactly when the satchel parks).
                    let (last_qs, digits) = self.phase.as_ref().map_or((0, None), |p| (p.last_qs, p.digits));
                    self.phase = Some(Box::new(StuckGadget {
                        normal,
                        arm: if self.weapon_type == 0x3a { 120.0 } else { 0.0 },
                        armed: false,
                        last_qs,
                        digits,
                    }));
                    return Some(GrenadeEvent::Stick(stuck_at));
                }
                // fix-weapon-fire-modes [M Bullet_CollisionHandler 0x12ce80:194-206]: a `+0x70 & 0x4`
                // record's round detonates at the contact — the Bounce event carries it to the Sim's
                // Bounce arm, which reads the record's flags and blasts instead of bouncing (the MGL's
                // 'Grenade' 0x2A [L ee_start_05.bin]; its 'Delay' twin 0x2B bounces to the fuse).
                // Bounce: reflect the velocity by the surface normal, × the material coefficient [L 0x3037dc].
                // grenade-rattle: the reflect arm is DOT-GATED [M 0x12ce80 0x12d5f0-0x12d600 —
                // `fVar23 = 2·(v·n); if (fVar23 <= 0.0) { reflect; Normalise; speed ×= coeff }`]: a
                // contact the grenade is already SEPARATING from (the 1 u look-ahead ray's hit while
                // climbing) parks and counts but leaves the velocity/speed alone — the port's
                // unconditional reflect flipped such a contact back INTO the surface and re-collided
                // every tick (the playtests' "rapidly contacting the ground"). The cap [M 0x12d578-
                // 0x12d57c — `(ushort)(counter+1) < 0x14` gates the whole arm]: at 20 bounces the
                // handler parks and counts but does NOT reflect or scale — the port's old `resting`
                // freeze at the cap is gone.
                // The park BEFORE the reflect (the game's order: LAB_0012d478's park, then the sound
                // block, then the counter, then the dot-gated reflect): `BU+0x30 = *(HITDATA+0x40) +
                // *(HITDATA+0x10) · 0.04` — the hit point offset ALONG the hit normal by 0x3d23d70a
                // (set at 0x12ce80:257-258 for the generic bounce arms; the contact-detonation arm's
                // 0.002 differs only on the MGL's Explode position, folded in the Sim's blast).
                // Without it the grenade rests exactly ON the collision triangle and every later
                // sight/LOS ray to it grazes the surface at the ray's end — the ai_live grenade-scare
                // guard never saw the bounced frag. The 1 u look-ahead ray's hit point may sit ahead
                // of the tick's move — the game parks AT the contact (the look-ahead is the game's).
                let off = 0.04f32;
                let hit = [hit[0] + normal[0] * off, hit[1] + normal[1] * off, hit[2] + normal[2] * off];
                self.pos = hit;
                self.bounces = self.bounces.saturating_add(1);
                let v = self.velocity;
                let dot = v[0] * normal[0] + v[1] * normal[1] + v[2] * normal[2];
                if self.bounces < BOUNCE_MAX && dot <= 0.0 {
                    let reflected = [v[0] - 2.0 * dot * normal[0], v[1] - 2.0 * dot * normal[1], v[2] - 2.0 * dot * normal[2]];
                    let c = bounce_coefficient(mat);
                    self.velocity = [reflected[0] * c, reflected[1] * c, reflected[2] * c];
                    // The game's rest: the slow-bounce tail zeroes the speed [M 0x12ce80 tail] and
                    // `Bullet_update`'s re-normalise (`Vec_NormaliseLen` stores the length back, disasm
                    // 0x12f020-0x12f030) keeps a zero-speed bullet's move at zero — a rested grenade
                    // never re-collides. fix-grenade-throw: the port's velocity kept re-accumulating
                    // gravity at rest, so the grenade re-collided EVERY tick (the user's "rapid sound
                    // as if it's rapidly contacting the ground").
                    let speed = (self.velocity[0] * self.velocity[0] + self.velocity[1] * self.velocity[1] + self.velocity[2] * self.velocity[2]).sqrt();
                    if speed < BOUNCE_MIN_SPEED {
                        self.velocity = [0.0; 3];
                        self.resting = true;
                    }
                }
                return Some(GrenadeEvent::Bounce(hit));
            }
        }
        self.pos = end;
        None
    }
}

/// The result of an explosion's damage application (`Explode_Propagate` 0x17c518): who was hit and for how much.
#[derive(Clone, Debug, Default)]
pub struct ExplosionHits {
    /// (drone index, damage) — `Drone_ExplosiveHit` → msg 9.
    pub drones: Vec<(usize, f32)>,
    /// Damage to Bond (`Player_Hurt`).
    pub player: f32,
    /// The explosion's position (for the sound and the camera shake).
    pub pos: V3,
}

/// `Explode_Propagate` (0x17c518): a sphere intersect of `radius` from `pos`. For each hit object, `damage = amount ·
/// (1 − dist / radius)` (clamped to ≥ 0.0002). Drones (type 2) take `Drone_ExplosiveHit` (msg 9); Bond (type 3) takes
/// `Player_Hurt`. The min cutoff 0.0002 means distant objects take no damage. [M] linear falloff, min 0.0002.
pub fn explode_propagate(
    pos: V3,
    radius: f32,
    damage: f32,
    drones: &[(usize, V3)],
    player: V3,
    _player_height: f32,
    difficulty: u32,
) -> ExplosionHits {
    let mut hits = ExplosionHits { pos, ..Default::default() };
    if radius < 0.0002 || damage < 0.0002 { return hits; }
    // Bond: a cylinder from feet to feet + height (the game tests 16 boxes; the port uses one cylinder).
    let pdy = player[1] - pos[1];
    let pdx = player[0] - pos[0];
    let pdz = player[2] - pos[2];
    let pdist = (pdx * pdx + pdy * pdy + pdz * pdz).sqrt();
    if pdist < radius {
        let d = damage * (1.0 - pdist / radius);
        // `Player_Hurt` applies the level's damage factor (`Plr_DMod_*`); 07000005 difficulty 1 → 0.6.
        if d >= 0.0002 { hits.player = d * crate::bullet::damage_factor(0x0700_0005, difficulty); }
    }
    for &(i, dpos) in drones {
        let dx = dpos[0] - pos[0];
        let dy = dpos[1] - pos[1];
        let dz = dpos[2] - pos[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist < radius {
            let d = damage * (1.0 - dist / radius);
            if d >= 0.0002 { hits.drones.push((i, d)); }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor quad at y = 0 (large enough that a thrown grenade stays above it while it falls).
    fn floor_world() -> CollisionWorld {
        CollisionWorld::from_triangles(vec![
            [[-100.0, 0.0, -100.0], [100.0, 0.0, 100.0], [100.0, 0.0, -100.0]],
            [[-100.0, 0.0, -100.0], [-100.0, 0.0, 100.0], [100.0, 0.0, 100.0]],
        ])
    }

    #[test]
    fn grenade_falls_under_gravity_and_bounces() {
        let w = floor_world();
        // A grenade thrown downward at 5 u/tick from 2 units up: it reaches the floor and bounces before the 210-tick
        // fuse expires.
        let mut g = Grenade {
            pos: [0.0, 2.0, 0.0], velocity: [5.0, -5.0, 0.0], speed: 7.0, travelled: 0.0, range: 210.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x34, mesh: 0x0200_02af, owner: 0, state: 0, bounces: 0, smoke_popped: false, resting: false,
            stuck: false, seek_target: None,
            phase: None,
        };
        let mut rng = Rng::new(0);
        let mut bounced = false;
        for k in 0..2000 {
            if let Some(ev) = g.update(&w, &mut rng) {
                if matches!(ev, GrenadeEvent::Bounce(_)) { bounced = true; break; }
                if matches!(ev, GrenadeEvent::Explode { .. }) { eprintln!("exploded at tick {k} pos={:?}", g.pos); break; }
            }
        }
        assert!(bounced, "grenade bounced off the floor");
    }

    #[test]
    fn grenade_sits_after_max_bounces_and_explodes_only_at_the_fuse() {
        let w = floor_world();
        // fix-weapons-all [M Bullet_CollisionHandler 0x12ce80]: the 20th bounce does NOT explode — it
        // zeroes the speed and the grenade sits until its tick fuse. (The old port exploded here:
        // "it blows up immediately as soon as it hits the ground".)
        let mut g = Grenade {
            pos: [0.0, 0.5, 0.0], velocity: [0.0, -30.0, 0.0], speed: 30.0, travelled: 190.0, range: 200.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x34, mesh: 0x0200_02af, owner: 0, state: 0, bounces: BOUNCE_MAX, smoke_popped: false, resting: false,
            stuck: false, seek_target: None,
            phase: None,
        };
        let mut rng = Rng::new(0);
        let mut bounced = false;
        for _ in 0..5 {
            if let Some(ev) = g.update(&w, &mut rng) {
                assert!(matches!(ev, GrenadeEvent::Bounce(_)), "at the bounce max: a bounce, got {ev:?}");
                bounced = true; break;
            }
        }
        assert!(bounced, "the 20th contact still bounces (sits)");
        assert_eq!(g.velocity, [0.0, 0.0, 0.0], "the speed zeroes at the bounce max [M 0x12ce80]");
        assert_eq!(g.state, 1, "still alive — the fuse explodes it");
        // The fuse: 10 more ticks (travelled 190 + 10 > 200) → the explosion.
        let mut exploded = false;
        for _ in 0..12 {
            if let Some(GrenadeEvent::Explode { .. }) = g.update(&w, &mut rng) { exploded = true; break; }
        }
        assert!(exploded, "the sitting grenade explodes at its fuse");
        // Sister: one bounce short (19) with a fast reflection it keeps flying.
        let mut g = Grenade {
            pos: [0.0, 10.0, 0.0], velocity: [0.0, -30.0, 0.0], speed: 30.0, travelled: 0.0, range: 200.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x34, mesh: 0x0200_02af, owner: 0, state: 0, bounces: BOUNCE_MAX - 1, smoke_popped: false, resting: false,
            stuck: false, seek_target: None,
            phase: None,
        };
        if let Some(ev) = g.update(&w, &mut rng) { assert!(matches!(ev, GrenadeEvent::Bounce(_)), "one short: a bounce, got {ev:?}"); }
        else { assert!(g.bounces == BOUNCE_MAX, "the bounce counted"); }
    }

    /// fix-weapons-all [M Bullet_CollisionHandler 0x12ce80 tail]: a reflected speed below
    /// `FRAME_RATE_MUL · 0.01` zeroes the speed — the grenade sits; it does NOT explode.
    #[test]
    fn grenade_never_explodes_from_a_slow_bounce() {
        let w = floor_world();
        let mut g = Grenade {
            pos: [0.0, 0.05, 0.0], velocity: [0.01, -0.05, 0.0], speed: 0.05, travelled: 0.0, range: 300.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x34, mesh: 0x0200_02af, owner: 0, state: 1, bounces: 0, smoke_popped: false, resting: false,
            stuck: false, seek_target: None,
            phase: None,
        };
        let mut rng = Rng::new(0);
        let mut events = Vec::new();
        for _ in 0..100 { if let Some(ev) = g.update(&w, &mut rng) { events.push(ev); } }
        assert!(events.iter().any(|e| matches!(e, GrenadeEvent::Bounce(_))), "it touches the floor");
        assert!(!events.iter().any(|e| matches!(e, GrenadeEvent::Explode { .. })),
            "a slow bounce must sit, not explode [M 0x12ce80]");
        assert_eq!(g.velocity, [0.0; 3], "the slow bounce zeroes the speed");
    }

    #[test]
    fn explode_propagate_linear_falloff() {
        // A grenade at the origin, radius 4, damage 100. Bond at distance 0 takes 100; at distance 2 takes 50; at
        // distance 4 takes 0 (the cutoff); a drone at distance 1 takes 75.
        let hits = explode_propagate([0.0, 1.0, 0.0], 4.0, 100.0, &[(0, [1.0, 1.0, 0.0])], [0.0, 1.0, 0.0], 1.8, 1);
        // Bond at the centre: damage 100 × the difficulty factor (0.6 on 07000005 difficulty 1) = 60.
        assert!((hits.player - 100.0 * 0.6).abs() < 1e-3, "player at centre: {}", hits.player);
        // Drone at distance 1: damage 100 × (1 − 1/4) = 75.
        assert!((hits.drones[0].1 - 75.0).abs() < 1e-3, "drone at distance 1: {}", hits.drones[0].1);
        // Sister: a target beyond the radius takes nothing.
        let hits2 = explode_propagate([0.0, 1.0, 0.0], 4.0, 100.0, &[(0, [10.0, 1.0, 0.0])], [10.0, 1.0, 0.0], 1.8, 1);
        assert!(hits2.drones.is_empty(), "drone beyond radius: no damage");
        assert!((hits2.player - 0.0).abs() < 1e-6, "player beyond radius: no damage");
    }

    /// A wall quad at z = 0 facing +z (the floor of `floor_world` keeps the grenade from falling out).
    fn wall_world() -> CollisionWorld {
        CollisionWorld::from_triangles(vec![
            [[-100.0, 0.0, -100.0], [100.0, 0.0, 100.0], [100.0, 0.0, -100.0]],
            [[-100.0, 0.0, -100.0], [-100.0, 0.0, 100.0], [100.0, 0.0, 100.0]],
            // The wall: z = 0 plane, y from 0 to 4 (two triangles).
            [[-100.0, 0.0, 0.0], [-100.0, 4.0, 0.0], [100.0, 4.0, 0.0]],
            [[-100.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 4.0, 0.0]],
        ])
    }

    fn satchel() -> Grenade {
        Grenade {
            pos: [0.0, 1.0, 2.0], velocity: [0.0, 0.0, -1.0], speed: 1.0, travelled: 0.0, range: 300.0,
            damage: 2000.0, radius: 8.0, weapon_type: 0x3b, mesh: 0x0200_07b3, owner: usize::MAX, state: 0,
            bounces: 0, smoke_popped: false, resting: false, stuck: false, seek_target: None, phase: None,
        }
    }

    /// fix-satchel-tripbomb [M `Bullet_CollisionHandler` 0x12ce80:232-394 + `Bullet_DoTrails`
    /// 0x12d890:262-285]: the satchel (0x3b, `+0x70 & 0x100` sticky [L record] + the 300-tick family
    /// fuse [M 0x12c5a0:199-208]) STICKS at its first world contact — state 3, the phase carries the
    /// hit normal and the 0.04 offset parks it just off the surface — and blows on the 301st tick
    /// FROM THE THROW (the fuse ticks through the flight too). Sister: the fuse total is 300, not
    /// the port's old default 210 (`Bullet_init`'s family arm, ((0x3b − 0x3b)·5 + 5)·60).
    #[test]
    fn satchel_sticks_at_the_wall_and_blows_at_tick_301() {
        let w = wall_world();
        let mut g = satchel();
        let mut rng = Rng::new(0);
        let mut stuck_at = None;
        for t in 0..40 {
            if let Some(ev) = g.update(&w, &mut rng) {
                if matches!(ev, GrenadeEvent::Stick(_)) { stuck_at = Some(t); break; }
                panic!("the satchel should stick first, got {ev:?}");
            }
        }
        let stuck_tick = stuck_at.expect("the satchel sticks at the wall");
        assert!(g.stuck, "the stick latch");
        let p = g.phase.as_ref().expect("the phase box");
        assert!((p.normal[2] - 1.0).abs() < 1e-4, "the normal faces +z (out of the wall): {:?}", p.normal);
        assert!((g.pos[2] - 0.04).abs() < 1e-3, "parked 0.04 off the wall [M 0x12ce80:323]: {}", g.pos[2]);
        assert_eq!(g.velocity, [0.0, 0.0, 0.0], "the stick zeroes the velocity [M case 3]");
        // The beeps: every quarter-second change beeps at sec < 5 [M 0x12d890:441] — the first tail
        // tick (fuse 299 → qs 19 ≠ the 0 latch, sec 4 < 5) beeps on the throw's FIRST tick.
        let mut beeps = Vec::new();
        let mut exploded = None;
        for t in stuck_tick + 1..400 {
            match g.update(&w, &mut rng) {
                Some(GrenadeEvent::Beep(_)) => beeps.push(t),
                Some(GrenadeEvent::Explode { radius, damage, .. }) => {
                    exploded = Some(t);
                    assert!((radius - 8.0).abs() < 1e-4 && (damage - 2000.0).abs() < 1e-3,
                        "the satchel blast is the record's 8/2000 [L], got {radius}/{damage}");
                    break;
                }
                _ => {}
            }
        }
        let boom = exploded.expect("the stuck satchel explodes at its fuse");
        assert_eq!(boom, 300, "the fuse is 300 ticks FROM THE THROW (the 301st update, 0-indexed t=300), got {boom}");
        assert!(!beeps.is_empty(), "the countdown beeps while stuck");
        assert!((beeps[0] as i32 - stuck_tick as i32 - 1).abs() <= 1, "the first beep is the first stuck tail tick");
        // The digits: sec = fuse/60 — just after the stick (fuse ~296) the display reads 04, and it
        // has reached 00 by the blast [M 0x12d890:421-422, 485-553].
        let p = g.phase.as_ref().unwrap();
        assert_eq!(p.digits, Some((0, 0)), "the display counts down to 00");
    }

    /// Sister: a 210-tick fuse would have blown 91 ticks early — the pre-fix port's default. The
    /// satchel still alive at tick 300 (pre-fix it exploded at 301 with fuse 210 → gone) proves the
    /// family arm [M 0x12c5a0:199-208].
    #[test]
    fn satchel_fuse_is_not_the_default_210() {
        let w = wall_world();
        let mut g = satchel();
        let mut rng = Rng::new(0);
        for _ in 0..40 { if g.update(&w, &mut rng).is_some() { break; } }
        assert_eq!(g.range, 300.0, "the satchel's fuse is the family arm's 300, not 210");
        for _ in 0..250 { g.update(&w, &mut rng); }
        assert!(g.state < 2, "alive past the old 210-tick mark (the family fuse)");
    }

    /// fix-satchel-tripbomb [M `Bullet_CollisionHandler` 0x12ce80:325-334 + `Bullet_update` case 3
    /// 0x12efa4-0x12efc8]: the tripbomb (0x3a, sticky + the 0x80000 laser bit [L record 0x80100])
    /// sticks flush (0.002), the stick overwrites the fuse slot with 120.0, and the laser arms on
    /// the 120th stuck tick. Sister: it does NOT fuse-explode (the record carries no +0x200).
    #[test]
    fn tripbomb_arms_120_ticks_after_the_stick_and_never_fuse_explodes() {
        let w = wall_world();
        let mut g = Grenade {
            pos: [0.0, 1.0, 2.0], velocity: [0.0, 0.0, -1.0], speed: 1.0, travelled: 0.0, range: 42857.0,
            damage: 100.0, radius: 8.0, weapon_type: 0x3a, mesh: 0x0200_074f, owner: usize::MAX, state: 0,
            bounces: 0, smoke_popped: false, resting: false, stuck: false, seek_target: None, phase: None,
        };
        let mut rng = Rng::new(0);
        let mut stuck_tick = None;
        for t in 0..40 {
            if matches!(g.update(&w, &mut rng), Some(GrenadeEvent::Stick(_))) { stuck_tick = Some(t); break; }
        }
        let stuck_tick = stuck_tick.expect("the tripbomb sticks");
        {
            let p = g.phase.as_ref().unwrap();
            assert!((p.arm - 120.0).abs() < 1e-4, "the stick writes the 120-tick arm [M 0x12ce80:330]");
            assert!((g.pos[2] - 0.002).abs() < 1e-4, "parked 0.002 off the wall [M 0x12ce80:381]");
        }
        let mut armed_at = None;
        for t in stuck_tick + 1..stuck_tick + 200 {
            match g.update(&w, &mut rng) {
                Some(GrenadeEvent::Arm(_)) => { armed_at = Some(t); break; }
                Some(ev) => panic!("no event before the arm, got {ev:?}"),
                None => {}
            }
        }
        assert_eq!(armed_at, Some(stuck_tick + 121), "the laser arms the tick after the 120th decrement [M case 3]");
        assert!(g.phase.as_ref().unwrap().armed, "the armed latch");
        // Sister: no fuse explosion — 2000 more ticks stay silent (the record has no +0x200 [L]).
        for _ in 0..2000 { assert!(g.update(&w, &mut rng).is_none(), "the tripbomb never fuse-explodes"); }
        assert!(g.state < 2, "still alive");
    }

    /// fix-satchel-stick-play, the first-tick contact [M `Bullet_CollisionHandler` 0x12ce80 — the
    /// sticky arm runs on the move's HITDATA, and the move's hit at t=0 (spawned already touching
    /// the surface, moving into it) is a hit like any other]: a satchel spawned ON the wall face
    /// moving into it sticks ON THE FIRST update — no bounce, no flight. Sister: spawned just
    /// INSIDE the face (beyond it) the one-sided move ray misses the entry face — the game's own
    /// `Intersect_RayGeom` 0x10a8c8:150-168 skips `n·dir >= 0` — so it keeps flying and sticks on
    /// the floor below instead; the port matches the game (no fix, the shared shape is documented).
    #[test]
    fn satchel_spawned_touching_the_wall_sticks_on_the_first_tick() {
        let w = wall_world();
        let mut g = satchel();
        g.pos = [0.0, 1.0, 0.0]; // ON the z = 0 wall face, moving −z (into it)
        let mut rng = Rng::new(0);
        let ev = g.update(&w, &mut rng);
        assert!(matches!(ev, Some(GrenadeEvent::Stick(_))), "the touching spawn sticks at once, got {ev:?}");
        assert!(g.stuck && g.velocity == [0.0, 0.0, 0.0], "stuck, velocity zeroed");
        let p = g.phase.as_ref().expect("the phase box");
        assert!((p.normal[2] - 1.0).abs() < 1e-4, "the wall's +z normal");
    }

    #[test]
    fn satchel_spawned_inside_the_wall_skips_the_entry_face_like_the_game() {
        let w = wall_world();
        let mut g = satchel();
        g.pos = [0.0, 1.0, -0.01]; // just INSIDE the wall, moving −z (deeper)
        let mut rng = Rng::new(0);
        let mut stuck_normal = None;
        for _ in 0..200 {
            if g.stuck { stuck_normal = Some(g.phase.as_ref().unwrap().normal); break; }
            g.update(&w, &mut rng);
        }
        let n = stuck_normal.expect("it sticks somewhere (the floor) within 200 ticks");
        assert!(n[1] > 0.9, "NOT on the wall's entry face — the one-sided ray skips it (the game's shape, 0x10a8c8:150-168); normal {n:?}");
    }

    /// satchel-stick2 [M `Bullet_update` 0x12e7b0:425-444 — the slow-move hittest is the 0x101
    /// sphere of radius max(0.25, obj+0x8C) AT THE CURRENT POSITION, `Intersect_PointGeom`
    /// 0x1e92d8 + `Dist2Tri` 0x1dfca8; the satchel's 0.0667 u/tick throw [L ee_start_05.bin
    /// weapon_data 0x3b +0x20] never leaves that form until gravity builds |v| past 0.125]: a
    /// satchel flying PARALLEL to a face within 0.25 u of it sticks on the first tick — there is
    /// no ray crossing to wait for. The old port ran a move-length ray and flew on (the playtest's
    /// "it still bounced": the throw passes just over/along the wall and nothing catches it).
    #[test]
    fn satchel_flying_along_a_wall_within_the_probe_radius_sticks_at_once() {
        let w = wall_world();
        let mut g = satchel();
        g.pos = [0.0, 1.0, 0.2]; // 0.2 u off the z = 0 wall face, moving ALONG it (−x)
        g.velocity = [-0.0666_6667, 0.0, 0.0];
        g.speed = 0.0666_6667;
        let mut rng = Rng::new(0);
        let mut stuck_at = None;
        for t in 0..3 {
            if let Some(ev) = g.update(&w, &mut rng) {
                if matches!(ev, GrenadeEvent::Stick(_)) { stuck_at = Some(t); break; }
            }
        }
        assert!(g.stuck, "the 0.2 u graze sticks within 3 ticks (the game's 0.25 probe)");
        assert_eq!(stuck_at, Some(0), "on the FIRST update");
        let p = g.phase.as_ref().expect("the phase box");
        assert!((p.normal[2] - 1.0).abs() < 1e-4, "the wall's +z normal, got {:?}", p.normal);
        // The park: Dist2Tri's closest point (z = 0) + normal·0.04 [M 0x12ce80:383-392].
        assert!((g.pos[2] - 0.04).abs() < 1e-4, "parked 0.04 off the face, got z {}", g.pos[2]);
        assert!((g.pos[1] - 1.0).abs() < 1e-4, "the closest point keeps the flight height");
    }

    /// Sister: the same probe catches a satchel descending onto a floor it has not reached yet —
    /// the game parks it 0.25 u early (the graze-over-the-wall-top case).
    #[test]
    fn satchel_descending_onto_the_floor_within_the_probe_radius_sticks_early() {
        let w = wall_world();
        let mut g = satchel();
        g.pos = [0.0, 0.2, 2.0]; // 0.2 u above the y = 0 floor, drifting along it
        g.velocity = [-0.0666_6667, 0.0, 0.0];
        g.speed = 0.0666_6667;
        let mut rng = Rng::new(0);
        for _ in 0..3 { g.update(&w, &mut rng); }
        assert!(g.stuck, "the 0.2 u floor graze sticks within 3 ticks");
        let p = g.phase.as_ref().expect("the phase box");
        assert!(p.normal[1] > 0.9, "the floor's +y normal, got {:?}", p.normal);
        assert!((g.pos[1] - 0.04).abs() < 1e-4, "parked 0.04 above the floor, got y {}", g.pos[1]);
    }
}
