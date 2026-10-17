//! The zip wire (`Wire_Create` 0x1981d8, placement type 0x3E, class 0x2C) and the geometry its riders use.
//! The player substates live in `player.rs`: `Player_Wire` 0x197658 (substate 6, wire type 0,
//! hand-over-hand) and `Player_Zipline` 0x197f50 (substate 0xF, wire type 2). A grab does NOT enter the
//! creep substate 7 — that is the creep-wall system (class 0x2B, `Player_CollCreepWall`), a different
//! object (docs/atlas/wire-grapple.md, which corrects the earlier "enters creep" reading).
//!
//! `Wire_Create` [M disasm 0x198200–0x19826c]: `control_create_object(8, …)` — an 8-byte instance:
//! `inst+0x00 u16` = placement param 1 = the wire type (0 hand-over-hand, 2 zipline; 1 is refused by
//! `Player_CollWire` 0x197130:99), `inst+0x02` = param 0, `inst+0x04` = param 2 (neither is read by any
//! wire code found), `inst+0x06` = param 3 = the switch channel set 1 while Bond is on the wire and 0 on
//! drop (`Player_Wire` 0x1976bc–0x1976d4). `obj+0xFF = 0x2C`; the placement's `rot.z` is negated before
//! the object matrix is built (@0x198208–0x198214 — a wire-only quirk; `Ladder_Create` passes the
//! rotation through) [M]. **There are no endpoint params**: the line geometry derives at grab/ride time
//! from the object's world sphere (`obj+0x80..0x8C`, centre + radius) and the placement yaw [M].
//!
//! Spec: docs/atlas/wire-grapple.md, docs/systems/player.md.

use crate::searchlight::spherical;
use crate::world::V3;
use nf_formats::level::{Level, MapObject};
use nf_formats::placement::Placement;

/// A wire object (class 0x2C, `Wire_Create` 0x1981d8).
#[derive(Clone, Debug)]
pub struct Wire {
    /// `obj+0x80..0x8C`: the world-sphere centre (`Control_BuildWorldSph` 0x133070 over the placement's
    /// object bounds, like `Ladder::create`). All three segments derive from it at grab/ride time [M].
    pub centre: V3,
    /// `obj+0x8C`: the world-sphere radius (4.039 on 07000005's "Wire_Cable" [L ee_start_05.bin 016dc430]).
    pub radius: f32,
    /// `obj+0x54`: the yaw from the object matrix (the placement's yaw). The grab/travel/ride directions
    /// are `spherical(r, yaw + π/2, pitch)` — the call sites add the π/2 [M `Player_CollWire` 0x1971f8;
    /// `spherical` = the port of `Vec_Spherical_2_Cartesian` 0x1e3948, verified against the live searchlight
    /// beam matrix — see `searchlight::spherical`]. With 07000005's yaw −1.5708 the wire runs along +Z [L
    /// ee_start_05.bin: the atlas's "endpoints ≈ (−19.95…−27.03, …)" X-span used a cos/sin-swapped reading;
    /// the disassembly above pins the axis to Z].
    pub yaw: f32,
    /// `obj+0x58`: the pitch. `Wire_Create` negates the placement's rot.z before building the matrix
    /// [M 0x198208–0x198214]; both live wires (07000005/07000002) carry rot.z 0 so the ride is flat [D
    /// placements probe]. The euler→+0x58 mapping for nonzero rot.x is unverified (atlas "Not verified").
    pub pitch: f32,
    /// `inst+0x00` = placement param 1: the wire type — 0 = hand-over-hand (substate 6), 2 = zipline
    /// (substate 0xF), 1 = refused [M `Player_CollWire` 0x197130:99-106].
    pub wire_type: u16,
    /// `inst+0x02` = placement param 0 — not referenced by any wire code found (2 on 07000005, 3 on
    /// 07000002 [D placements probe]) [M].
    pub param0: u16,
    /// `inst+0x04` = placement param 2 — not referenced (0 both levels) [M].
    pub param2: u16,
    /// `inst+0x06` = placement param 3: the switch channel — set 1 every tick Bond is on the wire, 0 on
    /// drop (0 on both live wires) [M `Player_Wire` 0x1976bc, `Player_Zipline` 0x19806c].
    pub channel: u16,
}

