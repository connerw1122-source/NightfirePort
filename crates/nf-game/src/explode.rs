//! The explosion entity: `Explode_Create` 0x17cbd0, `Explode_Update` 0x17c350, `Explode_Delete` 0x17cdc0,
//! `Explode_CollisionHandler` 0x17cde0 and `Explode_Propagate` 0x17c518.
//!
//! The game's explosion is a real object (control type 0x40, `obj+0xFF = 0xF`, collision `obj+0xF0 = 0x30`)
//! carrying an EXPLODEINFO block at `obj+0xE0`:
//! - `+0x20` cel (`*(owner_obj+0x20)` — the collision world the sphere test runs against),
//! - `+0x24` attacker obj (the thrower; `Control_ObjectReferenced(attacker, 1)` keeps it alive),
//! - `+0x28` damage (float, `Explode_Propagate`'s amount),
//! - `+0x2C` radius (float, the falloff divisor and the camera-shake amount),
//! - `+0x30` lifetime ticks (script? `*(u16*)(SCRIPTINFO+0xAB0)` : 15),
//! - `+0x34` SCRIPTINFO (the effect script 0x6000052 for every disc bullet caller),
//! - `+0x38` u16 debris kind (`Explode_Update`'s switch: 5/6 metal, 8/10/0xB stone on 0x7000008 only),
//! - `+0x3A` u8 "propagate ran" latch (`Explode_CollisionHandler` fires `Explode_Propagate` once).
//!
//! Object fields: `obj+0x30` = the blast position, `obj+0xE8` = radius · 0.06666667 (the script scale),
//! `obj+0xF0` = 0x30, `obj+0xFF` = 0xF; `SCRIPTINFO+0x94` = f12 (the radius again at the fuse call site).
//!
//! [M] every field above is read off the ACTION.ELF disassembly (mipsdis 0x17cbd0/0x17c350/0x17cde0);
//! the decompiler drops the float args (f12/f13/f14 = script float/radius/damage), so the call sites at
//! 0x12cd98 (the fuse: radius = weapon+0x08, damage = weapon+0x0C, script 0x6000052, attacker = BU+0x30)
//! and 0x12ce80:194-206 (impact explode, `weapon+0x70 & 4`) were disassembled to pin them. The `addiu
//! $t2, $zero, 0x64` (100) at those sites is the 10th argument — an u8 `Explode_Create` never reads —
//! NOT the blast damage: the damage is `f14 = *(float*)(weapon_data+0x0C)` [L the records, named by the
//! disc labels (Txt_BindLabel of +0x38, resolved against USATxt.dat): 0x34 'Fragmentation Grenade' =
//! radius 8 / damage 100 — the real frag kill; 0x35 'Stun Grenade' = 10/0 and 0x36 'Smoke Grenade' = 0/0
//! never reach Explode_Create (the stun takes `Bullet_DoTrails` 0x12d890's `+0x70 & 0x2000` death branch,
//! the smoke its `& 0x20000` branch); 0x37-0x39 'Remote Mine' and 0x3a 'Laser Tripbomb' = 8/100;
//! 0x3b-0x40 'Satchel Charge' = 8/2000].

use crate::world::V3;

