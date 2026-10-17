//! Hurt volumes — placement type 0xE4 (`Hurt_Create` 0x17e870, `Hurt_Update` 0x17e5d0,
//! `Hurt_GetDamage` 0x17ea18, `Hurt_GetType` 0x17ea50), the game's contact damage volumes
//! (docs/systems/hurt-volumes.md).
//!
//! The param map [M Hurt_Create 0x17e870: the creator's record fields, `tag+0x2C+4k` = param k —
//! the convention every ported kind uses; pinned by 07000002 #13 (gate 141, sound 0xffff) and
//! 07000016 #309 (gate 243, damage 1000) [D probes]]:
//! - param 0 byte → ext+0x14: the pain type `Hurt_GetType` 0x17ea50 returns
//!   (`Player_HandlePain` 0x1902c8:84-104 gives types 1..7 armourFraction 0 — armour cannot
//!   absorb a hurt volume; the port's `PlayerHealth::pain_generic` mirrors that switch).
//! - param 1 int→float → ext+0x10: the damage `Hurt_GetDamage` 0x17ea18 returns.
//! - param 2 u16 → ext+0x18: the gate channel (`Hurt_Update` reads `switch_channels[gate]`).
//! - param 3 u16 → ext+0x1a (0 → 0xffff): the activation sound id.
//! - param 4 u16 → ext+0x16: the WHO mask the state-0 arm ANDs with the contact classes.
//! - param 5 byte → ext+0x15: the one-shot flag (state 1's expiry parks state 2 → retire).
//!
//! `Hurt_Update` 0x17e5d0's state machine [M, the decompile + disasm]:
//! - the gate maintenance is a LIVE level, every tick: with a gate set, the channel LOW sets
//!   `obj+0xF0 |= 0x20` and the channel UP clears it (`0x17e5d0:26-37`), and state 1's entry
//!   re-checks it (`:109-116` — a falling gate parks state 0 with the bit set). `Hurt_Create`'s
//!   tail sets the bit at create when the channel reads low [M 0x17e870 tail; the channels start
//!   zeroed, so every gated volume loads disabled].
//! - state 0 (armed) computes the who-mask `uVar5` (`:51-102`): gated → `uVar5 =
//!   switch_channels[gate]` — the channel VALUE is the mask, so a live channel starts the
//!   volume; ungated with who 0 → `uVar5 = 1` (starts at once); ungated with who ≠ 0 → walk the
//!   object's `obj+0xD0` hit chain OR-ing the contact classes (class 3 → 8, class 2 → 0x10,
//!   class 5 → 2; classes 1/4 collect nothing) then `uVar5 = who & collected`. `uVar5 != 0` →
//!   state 1. So who 1 collects nothing, ever — the configured-inert volumes (07000005
//!   #328–330/#334–340, 07000006 #373 [D params]); the player brings 0x10.
//! - state 1 (animating): the sound at t 0 + the spline pose; duration 0 parks (`:106-108` —
//!   every 07000005 hurt reads 0 keys [D]); a one-shot expires into state 2, which overwrites
//!   `obj+0xFF = 1` and retires the volume (`:129-132`, `:41-47`).
//! - THE DAMAGE does not flow through `Hurt_Update`: the player's object collision applies it —
//!   `Player_DealWithObjHit` 0x194458 (the class-0x27 arm) calls
//!   `Player_HandlePain(impact + Hurt_GetDamage(obj), …, Hurt_GetType(obj), −1)` with NO state
//!   or who check of its own; NPCs get the same treatment through `NDrone2_DealWithObjHit`
//!   0x13fff0. The gate therefore rides the collide contact itself: an inactive volume (the
//!   0x20 bit set, or an ungated who whose mask no contact class satisfies) must not produce
//!   the contact — that is the state machine's contact gate, and it is what keeps 07000016's
//!   #309 "HurtPoly-ShuttleLaunch" (gate 0xf3, 1000 damage [D]) from killing Bond on the silo
//!   floor during the chaser fight, before the launch raises ch 243.
//!
//! The port's contact is the standing capsule against the placement's own collision triangles —
//! the glist `Control_SetGList` links at create [M 0x17e870:44], resolved like the trigger
//! builder's (the same source) and tested with the trigger walk's capsule-vs-mesh contact
//! (triggers::capsule_hits_tris). The exact BL object-collision pick is the port's approximation
//! [S]; the class arm and the numbers are [M]. Only the player's damage pass is wired here (the
//! game's drone-side pass is `NDrone2_DealWithObjHit` 0x13fff0 — the who masks that name drone
//! classes, e.g. 07000008's #42–44 who 0x18, are delivered for it [D survey]).
use crate::triggers::capsule_hits_tris;
use crate::world::V3;
use nf_formats::level::Level;