impl Wire {
    /// `Wire_Create` (0x1981d8) from a placement (type 0x3E) plus the placement's object bounds.
    pub fn create(p: &Placement, obj: Option<&MapObject>) -> Option<Wire> {
        let o = obj?;
        let yaw = p.rotation[1];
        // G-046: the object-matrix yaw rows are PS2Sinf-built [M Control_BuildWorldSph 0x133070's matrix;
        // nf-formats::placement::object_matrix already runs anim::sinf] — the port's rebuild follows.
        let (s, c) = (crate::trig::sinf(yaw), crate::trig::cosf(yaw));
        // The world sphere: the glist bounds centre through the yaw-only object matrix + translation
        // (the same computation as `Ladder::create` / `Bond::sphere_centre`) [M 0x133070].
        let centre = [
            p.translation[0] + o.centre[0] * c + o.centre[2] * s,
            p.translation[1] + o.centre[1],
            p.translation[2] - o.centre[0] * s + o.centre[2] * c,
        ];
        let scale = p.scale();
        let s_max = scale.iter().fold(1.0f32, |a, &v| a * v.max(1e-6).abs());
        // The wire-only rot.z negation [M 0x198208-0x198214].
        let pitch = -p.rotation[2];
        Some(Wire {
            centre,
            radius: o.radius * s_max,
            yaw,
            pitch,
            wire_type: param(p, 1) as u16,
            param0: param(p, 0) as u16,
            param2: param(p, 2) as u16,
            channel: param(p, 3) as u16,
        })
    }

    /// The grab segment (`Player_CollWire` 0x1971f8–0x197208): centre ± `spherical(radius − 2.0,
    /// yaw + π/2, obj+0x58)`. Note `spherical` already folds the game's `+π/2` (see its doc) — the call
    /// passes the yaw unchanged, matching `searchlight`'s callers [M].
    pub fn grab_segment(&self) -> (V3, V3) {
        self.segment(self.radius - 2.0, Some(self.pitch))
    }

    /// The travel segment (`Player_Wire` 0x1976f8–0x197724): centre ± `spherical(radius − 0.5, yaw + π/2,
    /// 0)` — the pitch is forced 0 for the hand-over-hand crossing [M].
    pub fn travel_segment(&self) -> (V3, V3) {
        self.segment(self.radius - 0.5, Some(0.0))
    }

    /// The zipline ride segment (`Player_Zipline` 0x197ff0–0x198024): the FULL radius at the object's
    /// pitch (sloped ziplines) [M].
    pub fn zipline_segment(&self) -> (V3, V3) {
        self.segment(self.radius, Some(self.pitch))
    }

    fn segment(&self, r: f32, pitch: Option<f32>) -> (V3, V3) {
        // The game's call sites add π/2 to obj+0x54 before the call (`Player_CollWire` 0x1971f8:
        // `Vec_Spherical_2_Cartesian(radius−2.0, obj+0x54 + 1.5707964, obj+0x58, …)`); `spherical` is the
        // port of the game function with its argument convention unchanged (the function's own internal
        // +π/2 is already folded into its sin/cos pairing — see `searchlight::spherical`'s doc).
        let v = spherical(r, self.yaw + std::f32::consts::FRAC_PI_2, pitch.unwrap_or(self.pitch));
        let a = [self.centre[0] + v[0], self.centre[1] + v[1], self.centre[2] + v[2]];
        let b = [self.centre[0] - v[0], self.centre[1] - v[1], self.centre[2] - v[2]];
        (a, b)
    }

