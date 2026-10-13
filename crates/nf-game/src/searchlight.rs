//! Searchlights (docs/systems/world-objects.md "Searchlights"): `Searchlight_Create` 0x1cb4f8 /
//! `Searchlight_Update` 0x1cb900, ported from the disassembly [M] and checked against the live layout in
//! `work/ee_start_05.bin` [L] (29 parts on 07000005: 7 mounts, 7 heads, 6 beams, 7 coronas, 2 spots).
//!
//! Not ported yet (G-033): the spot's dynamic light (illumination is renderer work), `Gas_Init` smoke for
//! broken lights (no particle system; its 6 RNG draws per puff are therefore not consumed either), and the
//! bullet child-list (`obj+0xD0`, U-041) — bullets are tested against the solid parts' boxes instead.

use crate::drone::{Data, Player, Rng};
use crate::trig::{angle_difference, atan2_approx, sinf, PI, TAU};
use nf_formats::placement::{object_matrix, Placement};
use nf_formats::ps2float::{add_noguard, mul};

pub type V3 = [f32; 3];
pub type Mat4 = [[f32; 4]; 4];

/// The game's degrees→radians constant (`lui 3c8e / ori fa36` at 0x1cb604) — π/180 plus one ulp [M].
pub const DEG2RAD: f32 = f32::from_bits(0x3c8e_fa36);
/// The sweep step 2π/480 (0x3c567751 at 0x1cba68) [M].
const SWEEP: f32 = f32::from_bits(0x3c56_7751);
/// The tracking gain 0.15 (0x3e19999a at 0x1cbb40) [M].
const STEER: f32 = f32::from_bits(0x3e19_999a);
/// `FRAME_RATE` (read from 0x31d0d0) × 15 s: ticks the spot may lose Bond before the head sweeps again [M].
const LOST_LIMIT: f32 = 60.0 * 15.0;

/// One searchlight part object. Kinds [M]: 0 mount, 1 head (the controller), 2 spot (ground circle + light),
/// 3 beam (the visible cone), 4 corona (billboard at the lamp).
#[derive(Clone, Debug)]
pub struct Part {
    /// Index into the main map's placements (nf-view's stand-in for the object's glist).
    pub placement: usize,
    pub kind: u8,
    /// `obj+0x30`. A spot is moved −10000 at create until the first ray hit repositions it [M].
    pub pos: V3,
    /// The placement's rotation.y (`instance+0x14`): the sweep's yaw centre.
    pub base_yaw: f32,
    /// `instance+0x34/35/36`: on/off/alarm switch channels.
    pub on: u8, pub off: u8, pub alarm: u8,
    /// `instance+0x38/3c`: half-range and mid angle (radians).
    pub half: f32, pub mid: f32,
    /// `instance+4`: the head's sweep phase / the spot's ticks-not-seen counter.
    pub phase: f32,
    /// `obj+0xFC`: drawn (the game's `View_SetDrawInAllViews` / `…NoViews`).
    pub drawn: bool,
    /// `obj+0x90`: the part's world matrix (heads and mounts via `Mat_Align2Dir` against world up, the beam
    /// against Bond, the spot from the hit normal; coronas only carry the position).
    pub matrix: Mat4,
    /// Heads: `obj+0xF4`. 0 sweep, 1 tracking, 2 broken, 3 off.
    pub state: u8,
    /// Heads: `obj+0x50 / +0x54`.
    pub pitch: f32, pub yaw: f32,
    /// Heads: the linked parts by kind (`instance+0x20 + 4·kind`).
    pub links: [Option<usize>; 5],
    /// The off-channel kill is one-shot (`obj+0xFE |= 1`): the light can never re-light.
    pub killed: bool,
    /// Spots: the light node's flicker seed (`Rand_Rand(0)` at create); `None` only after a kill deletes it.
    pub light: Option<u32>,
    /// Coronas: the alpha byte (`obj+0x106`) and scale (`obj+0xE8`).
    pub alpha: u8, pub scale: f32,
    /// Heads and mounts: the object's gfx box (bullet stand-in; the game's child-list path is U-041).
    pub box_min: V3, pub box_max: V3,
}

