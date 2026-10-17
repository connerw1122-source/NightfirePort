//! Hurt volumes — placement type 0xE4, the game's contact damage volumes.
//!
//! `Hurt_Create` 0x17e870 (class byte obj+0xFF = 0x27): the params work array at obj+0xE0 gets
//! duration (record+2 − 1), the tick rate (record+4), the damage float (record+0x30 — what
//! `Hurt_GetDamage` 0x17ea18 returns), the gate channel (record+0x34 — the obj+0xF0 0x20
//! "disabled" bit is SET at create when the channel is off [M 0x17e870 tail], and cleared/kept
//! by `Hurt_Update` 0x17e5d0's state machine), the sound id (record+0x38, 0 → 0xffff none), the
//! presence mask (record+0x3C — 0 = always trigger, else the collide-class bits of the obj+0xD0
//! overlap list) and the one-shot byte (record+0x40 — state 1's expiry parks state 2, which
//! overwrites obj+0xFF to 1 and retires the volume).
//!
//! The damage itself rides the collide path, not the state machine: `Player_DealWithObjHit`
//! 0x194458:94-101 — a class-0x27 contact calls `Player_HandlePain(pain + Hurt_GetDamage(obj),
//! BLData, HITDATA, Hurt_GetType(obj), −1)`; NPCs get the same treatment through
//! `NDrone2_DealWithObjHit` 0x13fff0. `Player_HandlePain` 0x1902c8:84-104 lets armour absorb ONLY
//! type 0; types 1..7 drain health alone (the port's `PlayerHealth::pain_generic`).
//!
//! Placement params are the creator's record fields by index (the convention every ported kind
//! uses — e.g. `Destroy_Create`: tag+0x12 = param 0 [M disasm; props.rs]). 07000002's only Hurt,
//! #13 `electrocute` [D probe: params 0=1, 1=1, 2=0x8D(141), 3=0xffff, 4=1], pins the map where it
//! is provable: param 2 = the gate channel and param 3 = the sound id (0xffff = none, the creator's
//! own "unset" value). The remaining three all read 1 on the disc — whatever the exact
//! param→field split among {pain type, duration, rate}, #13 is type 1 (the electric pain class),
//! damage 1.0 per contact tick; the split below is marked [S] where it is a choice, and it cannot
//! change #13's behaviour. Gate 141 is written by Emitter #9 (`ParticleEmmiter` [D probe: params
//! 2=0x82 watch, 4=0x8D]) — the arc visual the smashed cable light #392 (ch 0x82) kills, so the
//! volume hurts exactly while the arc burns.
//!
//! The box is the placed object's bounds (props.rs `bounds`) — the port's established stand-in for
//! the placed glist contact (the game's Collide_Update 0x1ea398 builds the overlap list from the
//! glist; the port's boxes are the same bounds the bullet path uses).

use crate::props::bounds;
use crate::world::V3;

#[derive(Clone, Debug)]
pub struct Hurt {
    pub placement: usize,
    pub lo: V3,
    pub hi: V3,
    /// param 2 — the gate channel [D #13 = 141; M Hurt_Create's record+0x34].
    pub gate: u16,
    /// param 4 [S — see the module doc; 1.0 on the disc's only instance].
    pub damage: f32,
    /// param 0's low byte [S — 1 on the disc's only instance; 1..7 bypass armour [M 0x1902c8:84-104]].
    pub pain_type: u8,
    /// param 3, 0xffff = none [M record+0x38; D #13 = 0xffff].
    pub sound: u16,
    /// the one-shot byte [M record+0x40 → state 2 retires the volume; D default 0].
    pub one_shot: bool,
    /// The obj+0xF0 0x20 disabled bit: set at create when the gate channel is off [M 0x17e870 tail],
    /// maintained by Hurt_Update.
    pub disabled: bool,
    pub fired: bool,
}