/// One explosion (`Explode_Create` 0x17cbd0's object).
#[derive(Clone, Debug)]
pub struct Explosion {
    /// `obj+0x30`: the blast position.
    pub pos: V3,
    /// `EXPLODEINFO+0x2C`: the radius — the falloff divisor and the camera-shake amount.
    pub radius: f32,
    /// `EXPLODEINFO+0x28`: the damage (`Explode_Propagate`'s amount).
    pub damage: f32,
    /// `EXPLODEINFO+0x24`: the attacker (the port's drone index, `usize::MAX` = Bond's throw).
    pub attacker: usize,
    /// `EXPLODEINFO+0x30`: lifetime ticks (the script's `+0xAB0` u16, else 15 [M 0x17cd38]).
    pub lifetime: u16,
    /// `EXPLODEINFO+0x34 != 0`: a script was loaded (the port's script player carries the visual).
    pub script: bool,
    /// `EXPLODEINFO+0x38`: the debris kind (`Explode_Update`'s switch; the port has no debris system —
    /// the game's own debris only spawns for kinds 5/6 (metal) and 8/10/0xB (stone, 0x7000008 only)
    /// after a random gate [M 0x17c350:31-67]).
    pub debris: u16,
    /// `EXPLODEINFO+0x3A`: the propagate latch (`Explode_CollisionHandler`).
    pub propagated: bool,
    /// `obj+0xEC` vs `GameState+0x52`: the age in ticks (the port counts from creation).
    pub age: u32,
    /// The weapon type that caused it (informational here — the death-branch routing lives in
    /// `drone.rs`'s tick on the weapon's own `+0x70` flags: & 0x2000 flash / & 0x20000 smoke [M
    /// 0x12dd60, 0x12e104]).
    pub weapon_type: u16,
}

impl Explosion {
    /// `Explode_Create` (0x17cbd0): `script_hash = 0` gives lifetime 15 [M 0x17cd38 `addiu $v0, $zero, 0xf`];
    /// a script's lifetime is its `SCRIPTINFO+0xAB0` u16 [M 0x17cd18]. The camera shake is the caller's
    /// (`Camera_Shake(pos, radius)` only when radius ≥ 0.5 [M 0x17cc28-0x17cc44]); the port's Sim exposes
    /// it through `Drones::explosions` for the view.
    pub fn create(pos: V3, radius: f32, damage: f32, attacker: usize, script_lifetime: Option<u16>, debris: u16, weapon_type: u16) -> Explosion {
        Explosion {
            pos, radius, damage, attacker,
            lifetime: script_lifetime.unwrap_or(15),
            script: script_lifetime.is_some(),
            debris,
            propagated: false,
            age: 0,
            weapon_type,
        }
    }

    /// `Explode_Update` (0x17c350): the script player runs first (`Script_Update`); when it stops the
    /// object dies (`obj+0xFE |= 1` [M 0x17c394-0x17c3a0]). Without a script the lifetime ticks out.
    /// The debris spawns (kinds 5/6/8/10/0xB) are not ported — the port has no debris system; the
    /// gate was `lifetime>>2 < age` then `age % (Rand(5)+10) == 0` [M 0x17c3d8-0x17c460].
    /// Returns false when the explosion is done.
    pub fn update(&mut self) -> bool {
        self.age += 1;
        if self.script { return self.age <= self.lifetime as u32; }
        self.age < self.lifetime as u32
    }
}

/// Who an explosion damaged, in the port's terms (`Explode_Propagate`'s per-case effects).
#[derive(Clone, Debug, Default)]
pub struct PropagateHits {
    /// Case 2 (drones): `(drone index, damage)` — the game sends msg 9 with a
    /// `DroneImpact{+0 damage, +4 radius, +0x10 pos, +0x20 attacker}` [M 0x17c6e0-0x17c6fc].
    pub drones: Vec<(usize, f32)>,
    /// Case 3 (Bond): the `Player_Hurt` amount (pre-difficulty; the caller applies the level factor).
    pub player: Option<f32>,
    /// Case 0x20 (breakables): `(object index, damage)` — `hitdata+8 = damage`, `Break_ApplyDamage`.
    pub breaks: Vec<(usize, f32)>,
    /// Case 0x21 (destroyables): `Destroy_Smash(obj, attacker, 0x20)` when damage ≥ 1 and
    /// `*(u16*)(obj->data+0x14) & 0x20 == 0` [M 0x17c6a8-0x17c6c0].
    pub destroys: Vec<usize>,
}

