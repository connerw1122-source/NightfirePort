//! Doors (`docs/systems/world-objects.md` "Doors"): `Door_Create` 0x136100, `Door_SetupSwing` 0x135440,
//! `Door_SetState` 0x135618, `Door_Interp` 0x135738, `Door_ReactToHit` 0x135980, `Door_Activate` 0x135be0,
//! `Door_DoMsg` 0x135f18, `Doors_Calc` 0x135fe0, `Door_Update` 0x1363c8 — ported from the disassembly [M] and
//! checked against the live layout in `work/ee_start_05.bin` [L] (5 doors on 07000005: 4 `Door_Locked_3/4`
//! sliders without an anim and lock channel 200, 1 `Door_UNLocked` swing door with a 3-key anim).
//!
//! Not ported: the door→portal links (`Door_Link2Portal`, block 0x21), the text labels 0x2000003 / 0x200000b
//! (`Door_Update`'s unlock note, `Door_DoMsg` — no text system yet), and the collision-proxy list of
//! `Door_ReactToHit` (`obj+0xD0`, as U-041): the proxy scan is stood in for by the same bounding-sphere test
//! `Door_Update`'s proximity check uses.

use crate::triggers::Channels;
use crate::trig::sinf;
use nf_formats::placement::{object_matrix, Placement};

pub type V3 = [f32; 3];
/// Row-vector 4×4 (translation in row 3) — the same layout as the game's `MATRIX` at `obj+0x90` [L].
pub type Mat4 = [[f32; 4]; 4];

/// `FRAME_RATE` (0x30d0d0) while the level is being built: the doors are created at load and the fallback speed
/// 1/30 measures 0.03333 in the snapshot [L], although the global reads 60.0 during play (the video rate is set
/// after loading; the auto-close below uses the play value).
const CREATE_FRAME_RATE: f32 = 30.0;
/// `FRAME_RATE_INT` (60 during play) × 6: ticks an open door waits before it closes on its own.
const AUTO_CLOSE: u32 = 360;
/// `Door_Activate`'s wrong-side margin (0x3e99999a).
const MARGIN: f32 = 0.3;
/// `Door_ReactToHit`'s reversal threshold (ABS compares against 0.0002).
const HIT_EPS: f32 = 0.0002;

/// States (`obj+0xF4`): 2 closed, 4 open, 8 opening (pos moves away from closed), 0x10 closing.
pub const CLOSED: u16 = 2;
pub const OPEN: u16 = 4;
pub const OPENING: u16 = 8;
pub const CLOSING: u16 = 0x10;

/// One key of a door anim (a 0x19 block payload): 28 bytes — `f32[3]` position at +0, quaternion (x, y, z, w)
/// at +0xC [M: `Spline_Interp` 0x1e1858 addresses keys at `list + 0x1c·k`; L: `Door_UNLocked`'s 3 keys].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key { pub pos: V3, pub quat: [f32; 4] }

pub fn parse_keys(payload: &[u8]) -> Vec<Key> {
    (0..payload.len() / 28).map(|k| {
        let p = &payload[k * 28..];
        let w = |i: usize| f32::from_le_bytes(p[i..i + 4].try_into().unwrap());
        Key { pos: [w(0), w(4), w(8)], quat: [w(0xc), w(0x10), w(0x14), w(0x18)] }
    }).collect()
}

fn dot3(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }

/// `Mat_GetDir` (0x1e3398): row 2. `Mat_GetNorm` (0x1e33d8): row 0.
fn row(m: &Mat4, r: usize) -> V3 { [m[r][0], m[r][1], m[r][2]] }

/// Row-vector `v·M` (`RotPreTransVec`'s product with an inverse pose).
fn transform(m: &Mat4, v: V3) -> V3 {
    let mut o = [m[3][0], m[3][1], m[3][2]];
    for k in 0..3 { for c in 0..3 { o[c] += v[k] * m[k][c]; } }
    o
}

/// Inverse of a rigid matrix (transposed rotation, `−t·Rᵀ`) — what `Mat_Inverse` computes for the position +
/// quaternion poses. Anim-less doors never use it (the game inverts whatever `Spline_Interp` left, which is
/// nothing: it returns on a NULL framelist).
fn rigid_inverse(m: &Mat4) -> Mat4 {
    let mut o = [[0.0f32; 4]; 4];
    for i in 0..3 { for j in 0..3 { o[i][j] = m[j][i]; } }
    for c in 0..3 { o[3][c] = -(m[3][0] * o[0][c] + m[3][1] * o[1][c] + m[3][2] * o[2][c]); }
    o[3][3] = 1.0;
    o
}