#[derive(Clone, Debug)]
pub struct Hurt {
    pub placement: usize,
    /// ext+0x14 = param 0 byte: the pain type handed to `Player_HandlePain` [M 0x17e870, 0x17ea50].
    pub kind: u8,
    /// ext+0x10 = param 1 as int→float: the damage [M 0x17e870:24-31, 0x17ea18].
    pub damage: f32,
    /// ext+0x18 = param 2: the gate channel — `Hurt_Update` parks the volume while it reads low
    /// (the live 0x20 bit) [M 0x17e5d0:26-37]. 0 = ungated.
    pub gate: u16,
    /// ext+0x1a = param 3 (0 → 0xffff): the activation sound id [M 0x17e5d0:119-123; the state-1
    /// `Sound_Play3D(100.0, 1.0, 1.0, id, obj+0x30, …)`]. Delivered for the sound pass; the port
    /// does not fire it (no level in the survey sets one [D]).
    pub sound: u16,
    /// ext+0x16 = param 4: the WHO mask `Hurt_Update` 0x17e5d0's state 0 ANDs with the contact
    /// classes' bits (class 3 → 8, class 2 = the player → 0x10, class 5 → 2) when there is no
    /// gate [M 0x17e5d0:57-95]. 0 starts at once; who 1 collects nothing at all (the
    /// configured-inert volumes, e.g. 07000006 #373).
    pub who: u16,
    /// ext+0x15 = param 5: the one-shot flag — state 1's expiry parks state 2, which retires the
    /// volume [M 0x17e5d0:129-132 + :41-47].
    pub one_shot: bool,
    /// The volume's cels: the placement's object collision triangles in world space (the glist
    /// `Control_SetGList` links at create [M 0x17e870:44]; the same source the trigger builder
    /// reads — `level_tag` object collision, placement matrix applied).
    pub tris: Vec<[V3; 3]>,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    /// The obj+0xF0 0x20 disabled bit: set at create when the gate channel reads low (the
    /// channels start zeroed) [M 0x17e870 tail], maintained every tick by `Hurt_Update`
    /// [M 0x17e5d0:26-37 — channel low sets it, channel up clears it].
    pub disabled: bool,
    /// The one-shot spent (the state-2 retire) [M 0x17e5d0:41-47].
    pub fired: bool,
}

impl Hurt {
    /// `Hurt_Create` 0x17e870 over the main map's placements (placement order, like the game's
    /// `parsemap_create_dynamic_objects` 0x1d04f0 case 0xE4).
    pub fn create(level: &Level) -> Vec<Hurt> {
        let Some(m) = level.main_map() else { return Vec::new() };
        let map = &level.maps[m];
        let mut out = Vec::new();
        for (i, p) in map.placements.iter().enumerate() {
            if p.flags & 0xff != 0xe4 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
            let mx = nf_formats::placement::object_matrix(p.rotation, p.translation);
            let tris: Vec<[V3; 3]> = target
                .and_then(|(om, oi)| level.maps[om].objects[oi].collision.as_ref())
                .map(|c| c.triangles().into_iter().map(|t| t.map(|v| transform(&mx, v))).collect())
                .unwrap_or_default();
            let mut lo = [f32::MAX; 3];
            let mut hi = [-f32::MAX; 3];
            for t in &tris { for v in t { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
            let gate = param(2) as u16;
            out.push(Hurt {
                placement: i,
                kind: param(0) as u8,
                damage: param(1) as i32 as f32,
                gate,
                sound: param(3) as u16,
                who: param(4) as u16,
                one_shot: param(5) & 0xff == 1,
                tris, lo, hi,
                // Hurt_Create's tail [M]: the gate set and low at create → obj+0xF0 |= 0x20. The
                // switch channels start zeroed, so a gated volume always loads disabled.
                disabled: gate != 0,
                fired: false,
            });
        }
        out
    }

    /// The AABB quick reject + the capsule-mesh contact (the trigger `contains` rule). The
    /// capsule is the standing player segment (feet `a` → head `b`, radius `r`).
    pub fn contains(&self, a: V3, b: V3, r: f32) -> bool {
        if self.tris.is_empty() { return false; }
        if (0..3).any(|k| (a[k] < self.lo[k] - r && b[k] < self.lo[k] - r) || (a[k] > self.hi[k] + r && b[k] > self.hi[k] + r)) { return false; }
        capsule_hits_tris(a, b, r, &self.tris)
    }

    /// `Hurt_Update` 0x17e5d0's gate maintenance + state-0 start law, and the class-0x27 contact
    /// damage (`Player_DealWithObjHit` 0x194458's arm through the caller's `Player_HandlePain`
    /// pipeline — `PlayerHealth::pain_generic`: the difficulty factor, no armour for types 1..7
    /// [M 0x1902c8:84-104]). Returns Some((damage, pain type)) on a contact tick.
    ///
    /// `gate_on` is the gate channel's live value (the caller reads `channels.on(gate)`; a zero
    /// gate passes anything — the law below skips it). The contact gate is the state machine's:
    /// a disabled volume (the 0x20 bit the gate maintenance keeps) and an ungated who whose mask
    /// no contact class satisfies produce no contact, so no damage.
    pub fn update(&mut self, gate_on: bool, bond_pos: V3) -> Option<(f32, u8)> {
        // The gate maintenance [M 0x17e5d0:26-37 — the live every-tick level: the channel low
        // sets obj+0xF0|0x20, the channel up clears it; a zero gate never disables].
        if self.gate != 0 { self.disabled = !gate_on; }
        if self.disabled || self.fired { return None; }
        // The state-0 start law for an ungated volume [M 0x17e5d0:51-102]: who 0 starts at once
        // (uVar5 = 1); a nonzero who starts only when a contact class bit survives the AND — the
        // player brings 0x10 (class 2), so who 1 collects nothing and never starts (the
        // configured-inert volumes). Gated volumes: the channel VALUE is the who-mask
        // [M :97-99] — the live channel above is the start.
        if self.gate == 0 && self.who != 0 && self.who & 0x10 == 0 { return None; }
        // The collide contact: the standing capsule (r 0.55 [M Player_CollLadder's +0x7CC],
        // 1.8 tall) against the volume's glist tris.
        let a = [bond_pos[0], bond_pos[1], bond_pos[2]];
        let b = [bond_pos[0], bond_pos[1] + 1.8, bond_pos[2]];
        if !self.contains(a, b, 0.55) { return None; }
        // The one-shot expiry [M 0x17e5d0:129-132 → :41-47]: state 2 retires the volume. The
        // anim-clock path (the state-1 t/inc advance) is presentation — the port parks on the
        // first contact pass [S for the timing; no survey level sets param 5 [D]].
        if self.one_shot { self.fired = true; }
        Some((self.damage, self.kind))
    }
}

fn transform(m: &[[f32; 4]; 4], v: V3) -> V3 {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit hurt whose volume is a flat 2×2 quad at the origin (two tris).
    fn hurt(gate: u16) -> Hurt {
        let tris = vec![
            [[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0]],
            [[-1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]],
        ];
        Hurt { placement: 0, kind: 1, damage: 1.0, gate, sound: 0xffff, who: 0, one_shot: false, tris, lo: [-1.0, 0.0, -1.0], hi: [1.0, 0.0, 1.0], disabled: gate != 0, fired: false }
    }

    #[test]
    fn gate_off_disables_the_volume() {
        // [M Hurt_Create 0x17e870 tail + Hurt_Update 0x17e5d0:26-37: the gate channel off parks
        // obj+0xF0|0x20, the channel up clears it — the live every-tick law].
        let mut h = hurt(141);
        assert!(h.update(false, [0.0, 0.0, 0.0]).is_none(), "a gated-off volume never hurts");
        assert!(h.disabled);
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some(), "the gate on enables it");
        assert!(!h.disabled);
        assert!(h.update(false, [0.0, 0.0, 0.0]).is_none(), "the channel falling parks it again [M :109-116]");
    }

    #[test]
    fn contact_is_the_capsule_against_the_tris() {
        let mut h = hurt(0);
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some());
        assert!(h.update(true, [0.0, 0.4, 0.0]).is_some(), "the capsule spans 0.4..2.2 — the quad is inside its radius");
        assert!(h.update(true, [0.0, 1.7, 0.0]).is_none(), "feet above the plane out of reach: no contact");
        assert!(h.update(true, [3.0, 0.0, 0.0]).is_none(), "outside the quad's reach");
    }