impl Hurt {
    /// `Hurt_Create` 0x17e870 over the main map's placements (placement order, like the game's
    /// `parsemap_create_dynamic_objects` 0x1d04f0 case 0xE4).
    pub fn create(level: &nf_formats::level::Level) -> Vec<Hurt> {
        let Some(m) = level.main_map() else { return Vec::new() };
        let mut out = Vec::new();
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0xE4 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (lo, hi) = bounds(level, m, p);
            let gate = param(2) as u16;
            // The create-time disabled bit: a set gate whose channel is off parks the volume [M tail].
            let disabled = gate != 0 && !level_channel(level, gate);
            out.push(Hurt {
                placement: i,
                lo,
                hi,
                gate,
                damage: param(4) as f32,
                pain_type: (param(0) & 0xff) as u8,
                sound: param(3) as u16,
                one_shot: false,
                disabled,
                fired: false,
            });
        }
        out
    }

    /// `Hurt_Update` 0x17e5d0's gate maintenance + the contact test. The damage goes through the
    /// caller's `Player_HandlePain` pipeline (`PlayerHealth::pain_generic` — the difficulty factor,
    /// no armour for types 1..7 [M 0x194458:94-101 + 0x1902c8:84-104]). Returns Some((damage,
    /// pain type)) on a contact tick. The port tests the bond capsule against the volume's box;
    /// the game's collide glist is the same placed geometry's bounds.
    pub fn update(&mut self, gate_on: bool, bond_pos: V3) -> Option<(f32, u8)> {
        // Hurt_Update's disabled-bit maintenance [M 0x17e5d0: the gate channel off → 0xF0 |= 0x20,
        // on → &= ~0x20]; a zero gate never disables.
        self.disabled = self.gate != 0 && !gate_on;
        if self.disabled || self.fired { return None; }
        // The collide contact: the bond capsule (r 0.55 [M Player_CollLadder's +0x7CC], 1.8 tall)
        // against the volume box.
        const R: f32 = 0.55;
        const H: f32 = 1.8;
        let inside = (0..3).all(|k| {
            let (a, b) = if k == 1 { (bond_pos[1], bond_pos[1] + H) } else { (bond_pos[k] - R, bond_pos[k] + R) };
            b >= self.lo[k] && a <= self.hi[k]
        });
        if !inside { return None; }
        if self.one_shot { self.fired = true; }
        Some((self.damage, self.pain_type))
    }
}

/// The gate channel's state at create time. The level data does not carry channel values; the
/// game's `switch_channels` start zeroed, so a create-time read of an unraised channel is off
/// [M the channel model]. The caller passes the live table once the sim exists.
fn level_channel(_level: &nf_formats::level::Level, _ch: u16) -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    fn hurt(gate: u16) -> Hurt {
        Hurt { placement: 0, lo: [-1.0, -1.0, -1.0], hi: [1.0, 1.0, 1.0], gate, damage: 1.0, pain_type: 1, sound: 0xffff, one_shot: false, disabled: false, fired: false }
    }

    #[test]
    fn gate_off_disables_the_volume() {
        // [M Hurt_Create 0x17e870 tail + Hurt_Update 0x17e5d0: the gate channel off parks obj+0xF0|0x20]
        let mut h = hurt(141);
        assert!(h.update(false, [0.0, 0.0, 0.0]).is_none(), "a gated-off volume never hurts");
        assert!(h.disabled);
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some(), "the gate on enables it");
        assert!(!h.disabled);
    }

    #[test]
    fn contact_is_the_capsule_against_the_box() {
        let mut h = hurt(0);
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some());
        assert!(h.update(true, [0.0, 0.9, 0.0]).is_some(), "the capsule spans 0.9..2.7 — overlap");
        assert!(h.update(true, [0.0, 1.7, 0.0]).is_none(), "feet above the box top: no contact");
        assert!(h.update(true, [0.0, 1.9, 0.0]).is_none(), "above the box: no contact");
        assert!(h.update(true, [3.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn ungated_volumes_never_disable() {
        // #13's gate is 141, but a zero gate must read always-on [M 0x17e5d0 state 0: the gate==0
        // branch skips the channel test entirely].
        let mut h = hurt(0);
        assert!(h.update(false, [0.0, 0.0, 0.0]).is_some());
    }
}