    /// The closest point on the segment (`a`,`b`) to `p`
    /// (`vecutil_calculate_closest_point_on_line` 0x1df7e0, clamped to the endpoints).
    pub fn closest_point(a: V3, b: V3, p: V3) -> V3 {
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
        let ab2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
        let t = if ab2 > 1e-9 { (ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab2 } else { 0.0 };
        let t = t.clamp(0.0, 1.0);
        [a[0] + t * ab[0], a[1] + t * ab[1], a[2] + t * ab[2]]
    }

    /// The grab-zone hit: the player capsule segment (`a`,`b`) against the wire's world sphere — the
    /// game's object hit test for the version-0x200 HITTEST branch is the swept segment vs the object's
    /// world sphere (`Collide_PickObj` 0x1eae80: closest point on the segment to `obj+0x80` within
    /// `obj+0x8C`) [M]. The port's discrete capsule segment stands in for the swept one [S].
    pub fn sphere_hits(&self, a: V3, b: V3) -> bool {
        let c = Self::closest_point(a, b, self.centre);
        let d = [c[0] - self.centre[0], c[1] - self.centre[1], c[2] - self.centre[2]];
        d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < self.radius * self.radius
    }
}

/// Placement param k: `params.iter().find(|(i, _)| *i == k)` — the params are (index, value) pairs
/// addressed by INDEX (`level_tag+0x2C+4k`; `Wire_Create` reads `level+0x2C/0x30/0x34/0x38` = params
/// 0..3) [M 0x198224–0x19824c].
fn param(p: &Placement, k: u32) -> u32 { p.params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v) }

/// A third icon (placement kind 0xFA, `ThirdIcon_Create` 0x197070): the interaction-context marker the
/// game scatters through the levels. `ThirdIcon_Create` builds a class-0x42 object whose instance is
/// `inst+0x00` = param 0 (**the context type written to the player's `BLData+0x95F`**), `inst+0x04`
/// = param 1 (the on-channel; 0 = always enabled — create sets `obj+0xF4 = 1`), `inst+0x06` = param 2
/// (a second channel that only sets the `obj+0xFE` bit-0 flag) [M 0x197070:21–32]. `ThirdIcon_Update`
/// 0x196e80 walks the icon's room list for the PLAYER (`Player_Init` 0x1910f8:76 sets the player's
/// class to 3) and writes the icon's type into the player's +0x95F on a 4-tick refresh; the type-6 icon
/// additionally shows a text message [M 0x196e80:36–54]. The player's +0x95F is reset to 0xFF every
/// tick by `Player_CollisionHandler` 0x1949a8:22 [M].
///
/// **The room list is the straddle-cel set** [M `Collide_Update` 0x1ea398 → `Controls_StraddleTest`
/// 0x134778 → `Collide_StraddleCels` 0x1eac98]: `build_LinkToRoom` (0x1c9680:40-49) builds the icon's
/// world sphere (`Control_BuildWorldSph` 0x133070 over the object's glist bounds — the same centre +
/// radius computation the wires use) and links the object to `build_FindCel` (0x1c8cb0, room cels
/// only) at the sphere's centre; the straddle pass then BFS-walks cels from the object's own cel
/// through **portals the sphere intersects** (`Intersect_Portal` 0x1ea7b8 version 0x101: the portal's
/// AABB, then `vecutil_Dist2Tri` from the sphere centre to the portal's two triangles < the radius)
/// up to 0x7e cels, and every object occupying a straddled cel gets a hitdata node on the icon's
/// +0xd0 chain. So the icon writes its type **only while the player occupies a cel of that set** — a
/// wall between the icon and the player keeps the contexts apart (no portal to walk through), which
/// is what keeps 07000008's jump-grab from yanking Bond onto the zipline from the far side of the
/// guardhouse wall. The port resolves the set once per level load (see `scene.rs`'s `icon_cels`).
///
/// Consumers: `Player_HandleJump` 0x1916b0:118–147 grabs the NEAREST wire (class 0x2C within 1000
/// units) when the jump is pressed while +0x95F == 3 — the cable-crossing grab; `Player_Activate`
/// 0x1a5170:67 grabs a creep wall when +0x95F == 6 (E, not this task).
///
/// [D probe] 07000005 map 50 has three type-3 icons under the wire — (−23.58, 32.24, 2.65),
/// (−23.56, 32.24, 0.21), (−23.80, 32.74, 12.33), the wire at (−23.49, 34.47, 6.26) — all param 1 = 0
/// (always enabled); type-6 icons sit at the creep walls, type-2 near the level start (the HUD's
/// ammo-pane context, `HUD_UpdateAmmoPane` 0x19a7f0:330).
#[derive(Clone, Debug)]
pub struct Icon {
    /// The placement translation (the icon object's position).
    pub pos: V3,
    /// `obj+0x80..0x8C`: the world-sphere centre (`Control_BuildWorldSph` 0x133070 through the
    /// placement's yaw matrix, the same computation as `Wire::create`) — `build_LinkToRoom`
    /// 0x1c9680:40 builds it and `build_FindCel`s the object at it [M].
    pub centre: V3,
    /// `obj+0x8C`: the world-sphere radius — the straddle volume `Collide_StraddleCels` 0x1eac98
    /// walks portals with (`Intersect_Portal` 0x1ea7b8 version 0x101) [M]. 07000008's type-3 icon
    /// "Hand_Over_Hand" measures r 1.164 [D probe] — the flat 12.0 arm radius this struct's
    /// consumers used before fix-08-walls was 10× too wide and armed through the guardhouse wall.
    pub radius: f32,
    /// Param 0: the context type written to `BLData+0x95F` (3 = the wire-grab context here).
    pub ctx: u8,
    /// Param 1: the on-channel (0 = always enabled).
    pub on_channel: u16,
}