    #[test]
    fn the_who_mask_decides_whether_an_ungated_volume_can_start() {
        // [M Hurt_Update 0x17e5d0:57-95; docs/systems/hurt-volumes.md] who 1 collects nothing
        // (07000006 #373, 07000005 #328-330): inert. Sisters: who 0 and who 0x18 (07000008
        // #42-44) reach the player.
        let mut h = hurt(0);
        h.who = 1;
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_none(), "who 1 never starts");
        h.who = 0x18;
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some(), "who 0x18 carries the player bit");
        h.who = 0;
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some(), "who 0 starts at load");
    }

    #[test]
    fn ungated_volumes_never_disable() {
        // #13's gate is 141, but a zero gate must read always-on [M 0x17e5d0:51: the gate==0
        // branch skips the channel test entirely].
        let mut h = hurt(0);
        assert!(h.update(false, [0.0, 0.0, 0.0]).is_some());
    }

    #[test]
    fn the_gated_volume_reads_the_channel_value_not_the_who_param() {
        // [M 0x17e5d0:97-99: gated → uVar5 = switch_channels[gate] — the channel VALUE is the
        // who-mask. 07000016's #309 carries who 1 AND gate 0xf3: gated, so the who 1 does NOT
        // make it inert — the launch channel does start it; the fight's low channel does not.]
        let mut h = hurt(0xf3);
        h.who = 1;
        assert!(h.update(false, [0.0, 0.0, 0.0]).is_none(), "the low channel parks it");
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some(), "the channel up starts it despite who 1");
    }

    #[test]
    fn one_shot_retires_after_the_first_pass() {
        // [M 0x17e5d0:129-132 → :41-47: state 2 overwrites obj+0xFF = 1 — the volume retires.]
        let mut h = hurt(0);
        h.one_shot = true;
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_some(), "the first pass hurts");
        assert!(h.fired);
        assert!(h.update(true, [0.0, 0.0, 0.0]).is_none(), "the retired volume never hurts again");
    }
}