/// The targets one `Explode_Propagate` sweep can hit, as the port's Sim sees them. The game gets them
/// from `Collide_SphereIntersect(pos, radius, cel, …, 0xc0)`; the port passes its own lists.
pub struct Targets<'a> {
    /// Case 2 — live drones `(index, pos, hit radius)` (`obj+0x80` position, `obj+0x8C` sphere [M]).
    pub drones: &'a [(usize, V3, f32)],
    /// Case 3 — Bond's position.
    pub player: V3,
    /// Case 0x20 — the breakables `(index, pos, hit radius)`.
    pub breaks: &'a [(usize, V3, f32)],
    /// Case 0x21 — the destroyables `(index, pos, hit radius, smashed already)`.
    pub destroys: &'a [(usize, V3, f32, bool)],
}

/// `Explode_Propagate` (0x17c518): a sphere of `radius` from `pos`; for every hit object
/// `amount = damage · (1 − (int)(dist − obj_radius) / radius)` — the game truncates the
/// distance-minus-sphere term to an INTEGER [M 0x17c620 `cvt.w.s` / `mtc1` pair], clamped at ≥ 0,
/// and anything below 0.0002 takes nothing [M 0x17c62c]. Case dispatch on `obj+0xFF`:
/// 0 copter (accumulate), 2 drone, 3 player, 5 destructible models {0x2B, 0x34–0x37, 0x3A},
/// 0x20 break, 0x21 destroy, 0x28 vehicle, 0x35 MP, 0x36 GT (+ disable), 0x37 sensor, 0x38 monitor,
/// 0x3D sub (class 'J'), 0x4B hit-list entry — 12 damaging cases plus the default skip.
/// `radius < 0.0002` (the flashbang's 0) hits NOTHING [M 0x17c550 early return].
pub fn propagate(pos: V3, radius: f32, damage: f32, attacker: usize, t: &Targets) -> PropagateHits {
    let mut out = PropagateHits::default();
    if !(damage >= 0.0002) || !(radius >= 0.0002) { return out; }
    let mut sweep = |px: V3, obj_radius: f32, hit: &mut dyn FnMut(f32)| {
        let dx = px[0] - pos[0];
        let dy = px[1] - pos[1];
        let dz = px[2] - pos[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt() - obj_radius;
        // `fVar10 = param_2 * (1 - (int)fVar10 / param_1)` — the (int) cast truncates toward zero
        // [M 0x17c61c-0x17c628]; negative distances (inside the object's sphere) give full damage.
        let truncated = if dist > 0.0 { (dist as i32) as f32 } else if dist < 0.0 { -((-dist) as i32) as f32 } else { 0.0 };
        let amount = damage * (1.0 - truncated / radius);
        if amount >= 0.0002 { hit(amount); }
    };
    for &(i, p, r) in t.drones {
        let mut d = 0.0f32;
        sweep(p, r, &mut |a| d = a);
        if d > 0.0 { out.drones.push((i, d)); }
    }
    {
        let mut p = 0.0f32;
        sweep(t.player, 0.0, &mut |a| p = a);
        if p > 0.0 { out.player = Some(p); }
    }
    for &(i, p, r) in t.breaks {
        let mut d = 0.0f32;
        sweep(p, r, &mut |a| d = a);
        if d > 0.0 { out.breaks.push((i, d)); }
    }
    for &(i, p, r, smashed) in t.destroys {
        if smashed { continue; }
        let mut d = 0.0f32;
        sweep(p, r, &mut |a| d = a);
        // Case 0x21's own gate: the smash needs at least 1.0 [M 0x17c6a8].
        if d >= 1.0 { out.destroys.push(i); }
    }
    let _ = attacker;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explosion_without_script_lives_15_ticks() {
        // [M 0x17cd38] `Explode_Create` with script == 0 stores lifetime 15.
        let mut e = Explosion::create([0.0; 3], 10.0, 100.0, usize::MAX, None, 0, 0x35);
        for _ in 0..14 { assert!(e.update(), "alive at age {}", e.age); }
        assert!(!e.update(), "dead at age 15");
        // Sister: with a script the lifetime is the script's +0xAB0 (165 for the blast script's END 165).
        let mut e = Explosion::create([0.0; 3], 10.0, 0.0, usize::MAX, Some(165), 0, 0x35);
        for _ in 0..165 { assert!(e.update()); }
        assert!(!e.update(), "the scripted explosion dies at 166");
    }

    #[test]
    fn propagate_falloff_truncates_the_distance() {
        // Radius 10, damage 100: a drone at distance 5.7 − radius 1 = 4.7 → (int) 4 → 100·(1−4/10) = 60.
        // A smooth falloff would give 57; the game's integer truncation gives 60 [M 0x17c61c-0x17c628].
        let t = Targets { drones: &[(0, [5.7, 0.0, 0.0], 1.0)], player: [1000.0; 3], breaks: &[], destroys: &[] };
        let h = propagate([0.0; 3], 10.0, 100.0, usize::MAX, &t);
        assert_eq!(h.drones.len(), 1);
        assert!((h.drones[0].1 - 60.0).abs() < 1e-4, "truncated falloff, got {}", h.drones[0].1);
        // Inside the object's own sphere (dist − radius < 0): full damage, not negative.
        let t = Targets { drones: &[(0, [0.5, 0.0, 0.0], 1.0)], player: [1000.0; 3], breaks: &[], destroys: &[] };
        let h = propagate([0.0; 3], 10.0, 100.0, usize::MAX, &t);
        assert!((h.drones[0].1 - 100.0).abs() < 1e-4, "inside the sphere: full, got {}", h.drones[0].1);
        // Below the 0.0002 cutoff: nothing.
        let t = Targets { drones: &[(0, [9.999, 0.0, 0.0], 0.0)], player: [1000.0; 3], breaks: &[], destroys: &[] };
        let h = propagate([0.0; 3], 10.0, 100.0, usize::MAX, &t);
        assert!(h.drones.is_empty() || h.drones[0].1 >= 0.0002);
    }

    #[test]
    fn zero_radius_hits_nothing() {
        // The smoke grenade's record is radius 0 / damage 0 [L weapon 0x36 +0x08/+0x0C 'Smoke Grenade']:
        // the early return [M 0x17c550] means the smoke damages NOBODY through Propagate (and the stun
        // 0x35 never reaches Propagate at all — the `& 0x2000` flash branch).
        let t = Targets { drones: &[(0, [0.1, 0.0, 0.0], 0.0)], player: [0.1, 0.0, 0.0], breaks: &[], destroys: &[] };
        let h = propagate([0.0; 3], 0.0, 100.0, usize::MAX, &t);
        assert!(h.drones.is_empty() && h.player.is_none() && h.breaks.is_empty() && h.destroys.is_empty());
        let h = propagate([0.0; 3], 10.0, 0.0, usize::MAX, &t);
        assert!(h.drones.is_empty() && h.player.is_none());
    }

    #[test]
    fn destroy_case_needs_damage_at_least_one() {
        // Case 0x21's gate [M 0x17c6a8]: the smash fires only when the falloff reaches 1.0. With the
        // integer truncation the amounts jump in damage/radius steps: damage 5, radius 10 → a target at
        // 9.5 (truncated 9) takes 5·(1−9/10) = 0.5 → no smash; at 4.5 (truncated 4) it takes 3 → smash.
        let far = Targets { drones: &[], player: [1000.0; 3], breaks: &[],
            destroys: &[(0, [9.5, 0.0, 0.0], 0.0, false)] };
        let h = propagate([0.0; 3], 10.0, 5.0, usize::MAX, &far);
        assert!(h.destroys.is_empty(), "0.5 damage must not smash");
        let near = Targets { drones: &[], player: [1000.0; 3], breaks: &[],
            destroys: &[(0, [4.5, 0.0, 0.0], 0.0, false), (1, [4.5, 0.0, 0.0], 0.0, true)] };
        let h = propagate([0.0; 3], 10.0, 5.0, usize::MAX, &near);
        assert_eq!(h.destroys, vec![0], "3 damage smashes the unsmashed one only");
    }
}