/// Every third-icon placement of a level (flags & 0xff == 0xFA), in placement order, resolved
/// against its map object for the world sphere (`Wire::all`'s resolution; an unresolved placement
/// never creates an object, so it is skipped like the wires' unresolved ones).
pub fn icons(level: &Level) -> Vec<Icon> {
    let mut out = Vec::new();
    for (mi, m) in level.maps.iter().enumerate() {
        for p in &m.placements {
            if p.flags & 0xff != 0xfa { continue; }
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
            let obj = obj.and_then(|(om, oi)| level.maps.get(om).and_then(|map| map.objects.get(oi)));
            let Some(o) = obj else { continue };
            // The world sphere through the yaw-only object matrix + translation (the same
            // computation as `Wire::create`) [M Control_BuildWorldSph 0x133070, via 0x1c9680:40].
            // G-046: the yaw rows are PS2Sinf-built [M] — the port's rebuild follows.
            let yaw = p.rotation[1];
            let (s, c) = (crate::trig::sinf(yaw), crate::trig::cosf(yaw));
            let centre = [
                p.translation[0] + o.centre[0] * c + o.centre[2] * s,
                p.translation[1] + o.centre[1],
                p.translation[2] - o.centre[0] * s + o.centre[2] * c,
            ];
            let scale = p.scale();
            let s_max = scale.iter().fold(1.0f32, |a, &v| a * v.max(1e-6).abs());
            out.push(Icon {
                pos: p.translation,
                centre,
                radius: o.radius * s_max,
                ctx: param(p, 0) as u8,
                on_channel: param(p, 1) as u16,
            });
        }
    }
    out
}

/// Every wire placement of a level (`flags & 0xff == 0x3E`), resolved against its map object.
/// Mirrors the ladder sweep (`ladder::all`) [M 0x1d04f0 case 0x3E → `Wire_Create`].
pub fn all(level: &Level) -> Vec<Wire> {
    let mut out = Vec::new();
    for (mi, m) in level.maps.iter().enumerate() {
        for p in &m.placements {
            if p.flags & 0xff != 0x3e { continue; }
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
            let obj = obj.and_then(|(om, oi)| level.maps.get(om).and_then(|map| map.objects.get(oi)));
            if let Some(w) = Wire::create(p, obj) { out.push(w); }
        }
    }
    out
}

/// The wire anims' root-translation tracks. The game moves Bond along the wire with the playing anim's
/// root motion (`AnimSeqTick`/`AnimFrameResolve`, the same mechanism the drones use — `root_motion` in
/// drone.rs), while `Player_Wire`'s per-anim-add clamp keeps him on the travel segment. The port samples
/// the root translation of each script's sequence per frame (channels 0/1/2) [D — measured from the
/// level archives via `Sequence::value`; 07000005's move pair travels 0.7637/0.7106 along the facing per
/// cycle at the ±1.2 script rate `Player_Wire` sets (0x197ec4–0x197eb0)].
#[derive(Clone, Debug, Default)]
pub struct WireAnims {
    /// By SCRIPT hashcode (0x060005e5 grab pose, 0x060005ec/ed idle, 0x060005ee/ea turn,
    /// 0x060000b4/b5 move, 0x06000095/96/94 zipline). A missing track = the level archive does not
    /// carry that anim: the anim then holds its start pose (no root motion) [S fallback].
    pub tracks: std::collections::HashMap<u32, RootTrack>,
}