/// Quaternion → matrix (`Spline_Interp`'s tail): `m00 = 1−(2y·y+2z·z)` and so on, translation in row 3 [L].
fn quat_matrix(q: [f32; 4], t: V3) -> Mat4 {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let (yy, zz, xx) = (y + y, z + z, x + x);
    [
        [1.0 - (yy * y + zz * z), yy * x + zz * w, zz * x - yy * w, 0.0],
        [yy * x - zz * w, 1.0 - (xx * x + zz * z), zz * y + xx * w, 0.0],
        [zz * x + yy * w, zz * y - xx * w, 1.0 - (xx * x + yy * y), 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

/// `Spline_Interp`'s slerp: dot, shortest path, `PS2Sinf` weights while `1−|dot| > 0.01`, else the unnormalised
/// linear blend.
fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let neg = dot < 0.0;
    if neg { dot = -dot; }
    let (w0, w1) = if 0.01 < 1.0 - dot {
        let angle = dot.acos();
        let s = sinf(angle);
        (sinf(angle - t * angle) / s, sinf(t * angle) / s)
    } else { (1.0 - t, t) };
    let w0 = if neg { -w0 } else { w0 };
    [w0 * a[0] + w1 * b[0], w0 * a[1] + w1 * b[1], w0 * a[2] + w1 * b[2], w0 * a[3] + w1 * b[3]]
}

/// Catmull-Rom at `t` (`Spline_Interp`'s position formula, in the game's operation order).
fn catmull(p0: V3, p1: V3, p2: V3, p3: V3, t: f32) -> V3 {
    let mut o = [0.0f32; 3];
    for k in 0..3 {
        let (a, b, c, d) = (p0[k], p1[k], p2[k], p3[k]);
        o[k] = (b + b + t * ((c - (t - 1.0) * (t - 1.0) * a)
            + t * (((b * -5.0 + t * b * 3.0 + c * 4.0) - t * c * 3.0) + (t - 1.0) * d))) * 0.5;
    }
    o
}

/// One interpolation of a door anim [M: `KeyFrame_Interp` 0x1e1140, `Spline_Interp` 0x1e1858]. `time` in key
/// units (the caller passes `pos01 · frames`), clamped to `[first, last]`; `k = floor(time)` clamped, `t` the
/// fraction; neighbour keys clamp into range. Keyframe mode lerps the position (clamped to the segment's far
/// end); spline mode runs Catmull-Rom (all four positions equal takes `k`'s directly). Both slerp the rotation
/// between keys `k` and `k+1` (equal quaternions pass through). `None` = a NULL framelist: nothing is written.
pub fn interp(keys: &[Key], first: u16, last: u16, time: f32, spline: bool) -> Option<Mat4> {
    if keys.is_empty() || first >= last { return None; }
    let (lo, hi) = (first as i32, last as i32);
    let time = time.max(lo as f32).min(hi as f32);
    let k = (time as i32).clamp(lo, hi);
    let t = time - k as f32;
    let key = |i: i32| &keys[(i.clamp(lo, hi) - lo) as usize];
    let (pk, pk1) = (key(k), key(k + 1));
    let pos = if spline {
        let (p0, p3) = (key(k - 1).pos, key(k + 2).pos);
        if p0 == pk.pos && pk.pos == pk1.pos && pk1.pos == p3 { pk.pos } else { catmull(p0, pk.pos, pk1.pos, p3, t) }
    } else {
        let mut o = [pk.pos[0] + (pk1.pos[0] - pk.pos[0]) * t, pk.pos[1] + (pk1.pos[1] - pk.pos[1]) * t, pk.pos[2] + (pk1.pos[2] - pk.pos[2]) * t];
        for c in 0..3 {
            let over = if pk1.pos[c] >= pk.pos[c] { pk1.pos[c] < o[c] } else { o[c] < pk1.pos[c] };
            if over { o[c] = pk1.pos[c]; }
        }
        o
    };
    let quat = if pk.quat == pk1.quat { pk.quat } else { slerp(pk.quat, pk1.quat, t) };
    Some(quat_matrix(quat, pos))
}

/// One door: the `obj_tag` fields the port keeps plus the 0x80-byte `DOORINFO` instance at `obj+0xE0`.
#[derive(Clone, Debug)]
pub struct Door {
    /// Index into the main map's placements.
    pub placement: usize,
    /// `obj+0xF6` (parameter 0): 0 swing, 1–2 slide, other = activation marker only.
    pub kind: i16,
    /// `+0x6A` (p1): door group — `Doors_Calc` slaves the group to its first non-busy member.
    pub group: u16,
    /// `+0x66` (p2): the open switch channel — on: open; off and not closed: close at once (no timer).
    pub ch_open: u16,
    /// `+0x68` (p3): the lock channel — off and not closed, activation only plays the hit sound; its rising
    /// edge unlocks the door for good (the door clears the field).
    pub ch_lock: u16,
    /// `obj+0xFD bit 3` (p4): link to a collision portal (block 0x21 — not ported).
    pub straddle: bool,
    /// `+0x60` (p5): entering state 8. `+0x62` (p6): entering 0x10. `+0x64` (p7): the locked/hit feedback.
    pub snd_open: u16, pub snd_close: u16, pub snd_hit: u16,
    /// `+0x70` (p8|p9): bit 0 spline (else keyframe) interpolation, bit 2 "starts open" (+ open channel forced on
    /// at create, `Door_Create` :96-99), bit 4 the busy marker, bit 5 keep open.
    pub flags: u16,
    /// `+0x76` (p10): 1 = activate from the front only, 2 = from behind only (one-shot; 0 = either side).
    pub approach: u8,
    /// `+0x77` (p11): never player-activated.
    pub locked_flag: u8,
    /// `+0xC`: the anim (a 0x19 key list). `None` = no anim: the door never visually moves.
    pub keys: Option<Vec<Key>>,
    /// `+0x6E`: the last key index (the record's count − 1; 0xFFFF without an anim).
    pub frames: u16,
    /// `obj+0xF4`.
    pub state: u16,
    /// `+0x50`: 0..1 along the anim; a swing door's closed position is 0.5.
    pub pos01: f32,
    /// `+0x54`: 1/rate (rate 0 → 1/30 at create [L]); `Door_SetupSwing` re-quantises a swing door to whole ticks.
    pub speed: f32,
    /// `+0x58` / `+0x5C`: the travel direction and the swing sign (`Door_SetupSwing`'s dot product).
    pub dir: f32, pub swing: f32,
    /// `+0x6C`: ticks since the last activation (`Door_ReactToHit` waits it out). `+0x72`: the auto-close timer.
    pub timer: i16, pub open_timer: u16,
    /// `+0x74`: `Door_DoMsg` shows its label once (no text system; the flag still latches).
    pub msg_shown: bool,
    /// `obj+0x90`: the drawn and collided pose. `+0x10` (swing): the inverse closed pose for the side tests.
    pub matrix: Mat4, pub inv_closed: Mat4,
    /// `obj+0x30` (the sound position `obj+0xC0` is the same point) and the sphere `obj+0x80/+0x8C`.
    pub pos: V3, pub sph: (V3, f32),
    /// The gfx sphere centre in the placement's local frame — transformed by the LIVE `matrix` each tick it gives
    /// `obj+0x80` (the moved pass rebuilds the world sphere after `Door_Interp` sets `obj+0xFA & 0x20` [M
    /// 0x135738:28, 0x1336f8:209]), so the auto-close blocking zone follows the open door leaf.
    pub sph_centre: V3,
    /// The object's hashcode (the drawn mesh and the collision shape).
    pub glist: u32,
}

impl Door {
    /// The pose at `pos01` (`Door_Interp`'s write; the create path always takes the spline mode).
    fn pose_at(&self, pos01: f32, spline: bool) -> Option<Mat4> {
        self.keys.as_ref().and_then(|k| interp(k, 0, self.frames, pos01 * self.frames as f32, spline))
    }
    /// `Door_SetupSwing`'s closed pose: `Spline_Interp(0.5 · frames)`.
    fn closed_pose(&self) -> Option<Mat4> { self.pose_at(0.5, true) }
}

/// All doors of a level, in placement (creation) order.
#[derive(Clone, Debug)]
pub struct Doors {
    pub list: Vec<Door>,
    /// `DoorGroupsStates` (0xca bytes, cleared by `Doors_Calc` each frame): per group, the OR of the states.
    pub group_states: [u16; 101],
    /// The previous tick's `switch_channels` (`switch_channels_prev`) for the lock channel's rising edge.
    prev_channels: [u8; 256],
    hold_channels: [u8; 256],
}

impl Default for Doors {
    fn default() -> Doors { Doors { list: Vec::new(), group_states: [0; 101], prev_channels: [0; 256], hold_channels: [0; 256] } }
}

/// The main map's door placements in creation order (create and nf-view).
pub fn door_placements(placements: &[Placement]) -> Vec<usize> {
    placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xdb && p.flags & 0xa000 == 0).map(|(i, _)| i).collect()
}

/// `Door_Update`'s proximity test [M 0x1363c8:52-68, 121-137]: `Vec_SqDist3D(door obj+0x80, actor obj+0x80) <
/// 2·(r_door + r_actor)²` with the `obj+0x8C` radii. `obj+0x80` is the door's LIVE world sphere (rebuilt by the
/// moved pass after every `Door_Interp`), so a slide door's blocking zone follows the open leaf — the port used to
/// test the fixed placement position with a 1.0 stand-in radius, which closed doors the game leaves open
/// (core-loop §7 item 4 / door-port-check #1).
fn blocked(door: (V3, f32), a: (V3, f32)) -> bool {
    let d = [door.0[0] - a.0[0], door.0[1] - a.0[1], door.0[2] - a.0[2]];
    let r = door.1 + a.1;
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < r * r + r * r
}

/// A body the door's proximity scans see: position, bounding radius, and whether its `obj+0xF0` flags carry bit
/// 0x10 — the scan skips such objects entirely [M 0x136678-0x136684; the 07000001 player (0x890) and drones
/// (0x8B0) have it set [L], the 07000005 player (0x880) does not [L]. The writer of that bit is not ported, so
/// the port models it as clear (the 07000005 evidence; door-port-check #2).
pub type DoorActor = (V3, f32, bool);

/// The scan's candidates: every actor the `obj+0xF0 & 0x10` filter lets through [M 0x136678-0x136684].
fn visible_actors(actors: &[DoorActor]) -> impl Iterator<Item = (V3, f32)> + '_ {
    actors.iter().filter(|a| !a.2).map(|a| (a.0, a.1))
}