/// All searchlight parts of a level, in placement (creation) order.
#[derive(Clone, Debug, Default)]
pub struct Searchlights {
    pub parts: Vec<Part>,
}

/// `Searchlight_Create`'s part kinds for a placement list, in creation order (used by create and nf-view).
pub fn part_placements(placements: &[Placement]) -> Vec<usize> {
    placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xde && p.flags & 0xa000 == 0).map(|(i, _)| i).collect()
}

/// The create-time (p4, p5) → (half, mid), operation for operation [M, 0x1cb5c0–0x1cb680; L 7/7 heads exact]:
/// `lo = float(min)·DEG2RAD`, `hi = float(max)·DEG2RAD` (EE muls), `mid = (hi+lo)·0.5`, `half = (hi−lo)·0.5`
/// (EE add/sub; the halving is exact). Not `(max−min)·π/360` — the constant and the op order both differ.
pub fn half_mid(p4: u32, p5: u32) -> (f32, f32) {
    let (lo, hi) = if p4 <= p5 { (p4 as f32, p5 as f32) } else { (p5 as f32, p4 as f32) };
    let lo = mul(lo, DEG2RAD);
    let hi = mul(hi, DEG2RAD);
    let mid = mul(add_noguard(hi, lo, true), 0.5);
    let half = mul(add_noguard(hi, -lo, true), 0.5);
    (half, mid)
}

/// The state-0 sweep (`Searchlight_Update` 0x1cba60) [M; L five golden heads bit-exact]: pitch and yaw from the
/// phase, then the object's angles normalised to (−π, π] — the live heads with mid > π store the pitch 2π below
/// the raw sum (head 016e03f0: sinf·half + mid = 4.445 stored as −1.838) [L]. `phase` is the value *before* this
/// tick's advance (`+= FRAME_RATE_MUL`); the `fmodf(phase, 480)` call's result is discarded (the store sits in
/// the jal's delay slot) — the phase grows unbounded and `PS2Sinf`'s own wrap keeps precision [M].
pub fn sweep(phase: f32, half: f32, mid: f32, base_yaw: f32) -> (f32, f32) {
    let x = mul(phase, SWEEP);
    let mut pitch = add_noguard(mul(sinf(x), half), mid, true);
    let mut yaw = add_noguard(mul(sinf(add_noguard(x, crate::trig::HALF_PI, true)), half), base_yaw, true);
    while pitch > PI { pitch = add_noguard(pitch, -TAU, true); }
    while pitch <= -PI { pitch = add_noguard(pitch, TAU, true); }
    while yaw > PI { yaw = add_noguard(yaw, -TAU, true); }
    while yaw <= -PI { yaw = add_noguard(yaw, TAU, true); }
    (pitch, yaw)
}

/// `Vec_Spherical_2_Cartesian` (0x1e3948) [M]: `(r·sin(yaw)·cos(p), r·sin(p), r·cos(yaw)·cos(p))`, the sines
/// from the game's `PS2Sinf` and the products EE-chopped.
pub fn spherical(r: f32, yaw: f32, p: f32) -> V3 {
    let (sy, cy, sp, cp) = (sinf(yaw), crate::trig::cosf(yaw), sinf(p), crate::trig::cosf(p));
    [mul(mul(r, sy), cp), mul(r, sp), mul(mul(r, cy), cp)]
}

