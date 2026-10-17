//! The hurt volumes (placement kind 0xE4, `Hurt_Create` 0x17e870, `Hurt_Update` 0x17e5d0,
//! `Hurt_GetDamage` 0x17ea18, `Hurt_GetType` 0x17ea50).
//!
//! The game's model [M]:
//! - `Hurt_Create` 0x17e870 builds a class-0x27 object (`*(undefined1 *)(iVar8 + 0xff) = 0x27`),
//!   links the placement's glist (`Control_SetGList`), and copies the placement params into the
//!   ext record: param 0 byte → +0x14 (the type, `Hurt_GetType`), param 1 int → float +0x10 (the
//!   damage, `Hurt_GetDamage`), param 2 u16 → +0x18 (the gate channel — `Hurt_Update` reads
//!   `switch_channels[gate]`), param 3 u16 → the one-shot fire sound id (0xFFFF = none), param 4
//!   u16 → +0x16 (the class mask the state-0 arm scans), param 5 byte → the loop flag.
//! - `Hurt_Update` 0x17e5d0 only arms (state 0 → 1), plays the fire sound and animates the object
//!   (its frame count comes from placement+0x2, which is 0 on every 07000005 hurt — the anim never
//!   runs [M :96-99]). THE DAMAGE DOES NOT FLOW THROUGH `Hurt_Update`: the player's object
//!   collision applies it — `Player_DealWithObjHit` 0x194458:95-103 sees the hit object's
//!   obj+0xFF == 0x27 and calls `Player_HandlePain(damage + hitdata_extra, Hurt_GetType(obj), −1)`.
//!   `Player_HandlePain` 0x1902c8:84-104 gives pain types 1..7 armourFraction 0 — armour cannot
//!   absorb a hurt volume (the port's `pain_generic` mirrors exactly that switch).
//! - The contact itself is the player capsule against the volume's own cels (the glist the create
//!   linked). The port approximates the BL object-collision pick with the same capsule-vs-mesh
//!   contact the triggers use (`triggers::capsule_hits_tris`) — [S] for the exact pick rule, [M]
//!   for the class-0x27 damage arm and the numbers.
//!
//! 07000005 carries 13 of them [D placements]: Hurt_A ×3 at y≈1849 (the sky), Hurt_B ×3 at the
//! map border, Hurt_C ×4 at the valley floor edges, + 3 origin-parked (Hurt_sk / Hurt_Tower_sniper
//! / hurter_sk). All params (0,6),(1,1000),(2,0),(3,65535) [D] — type 6, damage 1000, no gate, no
//! sound: touching one kills Bond outright (05–08's damage factor 0.6..1.0 [M 0x1d3c44] leaves it
//! ≥ 600 against a ≤ 100 health pool).

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
    /// ext+0x18 = param 2: the gate channel — `Hurt_Update` refuses to arm while it is off
    /// [M 0x17e5d0:31-56]. 0 = ungated.
    pub gate: u16,
    /// The volume's cels: the placement's object collision triangles in world space (the glist
    /// `Control_SetGList` links at create [M 0x17e870:44]; the same source the trigger builder
    /// reads — `level_tag` object collision, placement matrix applied).
    pub tris: Vec<[V3; 3]>,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}

impl Hurt {
    pub fn create(level: &Level) -> Vec<Hurt> {
        let Some(m) = level.main_map() else { return Vec::new() };
        let map = &level.maps[m];
        let mut out = Vec::new();
        for (i, p) in map.placements.iter().enumerate() {
            if p.flags & 0xff != 0xe4 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
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
            out.push(Hurt {
                placement: i,
                kind: param(0) as u8,
                damage: param(1) as i32 as f32,
                gate: param(2) as u16,
                tris, lo, hi,
            });
        }
        out
    }

    /// The AABB quick reject + the capsule-mesh contact (the trigger `contains` rule).
    pub fn contains(&self, a: V3, b: V3, r: f32) -> bool {
        if self.tris.is_empty() { return false; }
        if (0..3).any(|k| (a[k] < self.lo[k] - r && b[k] < self.lo[k] - r) || (a[k] > self.hi[k] + r && b[k] > self.hi[k] + r)) { return false; }
        capsule_hits_tris(a, b, r, &self.tris)
    }
}

fn transform(m: &[[f32; 4]; 4], v: V3) -> V3 {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2],
    ]
}