/// A sound `Door_SetState` / `Door_Activate` asks for: id and position.
pub type SoundEvent = (u16, V3);

impl Doors {
    /// `Door_Create` over the main map's placements. `path_blocks` is the level's 0x19 payloads (its
    /// `path_blocks()`); `object` resolves a placement to `(glist hashcode, gfx sphere centre, radius)`.
    pub fn create(
        placements: &[Placement],
        path_blocks: &[&[u8]],
        mut object: impl FnMut(&Placement) -> Option<(u32, V3, f32)>,
    ) -> Doors {
        let mut list = Vec::new();
        for &pi in door_placements(placements).iter() {
            let p = &placements[pi];
            let par = |k: u32| p.params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v);
            // The runtime record `parsemap_block_map_data_dynamic` (0x1d1b68) builds on its stack: the record's
            // +0x40 (count) → +0x02, +0x42 (rate) → +0x04, and `m_paths[record+0x44]` → +0x08 — that is,
            // extra[7]'s halves and extra[8]'s low half.
            let count = (p.extra[7] & 0xffff) as u16;
            let rate = (p.extra[7] >> 16) as u16;
            let index = (p.extra[8] & 0xffff) as usize;
            let keys = if count != 0 { path_blocks.get(index).map(|b| parse_keys(b)) } else { None };
            let (glist, centre, radius) = object(p).unwrap_or((0, p.translation, 0.0));
            let m = object_matrix(p.rotation, p.translation);
            let flags = (par(8) | par(9)) as u16;
            let mut d = Door {
                placement: pi,
                kind: par(0) as i16,
                group: par(1) as u16,
                ch_open: par(2) as u16,
                ch_lock: par(3) as u16,
                straddle: par(4) != 0,
                snd_open: par(5) as u16, snd_close: par(6) as u16, snd_hit: par(7) as u16,
                flags,
                approach: par(10) as u8,
                locked_flag: par(11) as u8,
                keys, frames: count.wrapping_sub(1),
                state: if flags & 4 != 0 { OPEN } else { CLOSED },
                pos01: 0.0, speed: if rate != 0 { 1.0 / rate as f32 } else { 1.0 / CREATE_FRAME_RATE },
                dir: 0.0, swing: 0.0, timer: 0, open_timer: 0, msg_shown: false,
                matrix: m, inv_closed: [[0.0; 4]; 4],
                pos: p.translation, sph: (transform(&m, centre), radius), sph_centre: centre, glist,
            };
            if d.kind == 0 {
                // `Door_SetupSwing`: the swing covers half the range, so the speed re-quantises to whole ticks
                // (`0.5 / (int)(0.5 / 2s)`; 1/30 → 1/14 [L]); closed sits at 0.5 (1.0 for a starts-open door);
                // the inverse closed pose and the swing sign come from the anim's ends.
                d.speed = 0.5 / ((0.5 / (d.speed + d.speed)) as i32 as f32);
                d.pos01 = if d.state & 4 != 0 { 1.0 } else { 0.5 };
                if let Some(closed) = d.closed_pose() {
                    d.inv_closed = rigid_inverse(&closed);
                    if let Some(open_a) = d.pose_at(0.0, true) {
                        d.swing = if dot3(row(&open_a, 2), row(&closed, 0)) < 0.0 { 1.0 } else { -1.0 };
                    }
                }
                if let Some(m) = d.pose_at(d.pos01, true) { d.matrix = m; }
            } else {
                d.pos01 = if d.state & 4 != 0 { 1.0 } else { 0.0 };
                if let Some(m) = d.pose_at(d.pos01, true) { d.matrix = m; }
                if let Some(c) = d.pose_at(0.0, true) { d.inv_closed = rigid_inverse(&c); }
            }
            list.push(d);
        }
        Doors { list, group_states: [0; 101], prev_channels: [0; 256], hold_channels: [0; 256] }
    }

    /// A door that starts open sets its open channel at create (`Door_Create`'s tail).
    pub fn create_channels(&self, channels: &mut Channels, frame: u32) {
        for d in &self.list {
            if d.state & 4 != 0 && d.ch_open != 0 { channels.set(d.ch_open as u32, 1, frame); }
        }
    }

    /// `Door_SetState`: the entering sounds, the open channel mirrors "in motion" (timestamped every motion
    /// transition, not only its rising edge), the auto-close timer resets.
    fn set_state(&mut self, i: usize, new: u16, channels: &mut Channels, frame: u32, sounds: &mut Vec<SoundEvent>) {
        let (old, pos, ch, so, sc) = {
            let d = &self.list[i];
            (d.state, d.pos, d.ch_open, d.snd_open, d.snd_close)
        };
        if new & 8 != 0 && old & 8 == 0 && so != 0xffff { sounds.push((so, pos)); }
        if new & 0x10 != 0 && old & 0x10 == 0 && sc != 0xffff { sounds.push((sc, pos)); }
        if ch != 0 {
            // `Door_SetState` (0x135618:26): the channel mirrors the OLD state's "in motion" bits (0xc), timestamped
            // every motion transition. The game reads `+0xf4 & 0xc` BEFORE writing the new state [M].
            let moving = (old & 0xc) != 0;
            channels.v[ch as usize] = moving as u8;
            if moving { channels.time[ch as usize] = frame; }
        }
        let d = &mut self.list[i];
        d.state = new;
        d.open_timer = 0;
    }

    /// `Door_Interp`: pose at the current `pos01`, advance, snap at the ends, re-pose. Without an anim the
    /// state machine still runs but nothing is posed (`Spline_Interp` returns on a NULL framelist).
    fn interp(&mut self, i: usize, advance: bool, channels: &mut Channels, frame: u32, sounds: &mut Vec<SoundEvent>) {
        let (spline, frames) = (self.list[i].flags & 1 != 0, self.list[i].frames);
        let _ = frames;
        if let Some(m) = self.list[i].pose_at(self.list[i].pos01, spline) { self.list[i].matrix = m; }
        if !advance { return; }
        let (speed, dir) = (self.list[i].speed, self.list[i].dir);
        let f = self.list[i].pos01 + speed * dir;     // FRAME_RATE_MUL is 1.0 during play [L]
        self.list[i].pos01 = f;
        let mut posed = false;
        let (kind, state) = (self.list[i].kind, self.list[i].state);
        if kind == 0 {
            if state & 8 == 0 || (0.0 <= f && f <= 1.0) {
                if state & 0x10 != 0 && (0.5 - self.list[i].pos01).abs() <= speed {
                    self.list[i].pos01 = 0.5;
                    self.set_state(i, CLOSED, channels, frame, sounds);
                    posed = true;
                }
            } else {
                self.list[i].pos01 = f.max(0.0).min(1.0);
                self.set_state(i, OPEN, channels, frame, sounds);   // fully swung to an end
                posed = true;
            }
        } else if (0..=2).contains(&kind) {
            if f < 0.0 {
                self.list[i].pos01 = 0.0;
                self.set_state(i, CLOSED, channels, frame, sounds);
                posed = true;
            }
            if self.list[i].pos01 > 1.0 {
                self.list[i].pos01 = 1.0;
                self.set_state(i, OPEN, channels, frame, sounds);
                posed = true;
            }
        }
        if posed {
            // The re-pose at the snapped position is always the spline mode (`Door_Interp`'s tail).
            if let Some(m) = self.list[i].pose_at(self.list[i].pos01, true) { self.list[i].matrix = m; }
        }
    }

    /// `Door_Update` for one door, in the control list's order. `actors`: the objects the proximity tests see —
    /// the game scans classes 2 (the player), 3 and 0x12 (drones) by bounding sphere, skipping every candidate
    /// whose `obj+0xF0` carries 0x10 [M 0x136678-0x136684]; **`actors[0]` is the activator** (`glb_players` in
    /// every call the port makes). Returns the sounds to play this tick.
    pub fn update(
        &mut self, i: usize, channels: &mut Channels, actors: &[DoorActor], frame: u32,
    ) -> Vec<SoundEvent> {
        let mut sounds = Vec::new();
        if self.list[i].timer != 0 { self.list[i].timer -= 1; }
        if self.list[i].flags & 0x10 != 0 { return sounds; }   // busy: `Door_Activate`'s marker, cleared by `calc`
        // The live `obj+0x80` sphere: the pose matrix from the LAST interp (the game's moved pass rebuilds the
        // sphere after the walk — one tick behind the visual, matching this read order) [M 0x1336f8:209].
        let door_sph = (transform(&self.list[i].matrix, self.list[i].sph_centre), self.list[i].sph.1);
        // The open channel: on opens (unless already open); off and not closed: close at once, timer reset.
        let (state, ch_open, ch_lock, keep_open) = {
            let d = &self.list[i];
            (d.state, d.ch_open, d.ch_lock, d.flags & 0x20 != 0)
        };
        if state & 6 != 0 && ch_open != 0 {
            if channels.on(ch_open as u32) {
                if state & 4 == 0 { self.activate(i, actors, false, channels, frame, &mut sounds); }
            } else if state & 2 == 0 {
                if !visible_actors(actors).any(|a| blocked(door_sph, a)) {
                    self.activate(i, actors, false, channels, frame, &mut sounds);
                }
                self.list[i].open_timer = 0;
            }
        }
        // The lock channel's rising edge: unlock for good (the game also shows text label 0x2000003).
        if ch_lock != 0 {
            let was = self.prev_channels[ch_lock as usize] != 0;
            let now = channels.on(ch_lock as u32);
            if !was && now {
                self.activate(i, actors, false, channels, frame, &mut sounds);
                let d = &mut self.list[i];
                d.ch_lock = 0;
                d.flags |= 0x10;
            }
        }
        // The auto-close: an open door without the keep-open flag, after `FRAME_RATE_INT·6` quiet ticks
        // [M 0x13660c-0x136700]. The timer ALWAYS increments while open; at 360 the proximity scan runs (skipping
        // `obj+0xF0 & 0x10` candidates), and if nobody is blocking, `Door_Activate` closes it. The timer ALWAYS
        // resets to 0 at 360 (whether or not the close fired). The scan centres on the LIVE `obj+0x80` sphere with
        // the real `obj+0x8C` radius — the open leaf, not the frame (core-loop §7 item 4).
        if self.list[i].state & 4 != 0 && !keep_open {
            self.list[i].open_timer = self.list[i].open_timer.wrapping_add(1);
            if self.list[i].open_timer as u32 >= AUTO_CLOSE {
                if !visible_actors(actors).any(|a| blocked(door_sph, a)) {
                    self.activate(i, actors, false, channels, frame, &mut sounds);
                }
                self.list[i].open_timer = 0;
            }
        }
        // Motion. `Door_ReactToHit` (0x135980) is the COLLIDE function (control_funcs[0x1C*0xC+4]), called
        // from `Collide_Update` (0x1ea398:59-64) via the proxy list (obj+0xD0) — NOT from `Door_Update`.
        // The port's proxy-list path (U-041) is not ported; until then, the anti-crush does not fire from
        // `update` — matching the disassembly's call structure. (A-11b: removed the `react_to_hit` call that
        // was reversing CLOSING→OPENING when Bond stood near the hinge, preventing the close.)
        if self.list[i].state & 0x18 != 0 {
            self.interp(i, true, channels, frame, &mut sounds);
        }
        let d = &self.list[i];
        if d.group != 0 && (d.group as usize) < self.group_states.len() {
            self.group_states[d.group as usize] |= d.state;
        }
        sounds
    }

    /// `Door_Activate` [M: disassembly 0x135be0–0x135f14]. `from_player` honours the locked flag.
    pub fn activate(
        &mut self, i: usize, actors: &[DoorActor], from_player: bool, channels: &mut Channels, frame: u32,
        sounds: &mut Vec<SoundEvent>,
    ) -> bool {
        let _ = frame;
        let (hit_snd, pos) = (self.list[i].snd_hit, self.list[i].pos);
        let mut hit = || { if hit_snd != 0xffff { sounds.push((hit_snd, pos)); } };
        if from_player && self.list[i].locked_flag != 0 { return false; }
        {
            let d = &self.list[i];
            if d.ch_lock != 0 && !channels.on(d.ch_lock as u32) && d.state & 0x12 != 0 { hit(); return false; }
            if d.flags & 0x10 != 0 { return false; }
        }
        // The approach side (one-shot): the activator's offset along the door's facing (`Mat_GetDir`).
        if self.list[i].approach != 0 {
            let dot = actors.first().map_or(0.0, |a| dot3(sub(a.0, self.list[i].pos), row(&self.list[i].matrix, 2)));
            if self.list[i].approach == 1 && dot < 0.0 { hit(); return false; }
            if self.list[i].approach == 2 {
                if dot > 0.0 { hit(); return false; }
                self.list[i].approach = 0;
            } else {
                self.list[i].approach = 0;
            }
        }
        let mut acted = false;
        let (kind, state) = (self.list[i].kind, self.list[i].state);
        match kind {
            0 => {
                // Swing: closed or closing swings away from the activator's side; open or opening closes toward
                // 0.5 — unless the activator stands in the swing path (the ±0.3 margin against the radius).
                if state & 0xc == 0 {
                    self.swing_from_side(i, actors, channels, frame, sounds);
                    acted = true;
                } else {
                    let act = actors.first().map_or(pos, |a| a.0);
                    let (dir, swing, radius) = (self.list[i].dir, self.list[i].swing, self.list[i].sph.1);
                    let local = transform(&self.list[i].inv_closed, act);
                    let m = if dir * swing < 0.0 { local[2] + MARGIN } else { local[2] - MARGIN };
                    let f = m * dir * swing;
                    if f < 0.0 && f.abs() < radius {
                        // In the doorway: the busy marker only (the door waits the two ticks out).
                    } else if state & 0x12 != 0 {
                        self.swing_from_side(i, actors, channels, frame, sounds);
                        acted = true;
                    } else if state & 0xc != 0 {
                        self.list[i].dir = if self.list[i].pos01 > 0.5 { -1.0 } else { 1.0 };
                        self.set_state(i, CLOSING, channels, frame, sounds);
                        acted = true;
                    }
                }
            }
            1..=2 => {
                // Slide: closed or closing opens; open or opening closes (a plain reverse).
                if state & 0x12 != 0 {
                    self.list[i].dir = 1.0;
                    self.set_state(i, OPENING, channels, frame, sounds);
                    acted = true;
                } else if state & 0xc != 0 {
                    self.list[i].dir = -1.0;
                    self.set_state(i, CLOSING, channels, frame, sounds);
                    acted = true;
                }
            }
            _ => {}
        }
        let d = &mut self.list[i];
        d.timer = 2;
        d.flags |= 0x10;
        acted
    }

    /// The swing door's `side·swing` opening: `dir = sign((inv_closed · activator).z) · swing`, state 8.
    fn swing_from_side(&mut self, i: usize, actors: &[DoorActor], channels: &mut Channels, frame: u32, sounds: &mut Vec<SoundEvent>) {
        let act = actors.first().map_or(self.list[i].pos, |a| a.0);
        let local = transform(&self.list[i].inv_closed, act);
        let side = if local[2] < 0.0 { -1.0 } else { 1.0 };
        self.list[i].dir = side * self.list[i].swing;
        self.set_state(i, OPENING, channels, frame, sounds);
    }

    /// `Door_ReactToHit` [M 0x135980], the proxy scan stood in for by the sphere test: a moving door that meets
    /// an actor reverses (the anti-crush). Waits the 2-tick activation timer out; kind 1 doors ignore it.
    /// NOT called from `Door_Update` (A-11b) — the game calls this from `Collide_Update` via the proxy list
    /// (obj+0xD0). Kept for the future proxy-list port (U-041); currently unused.
    #[allow(dead_code)]
    fn react_to_hit(&mut self, i: usize, actors: &[DoorActor], channels: &mut Channels, frame: u32, sounds: &mut Vec<SoundEvent>) {
        if self.list[i].timer != 0 || self.list[i].state & 0x18 == 0 { return; }
        let Some(act) = visible_actors(actors).find(|a| blocked((self.list[i].pos, self.list[i].sph.1), *a)).map(|a| a.0) else { return };
        let (kind, state, swing, pos01, dir) = {
            let d = &self.list[i];
            (d.kind, d.state, d.swing, d.pos01, d.dir)
        };
        match kind {
            0 => {
                // The collider in the door's current space: `f = local.z · swing`, negated past half travel.
                let local = transform(&rigid_inverse(&self.list[i].matrix), act);
                let mut f = local[2] * swing;
                if pos01 > 0.5 { f = -f; }
                if state & 0x10 == 0 {
                    if state & 8 == 0 || f <= HIT_EPS { return; }
                    self.list[i].dir = -dir;
                    self.set_state(i, CLOSING, channels, frame, sounds);
                } else {
                    if f >= -HIT_EPS { return; }
                    self.list[i].dir = -dir;
                    self.set_state(i, OPENING, channels, frame, sounds);
                }
            }
            2 => {
                if state & 0x10 == 0 { return; }
                self.list[i].dir = -dir;
                self.set_state(i, OPENING, channels, frame, sounds);
            }
            _ => return,
        }
        let d = &mut self.list[i];
        d.timer = 2;
        d.flags |= 0x10;
    }

    /// `Doors_Calc` (after every door's update): a group's members copy the first non-busy door's direction,
    /// position, state and timer, re-pose, and every busy marker clears.
    pub fn calc(&mut self) {
        for master in 0..self.list.len() {
            let group = self.list[master].group;
            if group == 0 { continue; }
            let (dir, pos01, state, open_timer) = {
                let m = &self.list[master];
                (m.dir, m.pos01, m.state, m.open_timer)
            };
            for other in 0..self.list.len() {
                if other == master { continue; }
                let spline;
                {
                    let d = &mut self.list[other];
                    if d.group != group || d.flags & 0x10 != 0 { continue; }
                    d.dir = dir;
                    d.pos01 = pos01;
                    d.state = state;
                    d.open_timer = open_timer;
                    d.flags |= 0x10;
                    spline = d.flags & 1 != 0;
                }
                if let Some(m) = self.list[other].pose_at(pos01, spline) { self.list[other].matrix = m; }
            }
        }
        for d in self.list.iter_mut() { d.flags &= !0x10; }
    }

    /// `Player_Activate`'s door part (0x1a5310): the use sphere — radius 1 at one unit ahead of the head —
    /// activates every door it meets (the game walks the whole hit list; walls do not stop it). `head` is also
    /// the activator's position for the side tests.
    pub fn player_use(&mut self, head: V3, look: V3, channels: &mut Channels, frame: u32) -> (Vec<SoundEvent>, usize) {
        // `Player_Activate` (0x1a5170): Bond's head + forward × 1.0, tested as a `Collide_SphereIntersect(1.0, ...)`
        // against every cel-linked object. Doors (type 0x1C) call `Door_Activate(obj, player, 1)`. The port uses the
        // door's placement position + a generous distance check (the door's bounding sphere may be 0 if the glist
        // sphere wasn't read) [M 0x1a5170:100-108,128].
        let mut sounds = Vec::new();
        let use_point = [head[0] + look[0], head[1] + look[1], head[2] + look[2]];
        let mut hits = 0;
        for i in 0..self.list.len() {
            let pos = self.list[i].pos;
            let d = [use_point[0] - pos[0], use_point[1] - pos[1], use_point[2] - pos[2]];
            // The game uses a 1.0-radius sphere at the use point against the door's collision mesh.
            // Stand-in: distance to the door's placement position < 2.5 (covers the door's extent + use radius).
            let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            if d2 < 6.25 { // 2.5² = generous reach
                self.activate(i, &[(head, 0.0, false)], true, channels, frame, &mut sounds);
                hits += 1;
            }
        }
        // The straddle walk-in (walkreal-c): the kind-2 slide doors carry the placement's straddle
        // param (par 4 [D] — every 07000015/16 kind-2 door has it) and the game arms a per-door
        // flag from it at create (`Door_Create` 0x136100 reads the +0x3c param into obj+0xfd bit 8
        // [M]). The player opens them by WALKING INTO the threshold — the 07000015 top-band door
        // [32] (25.1, 41.6, 70.1, unlocked) is E-dead in the port yet must open for the level's
        // only free west crossing [D flow]. The port had the flag parsed but never consumed.
        // Stand-in shape [S — the game's straddle test function is not pinned]: a closed straddle
        // door activates (from_player — the lock guards hold) when the player's feet stand inside
        // its threshold (xz 1.4, the leaf's half-width plus the capsule; y within the door band).
        for i in 0..self.list.len() {
            let d = &self.list[i];
            if !d.straddle || d.kind == 0 || d.state & 0xc != 0 { continue; }
            let dx = head[0] - d.pos[0];
            let dz = head[2] - d.pos[2];
            if dx * dx + dz * dz > 1.96 { continue; }
            if (head[1] - d.pos[1]).abs() > 2.2 { continue; }
            self.activate(i, &[(head, 0.0, false)], true, channels, frame, &mut sounds);
            hits += 1;
        }
        (sounds, hits)
    }

    /// The straddle walk-in, EVERY tick (fix-casings-16). The E-path arm above only runs while the
    /// use key is held, but the game's straddle doors open with no E at all — on 07000016 the
    /// arrival walkway's first door (glist 0x803f00a0, kind 2, straddle, ch_open 0,
    /// ch_lock 0, flags 0x3 → closed at create [M Door_Create 0x136100: state 4 only when
    /// doorinfo+0x4c|+0x50 bit 2]) blocks the only path, and neither `Door_Update` (a ch_open==0
    /// door skips every channel arm [M 0x1363c8: `(state & 6) == 0` / `uVar2 == 0 → LAB_00136550`])
    /// nor `Door_ReactToHit` (0x135980 reverses only MOVING leaves: kind 0 needs `state & 0x18`,
    /// kind 2 needs `state & 0x10`) ever opens a closed idle one. The game opens it through the
    /// collide-contact path the port has not rebuilt (U-041); until then the stand-in drives the
    /// SAME activation from `Door_Update`'s per-tick actor list, with the game's own proximity
    /// metric — `Vec_SqDist3D(door+0x80, actor+0x80) < (door+0x8C + actor+0x8C)² · 2`
    /// [M 0x1363c8 @0x136678-0x136684, the scan `Door_Update` itself runs for the auto-close] —
    /// so the open lands where the game's contact would. The lock guards hold: `activate`
    /// (from_player) rejects a locked door (the 320 thud) and the `state & 0xc` skip keeps a
    /// moving leaf alone. Swing doors (kind 0) stay E/ReactToHit-only, as in the E arm.
    pub fn straddle_walkin(&mut self, actors: &[DoorActor], channels: &mut Channels, frame: u32) -> Vec<SoundEvent> {
        let mut sounds = Vec::new();
        for i in 0..self.list.len() {
            let (straddle, kind, state) = { let d = &self.list[i]; (d.straddle, d.kind, d.state) };
            if !straddle || kind == 0 || state & 0xc != 0 { continue; }
            let (sc, sr) = self.list[i].sph;
            let near = actors.iter().filter(|a| !a.2).any(|a| {
                let dx = sc[0] - a.0[0]; let dy = sc[1] - a.0[1]; let dz = sc[2] - a.0[2];
                let th = (sr + a.1) * (sr + a.1) * 2.0;
                dx * dx + dy * dy + dz * dz < th
            });
            if !near { continue; }
            self.activate(i, actors, true, channels, frame, &mut sounds);
        }
        sounds
    }

    /// Snapshot the channels for this tick's lock-edge test (`switch_channels_prev`). The game
    /// copies BOTH latches right after `Trigger_Calc`, BEFORE the object walk — two amemcpys at
    /// the head of `control_movement_object_handler` [M 0x1336f8: `amemcpy(0x26fd90, 0x26fe90,
    /// 0x100); amemcpy(0x26fe90, 0x26fc90, 0x100)` — `prev ← hold; hold ← channels`]: `prev`
    /// lags `channels` by TWO snapshots, so a channel pulsed anywhere since the last snapshot
    /// (a trigger or script mid-walk, or the player's own laser burn in `Player_Update`'s
    /// class-4 slot [M Player_LaserPointer 0x1a65d8 → `Lock_Update` 0x1ae3d8:194]) still shows a
    /// rising edge in `Door_Update`'s `prev == 0 && now != 0` test (0x136550-0x136608). The old
    /// single-copy read made any pre-snapshot write (the Q-Decryptor keypad burn on 07000012)
    /// invisible to the edge — the door stayed locked forever.
    pub fn end_tick(&mut self, channels: &Channels) {
        self.prev_channels = self.hold_channels;
        for (k, c) in self.hold_channels.iter_mut().enumerate() { *c = channels.v.get(k).copied().unwrap_or(0); }
    }

    /// `NF_DOORLOG=1` per-tick diagnostic (Task A-8-2): for every door, prints placement, kind,
    /// ch_open, ch_lock, flags (with the keep-open and spline bits decoded), the glist hash, whether
    /// that hash is registered in the collision world's `shapes` table (the bullet-collision check —
    /// `0xffffffff` means the placement's object hashcode is `u32::MAX` and the door has no collision),
    /// the current state, `pos01`, the auto-close `open_timer`, and the bounding sphere.
    ///
    /// `world` is the collision world built by `CollisionWorld::build` (so `has_shape` can be tested);
    /// pass `None` to skip the `in_shapes` column. Called from `Drones::tick` after `end_tick`.
    pub fn log_doors(&self, world: Option<&crate::world::CollisionWorld>) {
        if std::env::var_os("NF_DOORLOG").is_none() { return; }
        eprintln!("=== NF_DOORLOG frame: {} doors ===", self.list.len());
        eprintln!("idx placement kind grp ch_open ch_lock flags     appr locked glist        in_shapes state  pos01  open_timer sph.pos                      sph.r");
        for (i, d) in self.list.iter().enumerate() {
            let in_shapes = world.map_or(false, |w| w.has_shape(d.glist));
            eprintln!(
                "{i:<3} {:<9} {:<4} {:<3} {:<7} {:<7} {:#06x} {:<4} {:<6} {:#010x} {:<9} {:#04x}  {:5.3} {:<10} ({:7.2},{:7.2},{:7.2}) {:5.3}",
                d.placement, d.kind, d.group, d.ch_open, d.ch_lock, d.flags,
                d.approach, d.locked_flag, d.glist, in_shapes,
                d.state, d.pos01, d.open_timer,
                d.sph.0[0], d.sph.0[1], d.sph.0[2], d.sph.1
            );
            let keep = d.flags & 0x20 != 0;
            let spline = d.flags & 1 != 0;
            eprintln!("    bits: keep_open={} spline={} busy={} starts_open={}", keep, spline, d.flags & 0x10 != 0, d.flags & 4 != 0);
            eprintln!("    matrix row3 ({:.3},{:.3},{:.3}) keys {}", d.matrix[3][0], d.matrix[3][1], d.matrix[3][2], if d.keys.is_some() { "yes" } else { "NONE" });
        }
    }

    /// `NF_DOORLOG=1` throttled diagnostic (Task A-9-2): every 60 ticks (≈1 second of
    /// game time at 60 fps) prints, per door, the four values that decide whether the
    /// auto-close path can fire — `state`, `pos01`, `open_timer` — plus the `keep_open`
    /// bit and whether **any** actor in `actors` is currently blocking the proximity
    /// check `dist² < (r1+r2)²·2` (the same `blocked()` test `Door_Update`'s 360-tick
    /// branch and `Door_ReactToHit`'s anti-crush use).
    ///
    /// Why this exists: the auto-close path increments `open_timer` every tick, then at
    /// 360 ticks checks `!actors.iter().any(|a| blocked(sph, *a))` — if any actor is
    /// blocking it skips the close **but still resets `open_timer` to 0**. So a Bond
    /// standing in or near the doorway (within `(door_r + 0.4)·√2` of the sphere centre)
    /// keeps the timer cycling 0→360→0 forever and the door never closes. The unit
    /// tests don't catch this because they park the actor at `[0,1,5]` (door 07000005)
    /// or `[1000,1000,1000]` (door_research_07000001) — far outside the blocking radius.
    /// This diagnostic, run against the live sim, makes the failure visible.
    ///
    /// `actors` is the same list `Drones::tick` builds for `doors.update` (player first
    /// with radius 0.4, then every live non-corpse drone with radius 0.4). The closest
    /// actor's distance² and the door's threshold `(r1+r2)²·2` are printed alongside
    /// so the margin can be read directly. Called from `Drones::tick` after `end_tick`.
    pub fn log_doors_throttled(
        &self,
        world: Option<&crate::world::CollisionWorld>,
        actors: &[DoorActor],
        frame: u32,
    ) {
        if std::env::var_os("NF_DOORLOG").is_none() { return; }
        if frame == 0 || frame % 60 != 0 { return; }
        let _ = world;
        eprintln!("=== NF_DOORLOG @ frame {frame} ({} doors; {} actors) ===", self.list.len(), actors.len());
        eprintln!("idx placement state  pos01  open_timer keep_open blocked blocked_by                     sph.pos                       sph.r   closest_actor (d² / thresh)");
        for (i, d) in self.list.iter().enumerate() {
            let keep = d.flags & 0x20 != 0;
            // The live blocking sphere and the same `blocked()` test the auto-close path runs.
            let live = (transform(&d.matrix, d.sph_centre), d.sph.1);
            let (is_blocked, by) = match visible_actors(actors).find(|a| blocked(live, *a)) {
                Some(a) => (true, format!("({:7.2},{:7.2},{:7.2}) r={:.2}", a.0[0], a.0[1], a.0[2], a.1)),
                None => (false, String::from("-")),
            };
            // The closest actor and its squared distance, plus the threshold the test
            // compares against (`(r1+r2)²·2`); lets you read the margin directly.
            let closest = visible_actors(actors).min_by(|a, b| {
                let da = sub(live.0, a.0); let db = sub(live.0, b.0);
                (da[0]*da[0] + da[1]*da[1] + da[2]*da[2])
                    .partial_cmp(&(db[0]*db[0] + db[1]*db[1] + db[2]*db[2])).unwrap_or(std::cmp::Ordering::Equal)
            });
            let closest_s = closest.map(|a| {
                let dd = sub(d.sph.0, a.0);
                let dist2 = dd[0]*dd[0] + dd[1]*dd[1] + dd[2]*dd[2];
                let r = d.sph.1 + a.1;
                let thresh = r * r + r * r;
                format!("({:7.2},{:7.2},{:7.2}) d²={:6.2} thresh={:6.2}{}",
                    a.0[0], a.0[1], a.0[2], dist2, thresh, if dist2 < thresh { " [BLOCKS]" } else { "" })
            }).unwrap_or_default();
            eprintln!(
                "{i:<3} {:<9} {:#04x}  {:5.3} {:<10} {:<8} {:<7} {:<30} ({:7.2},{:7.2},{:7.2}) {:5.3}  {}",
                d.placement, d.state, d.pos01, d.open_timer, keep, is_blocked, by,
                d.sph.0[0], d.sph.0[1], d.sph.0[2], d.sph.1, closest_s
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channels() -> Channels { Channels::default() }

    /// Two synthetic swing keys: closed at 0.5 (identity) and the ends ±90° about Y.
    fn swing_keys() -> Vec<Key> {
        let q = |a: f32| [0.0, (a / 2.0).sin(), 0.0, (a / 2.0).cos()];
        vec![
            Key { pos: [0.0, 0.0, 0.0], quat: q(-std::f32::consts::FRAC_PI_2) },
            Key { pos: [0.0, 0.0, 0.0], quat: q(0.0) },
            Key { pos: [0.0, 0.0, 0.0], quat: q(std::f32::consts::FRAC_PI_2) },
        ]
    }

    #[test]
    fn slerp_ends_are_the_keys() {
        // t = 0 and t = 1 give the keys back (the slerp weights are exactly 1 and 0 at the ends).
        let a = [0.0, 0.3826834, 0.0, -0.9238795];
        let b = [0.0, 0.9238795, 0.0, -0.3826834];
        for (t, q) in [(0.0, a), (1.0, b)] {
            let s = slerp(a, b, t);
            for k in 0..4 { assert!((s[k] - q[k]).abs() < 1e-6, "{s:?} at {t}"); }
        }
    }

    #[test]
    fn catmull_midpoint_hits_the_middle_key() {
        // Evenly spaced keys: the spline at t = 0.5 of the middle segment is the average of the neighbours.
        let k = |x: f32| Key { pos: [x, 0.0, 0.0], quat: [0.0, 0.0, 0.0, 1.0] };
        let m = catmull(k(0.0).pos, k(1.0).pos, k(2.0).pos, k(3.0).pos, 0.5);
        assert!((m[0] - 1.5).abs() < 1e-5, "{m:?}");
    }

    #[test]
    fn interp_equal_positions_takes_the_key() {
        let keys = swing_keys();
        let m = interp(&keys, 0, 2, 1.0, true).unwrap();   // the closed key
        assert!((m[0][0] - 1.0).abs() < 1e-6 && (m[3][0] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn keyframe_mode_clamps_to_the_far_end() {
        let keys = vec![
            Key { pos: [0.0, 0.0, 0.0], quat: [0.0, 0.0, 0.0, 1.0] },
            Key { pos: [2.0, 0.0, 0.0], quat: [0.0, 0.0, 0.0, 1.0] },
        ];
        let m = interp(&keys, 0, 1, 0.25, false).unwrap();
        assert!((m[3][0] - 0.5).abs() < 1e-6, "{m:?}");
        assert!(interp(&[], 0, 1, 0.0, true).is_none(), "a NULL framelist writes nothing");
    }

    #[test]
    fn swing_quantises_to_whole_ticks() {
        // 1/30 → 0.5/(int)(0.5/(2/30)) = 0.5/7 = 1/14 [L: the snapshot's speed].
        let s = 1.0 / CREATE_FRAME_RATE;
        assert!((0.5 / ((0.5 / (s + s)) as i32 as f32) - 1.0 / 14.0).abs() < 1e-7);
    }

    /// One swing door built like `Doors::create` would (keys at the world origin).
    fn swing_door() -> Doors {
        let keys = swing_keys();
        let mut doors = Doors::default();
        doors.list.push(Door {
            placement: 0, kind: 0, group: 0, ch_open: 0, ch_lock: 0, straddle: false,
            snd_open: 285, snd_close: 284, snd_hit: 320, flags: 1, approach: 0, locked_flag: 0,
            keys: Some(keys), frames: 2, state: CLOSED, pos01: 0.5, speed: 1.0 / 14.0,
            dir: 0.0, swing: -1.0, timer: 0, open_timer: 0, msg_shown: false,
            matrix: quat_matrix([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]),
            inv_closed: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            pos: [0.0, 0.0, 0.0], sph: ([0.0, 1.1, 0.0], 1.28), sph_centre: [0.0, 1.1, 0.0], glist: 7,
        });
        doors
    }

    #[test]
    fn swing_opens_away_from_the_player_and_auto_closes() {
        let mut doors = swing_door();
        let mut ch = channels();
        let mut sounds = Vec::new();
        // A player in front (+Z) of the closed door at the origin: it swings to the −dir side (away).
        let actors = [([0.0, 1.0, 2.0], 0.4f32, false)];
        assert!(doors.activate(0, &actors, true, &mut ch, 0, &mut sounds));
        let d = &doors.list[0];
        assert_eq!(d.state, OPENING);
        assert_eq!(sounds, vec![(285, [0.0, 0.0, 0.0])], "the opening sound");
        // local.z of (0,1,2) in the closed (identity) frame is +2 → side +1 → dir = +swing = −1.
        assert_eq!(d.dir, -1.0);
        // The busy marker clears in `calc`; then the door travels 0.5 → 0 in 7 ticks and ends OPEN. The player
        // stays where he was: near enough that the opening door's anti-crush sees him (local.z = 2, past the
        // margin), far enough that his sphere (r = 0.4) does not block the door's (r = 1.28).
        doors.calc();
        let near = [([0.0, 1.0, 2.0], 0.4f32, false)];
        for _ in 0..8 { sounds.extend(doors.update(0, &mut ch, &near, 0)); doors.calc(); }
        assert_eq!(doors.list[0].state, OPEN, "reached the swung end");
        // Sister: the timer branch needs the state, not this run — an open door with nobody near (the player
        // walked off) closes after the 360-tick timer (`FRAME_RATE_INT·6`) from whenever it reached OPEN.
        let far = [([0.0, 1.0, 5.0], 0.4f32, false)];
        let mut guard = 0;
        while doors.list[0].state == OPEN && guard < 600 { sounds.extend(doors.update(0, &mut ch, &far, 0)); doors.calc(); guard += 1; }
        assert_eq!(doors.list[0].state, CLOSING, "the 6-second timer closed it (after {guard} ticks)");
        let closing = sounds.clone();
        assert!(closing.contains(&(284, [0.0, 0.0, 0.0])), "the closing sound: {sounds:?}");
    }

    #[test]
    fn a_locked_slide_door_only_thuds() {
        let mut doors = Doors::default();
        doors.list.push(Door {
            placement: 0, kind: 2, group: 0, ch_open: 0, ch_lock: 200, straddle: false,
            snd_open: 0xffff, snd_close: 0xffff, snd_hit: 320, flags: 0, approach: 0, locked_flag: 0,
            keys: None, frames: 0xffff, state: CLOSED, pos01: 0.0, speed: 1.0 / 30.0,
            dir: 0.0, swing: 0.0, timer: 0, open_timer: 0, msg_shown: false,
            matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            inv_closed: [[0.0; 4]; 4], pos: [10.0, 0.0, 0.0], sph: ([10.0, 1.1, 0.0], 1.28), sph_centre: [10.0, 1.1, 0.0], glist: 8,
        });
        let mut ch = channels();
        let mut sounds = Vec::new();
        // Channel 200 off and the door closed: the locked feedback only.
        assert!(!doors.activate(0, &[], true, &mut ch, 0, &mut sounds));
        assert_eq!(sounds, vec![(320, [10.0, 0.0, 0.0])]);
        // The rising edge of channel 200 unlocks it (activates, clears the lock field). The edge is against
        // the channels at the end of the last tick (`switch_channels_prev`), so the set stays after it.
        let mut ch2 = channels();
        ch2.set(200, 1, 1);
        let mut doors2 = doors.clone();
        doors2.update(0, &mut ch2, &[], 1);
        assert_eq!(doors2.list[0].ch_lock, 0, "unlocked for good");
        // Sister: without the edge (the channel was already on last tick) the door stays locked.
        let mut doors3 = doors.clone();
        doors3.prev_channels[200] = 1;
        doors3.update(0, &mut ch2, &[], 1);
        assert_eq!(doors3.list[0].ch_lock, 200);
    }

    #[test]
    fn group_members_follow_the_master() {
        let mut doors = Doors::default();
        for k in 0..2 {
            doors.list.push(Door {
                placement: k, kind: 2, group: 5, ch_open: 0, ch_lock: 0, straddle: false,
                snd_open: 0xffff, snd_close: 0xffff, snd_hit: 0xffff, flags: 0, approach: 0, locked_flag: 0,
                keys: None, frames: 0xffff, state: CLOSED, pos01: 0.0, speed: 1.0 / 30.0,
                dir: 0.0, swing: 0.0, timer: 0, open_timer: 0, msg_shown: false,
                matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                inv_closed: [[0.0; 4]; 4], pos: [k as f32, 0.0, 0.0], sph: ([k as f32, 1.1, 0.0], 1.28), sph_centre: [k as f32, 1.1, 0.0], glist: 9,
            });
        }
        let mut ch = channels();
        let mut sounds = Vec::new();
        doors.activate(0, &[], true, &mut ch, 0, &mut sounds);
        doors.calc();
        assert_eq!(doors.list[1].state, OPENING, "the group member follows");
        assert_eq!(doors.list[1].pos01, doors.list[0].pos01);
        assert_eq!(doors.list[0].flags & 0x10, 0, "calc clears the busy markers");
    }
}