fn sub3(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot3(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross3(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }

/// `Mat_Align2Dir` (0x1df2a8) [M; cross order pinned against the live beam matrix 016e0130, L]:
/// row 2 = normalize(dir), row 0 = normalize(cross(up, row2)), row 1 = normalize(cross(row2, row0)). The
/// fallback replaces `up` when |up·row2| > 0.999; a degenerate cross leaves the last good axis.
pub fn align2dir(dir: V3, up: V3, fallback: V3) -> Mat4 {
    let len = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
    let z = if len == 0.0 { [0.0, 0.0, 1.0] } else { [dir[0] / len.sqrt(), dir[1] / len.sqrt(), dir[2] / len.sqrt()] };
    let ulen = up[0] * up[0] + up[1] * up[1] + up[2] * up[2];
    let un = if ulen == 0.0 { [0.0, 1.0, 0.0] } else { [up[0] / ulen.sqrt(), up[1] / ulen.sqrt(), up[2] / ulen.sqrt()] };
    let u = if (dot3(un, z)).abs() > 0.999 { fallback } else { un };
    let r0 = cross3(u, z);
    let r0l = r0[0] * r0[0] + r0[1] * r0[1] + r0[2] * r0[2];
    let r0 = if r0l < 1e-20 { [1.0, 0.0, 0.0] } else { [r0[0] / r0l.sqrt(), r0[1] / r0l.sqrt(), r0[2] / r0l.sqrt()] };
    let r1 = cross3(z, r0);
    let r1l = r1[0] * r1[0] + r1[1] * r1[1] + r1[2] * r1[2];
    let r1 = if r1l < 1e-20 { [0.0, 1.0, 0.0] } else { [r1[0] / r1l.sqrt(), r1[1] / r1l.sqrt(), r1[2] / r1l.sqrt()] };
    [[r0[0], r0[1], r0[2], 0.0], [r1[0], r1[1], r1[2], 0.0], [z[0], z[1], z[2], 0.0], [0.0, 0.0, 0.0, 0.0]]
    // Row 3 stays zeroed as in the live matrix [L]: `Mat_Align2Dir` writes only the rotation rows and
    // `Mat_SetTrans` the translation's x/y/z — the w slot keeps its cleared value. The port's `transform`
    // ignores w.
}

impl Searchlights {
    /// `Searchlight_Create` for every 0xDE placement of the main map, in placement order [M]. Each part draws
    /// `Rand_Rand(480)` for its phase; each spot another `Rand_Rand(0)` for its light's flicker seed — both from
    /// the shared stream, in this order [M, critical]. The linking walk sees the parts created so far (the cel
    /// list, newest first): a spot created before its head is moved −10000 by its own create and can never link
    /// (the measured orphan rule, L: spot 016e8ec0 on 07000005).
    pub fn create(placements: &[Placement], rng: &mut Rng, bounds: impl Fn(&Placement) -> (V3, V3)) -> Searchlights {
        let mut s = Searchlights::default();
        for i in part_placements(placements) {
            let p = &placements[i];
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
            let kind = param(0) as u8;
            let phase = rng.rand(480) as f32;
            let (box_min, box_max) = bounds(p);
            let mut part = Part {
                placement: i, kind, pos: p.translation, base_yaw: p.rotation[1],
                on: param(1) as u8, off: param(2) as u8, alarm: param(3) as u8,
                half: 0.0, mid: 0.0, phase, drawn: kind < 2,
                matrix: object_matrix(p.rotation, p.translation),
                state: if param(1) != 0 { 3 } else { 0 },
                pitch: 0.0, yaw: p.rotation[1], links: [None; 5], killed: false,
                light: None, alpha: 0xff, scale: 1.0, box_min, box_max,
            };
            let (half, mid) = half_mid(param(4), param(5));
            part.half = half;
            part.mid = mid;
            // The linking loop [M, disassembly 0x1cb6c0]: the cel list, newest first (`control_link_object_to_cel`
            // prepends); skip self, skip class ≠ 0x22, link only within 3 units (strict). Both stores are
            // unconditional, so the walk (newest → oldest) leaves the OLDEST match linked. Cels are approximated
            // by the distance here: the parts of one light sit within ~1.5 units and different lights are far
            // apart (07000005's measured links reproduce). The kind-2 branch (the light node and the −10000
            // move) runs AFTER this walk [M, 0x1cb79c], so a spot created after its head links while it is
            // still in place; a spot created before its head is moved away by its own create and the later
            // head's walk never sees it (the measured orphan rule, L: spot 016e8ec0 on 07000005).
            let me = s.parts.len();
            for j in (0..me).rev() {
                let d = sub3(s.parts[j].pos, part.pos);
                if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] >= 9.0 { continue; }
                if part.kind == 1 {
                    part.links[s.parts[j].kind as usize] = Some(j);
                    // A dark head's non-mount parts start dark (`View_SetDrawInNoViews`) [M].
                    if part.on != 0 && s.parts[j].kind != 0 { s.parts[j].drawn = false; }
                } else if s.parts[j].kind == 1 {
                    s.parts[j].links[part.kind as usize] = Some(me);
                    if s.parts[j].on != 0 && part.kind != 0 { part.drawn = false; }
                }
            }
            if kind == 2 {
                // The light node (`psiLight_Create`): radius 500, type 2, the flicker seed `Rand_Rand(0)`
                // (the second draw of this part's create) [M]. Illumination is renderer work (G-033).
                part.light = Some(rng.rand(0));
                // The spot waits below the world for its first ray hit [M].
                part.pos = [part.pos[0] - 10000.0, part.pos[1] - 10000.0, part.pos[2] - 10000.0];
                part.drawn = false;
                part.matrix = object_matrix(p.rotation, part.pos);
            }
            s.parts.push(part);
        }
        s
    }

    /// `Searchlight_Update` for one head, in the control list's order (called from `Drones::tick`). Returns the
    /// `Sound_Play3D` events (id at position): 0xE5 the relay clunk when a channel re-lights the head, 0xE6 the
    /// glass break when a bullet kills it [M].
    pub fn update(&mut self, d: &Data, player: &Player, channels: &mut crate::triggers::Channels, head: usize,
                  bullets: &[(V3, V3)], rng: &mut Rng, frame: u32) -> Vec<(u32, V3)> {
        let mut sounds = Vec::new();
        let (pos, base_yaw, half, mid, on, off) = {
            let h = &self.parts[head];
            (h.pos, h.base_yaw, h.half, h.mid, h.on, h.off)
        };
        // The kill check, any state [M]: the off-channel deletes the light, hides the parts for good and clears
        // the links and channel bytes — the head can never re-light.
        if off != 0 && channels.on(off as u32) {
            let links = self.parts[head].links;
            {
                let h = &mut self.parts[head];
                h.state = 3;
                h.killed = true;
                h.links = [None; 5];
                h.on = 0;
                h.off = 0;
            }
            for p in links.into_iter().flatten() {
                self.parts[p].drawn = false;
                if self.parts[p].kind == 2 { self.parts[p].light = None; }
            }
            return sounds;
        }
        let state = self.parts[head].state;
        let mut parts_now = false; // run the spot/beam/corona updates this tick (states 0/1 and the 3→0 turn-on)
        match state {
            // State 0: the sweep [M].
            0 => {
                let h = &mut self.parts[head];
                let (pitch, yaw) = sweep(h.phase, half, mid, base_yaw);
                h.pitch = pitch;
                h.yaw = yaw;
                h.phase += 1.0; // FRAME_RATE_MUL; the game's fmodf result is discarded (store in the delay slot)
                parts_now = true;
            }
            // State 1: hold the alarm channel and steer to Bond [M, disassembly 0x1cbb00].
            1 => {
                channels.set(self.parts[head].alarm as u32, 1, frame);
                let h = &mut self.parts[head];
                let dx = player.pos[0] - pos[0];
                let dy = player.pos[1] - pos[1];
                let dz = player.pos[2] - pos[2];
                // libm atan2f (not ATAN2_APPROX); last-ulp differences only steer a frame differently.
                let t_yaw = dx.atan2(dz);
                let t_pitch = -(dy - 1.0).atan2((dx * dx + dz * dz).sqrt());
                h.yaw += angle_difference(h.yaw, t_yaw) * STEER;
                h.pitch += angle_difference(h.pitch, t_pitch) * STEER;
                // Clamped to the sweep cone: yaw within base ± half, pitch within mid ± half.
                let dyaw = angle_difference(h.yaw, base_yaw);
                if dyaw > half { h.yaw = base_yaw - half; } else if dyaw < -half { h.yaw = base_yaw + half; }
                let dpitch = angle_difference(h.pitch, mid);
                if dpitch > half { h.pitch = mid - half; } else if dpitch < -half { h.pitch = mid + half; }
                parts_now = true;
            }
            // State 2: broken — the head droops, nothing else updates [M].
            2 => {
                let h = &mut self.parts[head];
                if h.pitch < 0.8 { h.pitch += 0.05; }
                // Gas_Init smoke every 7th drawn frame is not ported (no particle system); its RNG draws are
                // not consumed either (G-033).
                let _ = rng;
            }
            // State 3: off — the on-channel re-lights it with the relay clunk [M].
            _ => {
                if on != 0 && channels.on(on as u32) {
                    let links = self.parts[head].links;
                    self.parts[head].state = 0;
                    for p in links.into_iter().flatten() { self.parts[p].drawn = true; }
                    sounds.push((0xe5, pos));
                    parts_now = true; // the turn-on falls straight into the part updates [M]
                }
            }
        }
        if !parts_now {
            // The bullet check still runs in states 0/1/3 (the tail of the update) [M].
            self.bullet_check(head, bullets, &mut sounds);
            return sounds;
        }
        // The mount follows the head's yaw [M; L: mount 016dffd0's yaw = its head's].
        if let Some(m) = self.parts[head].links[0] {
            let yaw = self.parts[head].yaw;
            self.parts[m].yaw = yaw;
        }
        // The head's and mount's matrices: the framework's `Mat_Align2Dir` of the aim (the mount's pitch is 0)
        // against world up — [L] pinned on the snapshot's head 016dfe70, whose row 2 is the aim direction and
        // row 0 ⊥ with a zero y component.
        {
            let (yaw, pitch) = (self.parts[head].yaw, self.parts[head].pitch);
            let mut m = align2dir(spherical(1.0, yaw, -pitch), [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]);
            m[3] = [pos[0], pos[1], pos[2], 0.0];
            self.parts[head].matrix = m;
        }
        if let Some(m) = self.parts[head].links[0] {
            let (p, yaw) = (self.parts[m].pos, self.parts[m].yaw);
            let mut mm = align2dir(spherical(1.0, yaw, 0.0), [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]);
            mm[3] = [p[0], p[1], p[2], 0.0];
            self.parts[m].matrix = mm;
        }
        // The spot [M]: a 100-unit ray along the aim; on a hit the spot sits on the surface, oriented by the
        // hit normal. No hit: hidden. The ray's pick flags 0x18 do NOT exclude the player's class [M
        // Collide_PickObj 0x1eae80:26-40 — the class-3 exclusion is mask 0x400, absent from 0x18], so when
        // Bond stands in the beam the ray hits HIS world sphere (`obj+0x80` centre, `obj+0x8C` radius) first
        // and the spot lands on him — that is what makes `Vec_Dist3D(spot, player) <= 2.0` [M 0x1cb900:251]
        // arm the light in play at shallow beam angles, where a static-only ray would run past him to a far
        // wall. The port adds the ray-vs-sphere test on top of the static segment.
        if let Some(sp) = self.parts[head].links[2] {
            let (yaw, pitch) = (self.parts[head].yaw, self.parts[head].pitch);
            let dir = spherical(100.0, yaw, -pitch);
            let end = [pos[0] + dir[0], pos[1] + dir[1], pos[2] + dir[2]];
            // The player's pick sphere: centre `obj+0x80` (the `Player.sph` the port fills from
            // `Bond::sphere_centre`), radius `M_CONTROL+0x8C` = 1.0576 [L ee_start_05.bin].
            const PLAYER_SPH_R: f32 = 1.0576;
            let (mut t_hit, mut n_hit) = match d.world.segment_surface(pos, end, false) {
                Some((t, n, _)) => (Some(t), Some(n)),
                None => (None, None),
            };
            {
                // The ray is `pos + dir·t` with |dir| = 100 (spherical's length), t ∈ [0, 1] on the segment.
                let oc = [pos[0] - player.sph[0], pos[1] - player.sph[1], pos[2] - player.sph[2]];
                let a = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
                let b = oc[0] * dir[0] + oc[1] * dir[1] + oc[2] * dir[2];
                let c = oc[0] * oc[0] + oc[1] * oc[1] + oc[2] * oc[2] - PLAYER_SPH_R * PLAYER_SPH_R;
                let disc = b * b - a * c;
                if disc >= 0.0 {
                    let t = (-b - disc.sqrt()) / a;
                    if t > 0.0 && t_hit.is_none_or(|tw| t < tw) {
                        let p = [pos[0] + dir[0] * t, pos[1] + dir[1] * t, pos[2] + dir[2] * t];
                        let inv = 1.0 / PLAYER_SPH_R;
                        n_hit = Some([(p[0] - player.sph[0]) * inv, (p[1] - player.sph[1]) * inv, (p[2] - player.sph[2]) * inv]);
                        t_hit = Some(t);
                    }
                }
            }
            if let (Some(t), Some(n)) = (t_hit, n_hit) {
                let hit = [pos[0] + (end[0] - pos[0]) * t, pos[1] + (end[1] - pos[1]) * t, pos[2] + (end[2] - pos[2]) * t];
                let spot = &mut self.parts[sp];
                spot.pos = [hit[0] + n[0] * 0.01, hit[1] + n[1] * 0.01, hit[2] + n[2] * 0.01];
                spot.drawn = true;
                let a = atan2_approx(n[2], (n[0] * n[0] + n[2] * n[2]).sqrt());
                spot.matrix = object_matrix([a, crate::trig::HALF_PI - a, atan2_approx(n[0], n[2])], spot.pos);
            } else {
                self.parts[sp].drawn = false;
            }
            // Bond inside the circle arms the light; while tracking, the spot counts the ticks he is not [M].
            let sp_pos = self.parts[sp].pos;
            let dx = sp_pos[0] - player.pos[0];
            let dy = sp_pos[1] - player.pos[1];
            let dz = sp_pos[2] - player.pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if self.parts[head].state == 0 && dist <= 2.0 {
                self.parts[head].state = 1;
                self.parts[sp].phase = 0.0;
            } else if self.parts[head].state == 1 {
                if dist <= 2.0 {
                    self.parts[sp].phase = 0.0;
                } else {
                    let lost = self.parts[sp].phase + 1.0;
                    self.parts[sp].phase = lost;
                    if lost >= LOST_LIMIT { self.parts[head].state = 0; }
                }
            }
        }
        // The beam [M]: aligned to the aim, up = Bond − head (pinned against the live beam matrix, L).
        if let Some(b) = self.parts[head].links[3] {
            let (yaw, pitch) = (self.parts[head].yaw, self.parts[head].pitch);
            let up = sub3(player.pos, pos);
            let mut m = align2dir(spherical(1.0, yaw, -pitch), up, up);
            m[3] = [pos[0], pos[1], pos[2], 0.0];
            let beam = &mut self.parts[b];
            beam.matrix = m;
            beam.drawn = true;
        }
        // The corona [M, disassembly 0x1cc03c]: a billboard 0.4 along the aim, visible when the head is drawn
        // and the aim points within ~37° of Bond (dot > 0.2), fading with the angle and growing when in sight.
        if let Some(c) = self.parts[head].links[4] {
            let (yaw, pitch) = (self.parts[head].yaw, self.parts[head].pitch);
            let off = spherical(0.4, yaw, -pitch);
            let corona_pos = [pos[0] + off[0], pos[1] + off[1], pos[2] + off[2]];
            let to = sub3(player.eye, pos);
            let len = to[0] * to[0] + to[1] * to[1] + to[2] * to[2];
            let dot = dot3(off, to) / len.sqrt();
            let head_drawn = self.parts[head].drawn;
            if !head_drawn || dot <= 0.2 {
                self.parts[c].drawn = false;
            } else {
                let a = (dot - 0.2) * 4.0;
                let scale = a * 4.0;
                let alpha = (a * 255.0) as u8;
                let mut scale = scale;
                if alpha != 0 && d.world.line_of_sight(player.eye, corona_pos) { scale *= 1.5; }
                let corona = &mut self.parts[c];
                corona.pos = corona_pos;
                corona.drawn = true;
                corona.alpha = alpha;
                corona.scale = scale;
                corona.matrix = object_matrix([0.0; 3], corona_pos);
            }
        }
        // The tail's bullet check (states 0/1/3) [M].
        self.bullet_check(head, bullets, &mut sounds);
        sounds
    }

    /// The child-list walk's stand-in (U-041): a player bullet segment crossing the head's or mount's box
    /// breaks the light — state 2, the light deleted, the parts hidden, the glass sound [M for the effects].
    fn bullet_check(&mut self, head: usize, bullets: &[(V3, V3)], sounds: &mut Vec<(u32, V3)>) {
        if bullets.is_empty() { return; }
        let pos = self.parts[head].pos;
        let mut hit = false;
        for &k in [head].iter().chain(self.parts[head].links[0].iter()) {
            let p = &self.parts[k];
            // The segment against the part's oriented box: world → local (the matrix is a rotation and a
            // translation, so the inverse is the transposed rotation with the negated translation).
            let to_local = |v: V3| {
                let d = [v[0] - p.matrix[3][0], v[1] - p.matrix[3][1], v[2] - p.matrix[3][2]];
                [0, 1, 2].map(|k| d[0] * p.matrix[k][0] + d[1] * p.matrix[k][1] + d[2] * p.matrix[k][2])
            };
            for &(from, to) in bullets {
                if seg_box(to_local(from), to_local(to), p.box_min, p.box_max) { hit = true; }
            }
        }
        if hit {
            let links = self.parts[head].links;
            self.parts[head].state = 2;
            for p in links.into_iter().flatten() {
                self.parts[p].drawn = false;
                if self.parts[p].kind == 2 { self.parts[p].light = None; }
            }
            sounds.push((0xe6, pos));
        }
    }
}

/// A segment against an axis-aligned box (both endpoints transformed into the box's frame by the caller).
fn seg_box(from: V3, to: V3, min: V3, max: V3) -> bool {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    for k in 0..3 {
        if d[k].abs() < 1e-12 {
            if from[k] < min[k] || from[k] > max[k] { return false; }
        } else {
            let mut a = (min[k] - from[k]) / d[k];
            let mut b = (max[k] - from[k]) / d[k];
            if a > b { std::mem::swap(&mut a, &mut b); }
            t0 = t0.max(a);
            t1 = t1.min(b);
            if t0 > t1 { return false; }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_mid_matches_the_live_heads() {
        // [L] The seven heads of 07000005: (p4, p5) → stored (half, mid) bits in ee_start_05.bin.
        let golden: [(u32, u32, u32, u32); 7] = [
            (358, 345, 0x3de8_56a0, 0x40c4_508c),
            (0, 35, 0x3e9c_61ab, 0x3e9c_61ab),
            (250, 320, 0x3f1c_61ac, 0x409f_2c8d),
            (250, 320, 0x3f1c_61ac, 0x409f_2c8d),
            (0, 40, 0x3eb2_b8c3, 0x3eb2_b8c3),
            (0, 40, 0x3eb2_b8c3, 0x3eb2_b8c3),
            (355, 345, 0x3db2_b8c0, 0x40c3_7a15),
        ];
        for &(p4, p5, half, mid) in &golden {
            let (h, m) = half_mid(p4, p5);
            assert_eq!((h.to_bits(), m.to_bits()), (half, mid), "p4 {p4} p5 {p5}");
        }
        // Sister: the naive (max−min)·π/360 disagrees in the last bits (three of the seven heads).
        let naive = |p4: u32, p5: u32| ((p4.max(p5) - p4.min(p5)) as f32 * std::f32::consts::PI / 360.0).to_bits();
        assert!(golden.iter().any(|&(p4, p5, h, _)| naive(p4, p5) != h), "sister");
    }

    #[test]
    fn sweep_matches_the_live_heads() {
        // [L] Golden sweep vectors (world-objects.md): the stored phase is post-advance, so compute with −1.
        let golden: [(u32, u32, u32, u32, u32, u32); 5] = [
            (0x439f_0000, 0x3e9c_61ab, 0x3e9c_61ab, 0x3f49_0fdb, 0x3d41_00a8, 0x3f1f_5694),
            (0x43a0_8000, 0x3f1c_61ac, 0x409f_2c8d, 0x3f49_0fdb, 0xbfeb_4450, 0x3ef5_be04),
            (0x4383_0000, 0x3f1c_61ac, 0x409f_2c8d, 0xbf37_3094, 0xbfbcc_698, 0xbfa6_d999),
            (0x438a_8000, 0x3eb2_b8c3, 0x3eb2_b8c3, 0x3f25_514d, 0x3e43_2adf, 0x3eab_6499),
            (0x4160_0000, 0x3eb2_b8c3, 0x3eb2_b8c3, 0xbee8_5696, 0x3ed0_fcf8, 0xbde0_cab0),
        ];
        for &(phase, half, mid, base, pitch, yaw) in &golden {
            let (p, y) = sweep(f32::from_bits(phase) - 1.0, f32::from_bits(half), f32::from_bits(mid), f32::from_bits(base));
            assert_eq!((p.to_bits(), y.to_bits()), (pitch, yaw), "phase {phase:#x}");
        }
        // Sister: swapping the yaw's half-range for the pitch's disagrees (head 2: half ≠ mid).
        let (p, y) = sweep(f32::from_bits(0x43a0_8000) - 1.0, f32::from_bits(0x409f_2c8d), f32::from_bits(0x3f1c_61ac), f32::from_bits(0x3f49_0fdb));
        assert_ne!((p.to_bits(), y.to_bits()), (0xbfeb_4450, 0x3ef5_be04), "sister");
    }

    #[test]
    fn align2dir_matches_the_live_beam() {
        // [L] Head 016dfe70 (yaw/pitch) aiming at Bond at the level start: the beam 016e0130's matrix bits.
        // The cross ORDER is what this pins (the mirrored order is ~2.0 off on row 0); the components match
        // within 2 ulp — the game's `Vec_Spherical_2_Cartesian` (PS2Sinf + FPU ops) and the VU0 MAC's single
        // rounding are not modelled to the last bit for this visual-only matrix (see docs/systems/render.md's
        // stance on last-ulp visual differences).
        let (yaw, pitch) = (f32::from_bits(0x3f1f_5694), f32::from_bits(0x3d41_00a8));
        let head = [f32::from_bits(0x4278_ab6a), f32::from_bits(0x4209_7269), f32::from_bits(0x42d6_0361)];
        let player = [f32::from_bits(0x42d1_e873), f32::from_bits(0x41c7_b4d7), f32::from_bits(0x42d9_a68a)];
        let up = sub3(player, head);
        let mut m = align2dir(spherical(1.0, yaw, -pitch), up, up);
        m[3] = [head[0], head[1], head[2], 0.0];
        let golden: [[u32; 4]; 4] = [
            [0xbe5e_bc78, 0xbf78_9016, 0x3dcc_3e74, 0],
            [0x3f48_8620, 0xbe70_3cbc, 0xbf13_5fb4, 0],
            [0x3f15_1500, 0xbd40_ee5d, 0x3f4f_c322, 0],
            [0x4278_ab6a, 0x4209_7269, 0x42d6_0361, 0],
        ];
        for r in 0..4 {
            for c in 0..4 {
                let (got, want) = (m[r][c].to_bits(), golden[r][c]);
                let ulp = got.abs_diff(want);
                assert!(ulp <= 4, "row {r} col {c}: {got:#x} vs {want:#x} ({ulp} ulp)");
            }
        }
        // Sister: the mirrored cross order (z × up) is far off row 0.
        let z = spherical(1.0, yaw, -pitch);
        let r0 = cross3(z, up);
        assert!((r0[0] - f32::from_bits(0xbe5e_bc78)).abs() > 0.1, "sister");
    }

    #[test]
    fn seg_box_hits_and_misses() {
        let (min, max) = ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
        assert!(seg_box([-5.0, 0.0, 0.0], [5.0, 0.0, 0.0], min, max));
        assert!(!seg_box([-5.0, 2.0, 0.0], [5.0, 2.0, 0.0], min, max));
        assert!(seg_box([0.5, 0.5, 0.5], [3.0, 3.0, 3.0], min, max), "starts inside");
        assert!(!seg_box([2.0, 0.0, 0.0], [3.0, 0.0, 0.0], min, max), "past it");
    }
}