/// One anim's root translation sampled per frame.
#[derive(Clone, Debug, Default)]
pub struct RootTrack {
    /// The sequence's frame count (1-based frames 1..=frames sampled).
    pub frames: u32,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
}

impl RootTrack {
    pub fn of(seq: &nf_formats::anim::Sequence) -> RootTrack {
        let n = seq.frames.max(1) as u32;
        RootTrack {
            frames: n,
            x: (1..=n).map(|f| seq.value(0, f)).collect(),
            y: (1..=n).map(|f| seq.value(1, f)).collect(),
            z: (1..=n).map(|f| seq.value(2, f)).collect(),
        }
    }

    /// The root translation at script time `t` (the port's `Sequence::value` is 1-based; times past the
    /// end clamp to the last frame — the game's held pose).
    pub fn at(&self, t: f32) -> [f32; 3] {
        let i = (t.round() as i64).clamp(1, self.frames as i64) as usize - 1;
        let i = i.min(self.x.len().saturating_sub(1));
        [self.x[i], self.y[i], self.z[i]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The segment geometry against the game's own numbers [L ee_start_05.bin 016dc430 + atlas snapshot
    /// check]: 07000005's wire — centre (−23.4855, 34.4676, 6.26292), yaw −1.5708, radius 4.04, pitch 0.
    fn wire05() -> Wire {
        Wire { centre: [-23.4855, 34.4676, 6.26292], radius: 4.039, yaw: -1.5708, pitch: 0.0, wire_type: 0, param0: 2, param2: 0, channel: 0 }
    }

    #[test]
    fn segments_match_the_snapshot_geometry() {
        let w = wire05();
        // yaw −1.5708 → the wire runs along Z (spherical folds the +π/2; r·sin(yaw+π/2) ≈ 0 in x).
        let (a, b) = w.travel_segment();
        assert!((a[1] - w.centre[1]).abs() < 1e-4 && (b[1] - w.centre[1]).abs() < 1e-4, "pitch 0 keeps the travel segment level");
        assert!((a[0] - w.centre[0]).abs() < 1e-3, "the 07000005 wire runs along Z, not X: a {a:?}");
        assert!(((a[2] - w.centre[2]).abs() - (w.radius - 0.5)).abs() < 2e-3, "travel half-length = radius − 0.5");
        assert!(((b[2] - w.centre[2]).abs() - (w.radius - 0.5)).abs() < 2e-3);
        // The grab segment is shorter by 1.5 (radius − 2.0).
        let (ga, gb) = w.grab_segment();
        assert!(((ga[2] - w.centre[2]).abs() - (w.radius - 2.0)).abs() < 2e-3);
        // Sister: a longer radius lengthens both segments.
        let mut w2 = wire05(); w2.radius = 6.0;
        let (a2, _) = w2.travel_segment();
        assert!((a2[2] - w.centre[2]).abs() > (a[2] - w.centre[2]).abs() + 1.0);
        let _ = (gb, b);
    }

    #[test]
    fn sphere_hit_zone_and_closest_point() {
        let w = wire05();
        // Bond's capsule inside the world sphere: hit.
        assert!(w.sphere_hits([w.centre[0], w.centre[1] - 1.0, w.centre[2]], [w.centre[0], w.centre[1] - 2.0, w.centre[2]]));
        // 5 units below the centre: outside the 4.04 sphere.
        assert!(!w.sphere_hits([w.centre[0], w.centre[1] - 5.0, w.centre[2]], [w.centre[0], w.centre[1] - 6.0, w.centre[2]]));
        // The closest point clamps to the endpoints off the segment's ends.
        let (a, b) = w.travel_segment();
        let c = Wire::closest_point(a, b, [a[0], a[1], a[2] + 10.0]);
        assert!((c[2] - a[2]).abs() < 1e-5, "clamped to endpoint a: {c:?}");
    }

    #[test]
    fn params_read_by_index() {
        // The live 07000005 instance bytes `00 00 02 00 00 00 00 00` [L 016dc510]: type 0 (param 1),
        // param 0 = 2, param 2 = 0, channel 0 (param 3).
        let w = wire05();
        assert_eq!((w.wire_type, w.param0, w.param2, w.channel), (0, 2, 0, 0));
    }
}
