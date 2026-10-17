//! Sensors (placement type 0x28, class 0x37): `Sensor_Create` 0x1af010, `Sensor_Update` 0x1af618
//! (dispatched through `control_funcs[0x37]`, docs/atlas/object.md), `Sensor_InCone` 0x1af4f0,
//! `Sensor_SetBeam` 0x1af398, `Sensor_Init` 0x1b0868 (the CameraList 0x319930 init — monitor
//! infra, unported), `Sensor_ApplyDamage` 0x1b08a8 (an empty stub: sensors ignore damage).
//!
//! A sensor is the ceiling/wall security camera: it sweeps its 0x19 key-list pose (the same
//! `Spline_Interp` anim the doors/ss use), and watches a CONE — `Intersect_ConeSphere` 0x1ee100
//! against the player, gated by range and line-of-sight. Detection is PLAYER-ONLY: `Sensor_Update`
//! loads `glb_players[0]` once (disasm `lw $s4, -0x7720(0x2e)` @0x1af644 = 0x2d88e0) and passes it
//! as the test object to both `Sensor_InCone` calls (0x1af97c sweep, 0x1afae0 track) — the only
//! two call sites in the ELF, and no `switch_channels` writer is driven by NPC proximity
//! (docs/systems/script-census.md's "trip-by-anything" plain-words summary is NOT what the binary
//! does; see docs/reports/sensor-volumes.md). After 31 ticks (0x1e+) of seeing him it alerts
//! (state 1), tracks him with `Vec_Track` 0x1e20a0 at 0.05/tick, and after 2 s of continuous
//! sight writes **1 to its out switch channel** (param 3) — the alarm signal doors/scripts and
//! the drone AI (`DroneFunc_CheckAlarmRaised` 0x1453a8) listen for. Losing him for 61 ticks
//! (FRAME_RATE_INT) returns the head (state 2). The disable channel (param 4) set → state 3
//! (off/destroyed glist), cleared → re-arm.
//!
//! Not ported (all visual/audio, none of it sim-visible): the lens (param 0) and mount (param 7)
//! gfx proxy objects and their `hashtable_set_object_to_entity_gfx` meshes, the destroyed-glist
//! swap (`obj+0xD8`), the draw-flag bits (`obj+0xF0/0xFA`), the monitor-view hide latch
//! (`SENSOR+0x60`, written by `Monitor_Update` 0x1afcc8 — the monitor console type 0x30 is
//! unported), and every sound (the 0x4AA hum, the 0x4DD servo, the 0x4A9 beeps — `Sound_Play3D`
//! enters no RNG draw but nothing in the sim consumes them; ss.rs skips the same way).

use crate::door::{interp, parse_keys, quat_matrix, Key, Mat4};
use crate::trig::sinf;
use crate::triggers::Channels;
use crate::world::{CollisionWorld, V3};
use nf_formats::placement::{object_matrix, Placement};

/// `SENSOR+0x14` — the aim quaternion the track/return states blend.
type Quat = [f32; 4];

/// `Sensor_Update`'s degrees→half-angle constant (`0.008726647` = π/360, literal in the decompile
/// at 0x1af038/0x1af05c) [M].
const HALF_DEG: f32 = 0.008_726_647;
/// π/2 (the `1.5707964` literal at 0x1af044) [M].
const HALF_PI: f32 = 1.570_796_4;
/// `REC_FRAME_RATE` (0x30d0dc) = 1/FRAME_RATE = 1/60 [M GS_SetRefreshRate 0x17d2b8:16].
const REC: f32 = 1.0 / 60.0;
/// `FRAME_RATE_INT` (0x30d0cc) = 60 [M PS2FrameRate 60 NTSC]: ticks state 1 may lose Bond before
/// returning (the decompile's `FRAME_RATE_INT < obj+0xF6` at 0x1af618:148).
const LOST_LIMIT: i16 = 60;
/// The sweep's alert threshold: `obj+0xF6` counts consecutive in-cone ticks and transitions at
/// `prev > 0x1e` [M 0x1af618:211-216, disasm `slti 0x1f` @0x1af9a4].
const ALERT_SEEN: i16 = 0x1e;
/// `Vec_Track`'s tracking gain (0x3d4ccccd at 0x1afaa8) [M].
const TRACK: f32 = 0.05;
/// State 2's return slerp factor (0x3dcccccd at 0x1afbf0) and its `Quat_IsEqual` epsilon
/// (0x3c23d70a at 0x1afc28) [M].
const RETURN_RATE: f32 = 0.1;
const RETURN_EPS: f32 = 0.01;
/// The detection-time thresholds (the beep acceleration's 2.0/3.0 at 0x1af618:125/128 and the
/// channel write's 2.0 at :154) [M].
const TRIP_TIME: f32 = 2.0;
/// The player's world-sphere radius (`obj+0x8C`) stand-in — the port's documented 0.4 for every
/// body-radius read (triggers.rs touchers, door.rs actors) [S; the live `Control_BuildWorldSph`
/// value is not in the snapshots].
const TARGET_RADIUS: f32 = 0.4;

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot3(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
/// `Mat_GetDir` 0x1e3398: matrix bytes +0x20..+0x2C — row 2 of the row-vector MATRIX [M; the
/// port's Mat4 is the same layout, door.rs "translation in row 3"].
fn dir_of(m: &Mat4) -> V3 { [m[2][0], m[2][1], m[2][2]] }

/// `Quat_MatToQuat` 0x1e03f0: the standard Shepperd method, trace table (1, 2, 0)
/// [L 0x305a10 in ee_snapshot.bin]. Quat order (x, y, z, w) — `param_1[3] = w`.
fn mat_to_quat(m: &Mat4) -> Quat {
    let t = m[0][0] + m[1][1] + m[2][2];
    if t >= 0.0 {
        let s = (t + 1.0).sqrt();
        let k = 0.5 / s;
        [(m[1][2] - m[2][1]) * k, (m[2][0] - m[0][2]) * k, (m[0][1] - m[1][0]) * k, s * 0.5]
    } else {
        // Largest-diagonal branch: the game's index table picks (i, j, k) permutations of (1, 2, 0).
        let (i, j, k) = if m[0][0] > m[1][1] && m[0][0] > m[2][2] { (0usize, 1usize, 2usize) }
            else if m[1][1] > m[2][2] { (1, 2, 0) } else { (2, 0, 1) };
        let s = (m[i][i] - (m[j][j] + m[k][k]) + 1.0).sqrt();
        let mut q = [0.0f32; 4];
        q[i] = s * 0.5;
        let f = if s != 0.0 { 0.5 / s } else { 0.0 };
        q[3] = (m[j][k] - m[k][j]) * f;
        q[j] = (m[i][j] + m[j][i]) * f;
        q[k] = (m[i][k] + m[k][i]) * f;
        q
    }
}

/// `Quat_Slerp_Acc` 0x1e4188 / `Vec_Track`'s inner blend — the same slerp `door.rs` uses for the
/// anim keys: shortest path, `PS2Sinf` weights while `1−|dot| > 0.01`, else the linear blend.
fn slerp(a: Quat, b: Quat, t: f32) -> Quat {
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let neg = d < 0.0;
    if neg { d = -d; }
    let (w0, w1) = if 0.01 < 1.0 - d {
        let ang = d.acos();
        let s = sinf(ang);
        (sinf(ang - t * ang) / s, sinf(t * ang) / s)
    } else { (1.0 - t, t) };
    let w0 = if neg { -w0 } else { w0 };
    [w0 * a[0] + w1 * b[0], w0 * a[1] + w1 * b[1], w0 * a[2] + w1 * b[2], w0 * a[3] + w1 * b[3]]
}

/// `Quat_IsEqual` 0x1e3ae8: every component within `eps`.
fn quat_is_equal(a: Quat, b: Quat, eps: f32) -> bool {
    (a[0] - b[0]).abs() <= eps && (a[1] - b[1]).abs() <= eps && (a[2] - b[2]).abs() <= eps && (a[3] - b[3]).abs() <= eps
}

/// The tracked orientation for `dir`: `Mat_Align2Dir` 0x1df2a8 with the game's constants
/// up (0, 1, 0) [L 0x2d6210] and right fallback (1, 0, 0) [L 0x2d6220]. Row 2 (= `Mat_GetDir`'s
/// axis, what the cone test uses) is exactly `dir`; the roll of rows 0/1 is the standard
/// right-handed up-referenced basis — the game's VU transcription differs only in that roll
/// [S unverified: the roll is invisible to the cone axis and to every channel write].
fn look_quat(dir: V3) -> Quat {
    let z = dir;
    let x = if dot3(z, [0.0, 1.0, 0.0]).abs() > 0.999 { [1.0, 0.0, 0.0] } else { cross([0.0, 1.0, 0.0], z) };
    let xl = dot3(x, x).sqrt();
    let x = if xl > 0.0 { [x[0] / xl, x[1] / xl, x[2] / xl] } else { [1.0, 0.0, 0.0] };
    let y = cross(z, x);
    mat_to_quat(&[[x[0], x[1], x[2], 0.0], [y[0], y[1], y[2], 0.0], [z[0], z[1], z[2], 0.0], [0.0; 4]])
}

/// One sensor: the `SENSOR` instance fields (obj+0xE0) plus the obj_tag state the update walks.
#[derive(Clone, Debug)]
pub struct Sensor {
    /// Index into the main map's placements.
    pub placement: usize,
    /// The placement translation (`obj+0x30` / the cone apex at `obj+0xC0`) [M 0x1af010].
    pub pos: V3,
    /// `obj+0x90` — the current pose matrix (swept / tracked).
    pub matrix: Mat4,
    /// `SENSOR+0x14` — the aim quaternion (only written by track/return; NOT by the sweep, which
    /// poses `obj+0x90` directly [M 0x1af618:174]).
    pub quat: Quat,
    /// The 0x19 key list (`SENSOR+0x24` = `m_paths[record+0x44]`).
    pub keys: Vec<Key>,
    /// `SENSOR+0x30` = key count − 1 (0 when the header count is 0) [M 0x1af010:82-88].
    pub limit: u16,
    /// `SENSOR+0x2C` = `(count−1) · REC_FRAME_RATE · (FRAME_RATE / rate)` [M 0x1af010:89-96] —
    /// the sweep's whole duration is the rate word (`extra[7] >> 16`) in ticks.
    pub step: f32,
    /// `SENSOR+0x28` — the sweep playhead.
    pub t: f32,
    /// `SENSOR+0x38` = `PS2Sinf(angle·HALF_DEG + HALF_PI)` — the cone half-angle cosine [M 0x1af010:63-64].
    pub cos_half: f32,
    /// `SENSOR+0x3C` = `PS2Sinf(angle·HALF_DEG)` — the half-angle sine [M 0x1af010:65-66].
    pub sin_half: f32,
    /// `SENSOR+0x40` = (f32)(i32) param 2 — the cone range [M 0x1af010:67-73].
    pub range: f32,
    /// `SENSOR+0x58` = param 3 — the OUT switch channel, 1 after 2 s of sight [M 0x1af618:154-157].
    pub out: u16,
    /// `SENSOR+0x5A` = param 4 — the disable channel: set → state 3, cleared → re-arm [M 0x1af618:107-117, 259-263].
    pub disable: u16,
    /// `SENSOR+0x5E` = param 6 byte — the monitor group id (monitor infra unported; field map).
    pub monitor_id: u8,
    /// `SENSOR+0x5F` = param 5 byte — when 0 the sensor joins the CameraList [M 0x1af010:135-137].
    pub listed: bool,
    /// param 0 — the lens gfx hash (visual; also arms `Sensor_SetBeam`'s ray [M 0x1af398:67]).
    pub lens: u32,
    /// param 7 — the mount gfx hash (visual).
    pub mount: u32,
    /// param 8 u16 — the monitor-view channel (`Monitor_Update`'s write; unported).
    pub monitor_ch: u16,
    /// `obj+0xF4` — the state: 0 sweep, 1 tracking, 2 returning, 3 disabled.
    pub state: u16,
    /// `SENSOR+0x44` — the detection timer (the 2.0 s trip clock).
    pub detect: f32,
    /// `SENSOR+0x48` — the beep/servo timer (only gates unplayed sounds; kept for the field map).
    pub beep: f32,
    /// `obj+0xF6` — the consecutive seen/lost tick counter (both states use it).
    pub seen: i16,
    /// The last detection result (for the report/tests; not a game field).
    pub last_hit: bool,
}

impl Sensor {
    /// `Sensor_InCone` 0x1af4f0 against the player [M; detection is player-only — disasm
    /// 0x1af644/0x1af97c/0x1afae0 pass `glb_players[0]` and nothing else ever reaches here].
    fn in_cone(&self, world: &CollisionWorld, player: &crate::drone::Player) -> bool {
        let target_sph = player.sph;
        // `Sensor_SetBeam` 0x1af398's shortcut: with a lens, the beam ray (pos + dir·range)'s
        // FIRST hit being the tested object detects it outright (the decompile returns 1 before
        // the cone test). The port's ray is the static world + a sphere test on the target —
        // other dynamics between the lens and the target are not identified [S stand-in; the
        // port's ray-vs-dynamic model has no object-identifying raycast yet].
        if self.lens != 0 {
            let dir = dir_of(&self.matrix);
            let end = [self.pos[0] + dir[0] * self.range, self.pos[1] + dir[1] * self.range, self.pos[2] + dir[2] * self.range];
            if world.line_of_sight(self.pos, end) && segment_hits_sphere(self.pos, end, target_sph, TARGET_RADIUS) {
                return true;
            }
        }
        // The range gate [M 0x1af4f0:60-62]: `sqdist ≤ (range + target radius)²`.
        let d = sub(target_sph, self.pos);
        let d2 = dot3(d, d);
        let rr = self.range + TARGET_RADIUS;
        if d2 > rr * rr { return false; }
        // `Intersect_ConeSphere` 0x1ee100 (cos = SENSOR+0x38, sin = SENSOR+0x3C, apex = the
        // sensor position, dir = `Mat_GetDir`, sphere = the target's world sphere) [M].
        let dir = dir_of(&self.matrix);
        let proj = dot3(d, dir);
        let hit = if proj * proj >= d2 * self.cos_half * self.cos_half && proj > 0.0 {
            true
        } else {
            let t = self.cos_half * proj + self.sin_half * (d2 - proj * proj).abs().sqrt();
            t >= 0.0 && t * t - d2 + TARGET_RADIUS * TARGET_RADIUS >= 0.0
        };
        if !hit { return false; }
        // The line-of-sight gate [M 0x1af4f0:84-87 `Collide_LineOfSight(pos, target+0x30, …, 10)`]
        // — the port's static-only `line_of_sight` is the same stand-in drone vision uses.
        world.line_of_sight(self.pos, player.pos)
    }

    /// One `Sensor_Update` 0x1af618 tick.
    pub fn update(&mut self, world: &CollisionWorld, player: &crate::drone::Player, ch: &mut Channels, frame: u32) {
        // The preamble [M 0x1af618:96-117]: while the disable channel (SENSOR+0x5A) is set the
        // sensor is pinned to state 3 (the 0x4AA hum stop is unplayed).
        if self.disable != 0 && ch.on(self.disable as u32) {
            self.state = 3;
        }
        match self.state {
            0 => {
                // State 0 clears the out channel every tick [M 0x1af618:169-171] and resets the
                // detection timer [:172].
                if self.out != 0 { ch.set(self.out as u32, 0, frame); }
                self.detect = 0.0;
                // The sweep [M 0x1af618:173-203]: pose at the playhead, advance
                // `t += step · FRAME_RATE_MUL` (1.0 at 60 Hz), wrap past the limit. The servo
                // sounds (0x4DD at the half and at the wrap) are unplayed.
                if self.limit != 0 {
                    if let Some(m) = interp(&self.keys, 0, self.limit, self.t, true) { self.matrix = m; }
                    self.t += self.step;
                    if self.t > self.limit as f32 { self.t = 0.0; }
                }
                // Detection [M 0x1af618:204-222]: hit → count; `prev > 0x1e` → alert (state 1).
                let hit = self.in_cone(world, player);
                self.last_hit = hit;
                if hit {
                    let prev = self.seen;
                    self.seen += 1;
                    if prev > ALERT_SEEN {
                        self.detect = 0.0;
                        self.beep = 0.0;
                        self.state = 1;
                    }
                } else {
                    self.seen = 0;
                }
            }
            1 => {
                // State 1 tracks Bond [M 0x1af618:137-138 `Vec_Track(0.05, players[0]+0x80,
                // obj+0x30, obj+0x90, SENSOR+0x14)`]: slerp the pose toward facing his sphere.
                let qa = mat_to_quat(&self.matrix);
                let d = sub(player.sph, self.pos);
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                let dir = if len > 0.0 { [d[0] / len, d[1] / len, d[2] / len] } else { dir_of(&self.matrix) };
                let qb = look_quat(dir);
                self.quat = slerp(qa, qb, TRACK);
                self.matrix = quat_matrix(self.quat, self.pos);
                // Sight bookkeeping [M 0x1af618:139-150]: 61 lost ticks (FRAME_RATE_INT) → return.
                let hit = self.in_cone(world, player);
                self.last_hit = hit;
                if hit {
                    self.seen = 0;
                } else {
                    self.seen += 1;
                }
                if LOST_LIMIT < self.seen {
                    self.state = 2;
                } else {
                    // The trip [M 0x1af618:152-157]: `detect += REC_FRAME_RATE`; at ≥ 2.0 with an
                    // out channel: `switch_channels[out] = 1` + the time stamp — re-written and
                    // re-stamped every tick while the sight holds (a plain array write in the
                    // game; `Channels::set`'s nonzero stamp matches `switch_channels_time`).
                    self.detect += REC;
                    if self.detect >= TRIP_TIME && self.out != 0 {
                        ch.set(self.out as u32, 1, frame);
                    }
                }
            }
            2 => {
                // State 2 returns [M 0x1af618:225-243]: clear the channel, slerp the aim quat
                // toward the sweep pose at the current playhead, state 0 when equal.
                self.beep = 0.0;
                if self.out != 0 { ch.set(self.out as u32, 0, frame); }
                self.seen = 0;
                match interp(&self.keys, 0, self.limit, self.t, true).map(|m| mat_to_quat(&m)) {
                    Some(tq) => {
                        self.quat = slerp(self.quat, tq, RETURN_RATE);
                        self.matrix = quat_matrix(self.quat, self.pos);
                        if quat_is_equal(self.quat, tq, RETURN_EPS) { self.state = 0; }
                    }
                    // No keys: `Spline_Interp` writes nothing (NULL framelist [M 0x1e1858]), the
                    // game's target quat is uninitialised stack — unobservable; the port returns
                    // straight to the sweep [S unverified].
                    None => self.state = 0,
                }
            }
            _ => {
                // State 3 (disabled): the destroyed-glist swap and the lens hide are visual; the
                // re-arm [M 0x1af618:259-263] waits for the disable channel to clear.
                if self.disable != 0 && !ch.on(self.disable as u32) {
                    self.state = 0;
                }
            }
        }
    }

    /// The placement index (the order-chain sort key comes from it).
    pub fn pos(&self) -> V3 { self.pos }
}

/// The closest-approach segment/sphere test (the same shape as drone.rs's `segment_hits_sphere`).
fn segment_hits_sphere(a: V3, b: V3, c: V3, r: f32) -> bool {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let len2 = dot3(ab, ab);
    let t = if len2 <= 0.0 { 0.0 } else { (dot3(ac, ab) / len2).clamp(0.0, 1.0) };
    let d = [ac[0] - ab[0] * t, ac[1] - ab[1] * t, ac[2] - ab[2] * t];
    dot3(d, d) <= r * r
}

/// Every sensor of the level, in placement order (`Sensor_Create` order — `parsemap_create_dynamic_objects`
/// 0x1d04f0 case 0x28, no RNG draw [M 0x1af010 has none]).
#[derive(Clone, Debug, Default)]
pub struct Sensors { pub list: Vec<Sensor> }

impl Sensors {
    /// `Sensor_Create` over the main map's 0x28 placements [M]. Params (level_tag → `Placement`):
    /// p0 lens hash, p1 cone angle (degrees), p2 range, p3 out channel, p4 disable channel,
    /// p5 skip-CameraList byte, p6 monitor id byte, p7 mount hash, p8 monitor channel
    /// [M 0x1af010 + docs/fields/level-tag.md's sensor row].
    pub fn create(placements: &[Placement], path_blocks: &[&[u8]]) -> Sensors {
        let mut list = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0x28 { continue; }
            // The spawn gate every creator shares (bit 0x8000 static cel, 0x2000 template).
            if p.flags & 0xa000 != 0 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let count = (p.extra[7] & 0xffff) as usize;
            let limit = if count == 0 { 0 } else { count - 1 };
            let index = (p.extra[8] & 0xffff) as usize;
            let keys = path_blocks.get(index).map(|payload| parse_keys(payload)).unwrap_or_default();
            let rate = (p.extra[7] >> 16) as f32;
            // `SENSOR+0x2C` in the game's op order [M 0x1af010:89-96]: (count−1) · (1/60) · (60/rate).
            let step = if rate > 0.0 { limit as f32 * REC * (60.0 / rate) } else { 0.0 };
            let angle = param(1) as f32;
            let matrix = object_matrix(p.rotation, p.translation);
            list.push(Sensor {
                placement: i,
                pos: p.translation,
                matrix,
                quat: mat_to_quat(&matrix),
                keys,
                limit: limit as u16,
                step,
                t: 0.0,
                cos_half: sinf(angle * HALF_DEG + HALF_PI),
                sin_half: sinf(angle * HALF_DEG),
                range: param(2) as i32 as f32,
                out: param(3) as u16,
                disable: param(4) as u16,
                monitor_id: param(6) as u8,
                listed: param(5) as u8 == 0,
                lens: param(0),
                mount: param(7),
                monitor_ch: param(8) as u16,
                state: 0,
                detect: 0.0,
                beep: 0.0,
                seen: 0,
                last_hit: false,
            });
        }
        Sensors { list }
    }

    /// One sensor's update (the control-list walk calls per object, like `Doors::update`).
    pub fn update(&mut self, index: usize, world: &CollisionWorld, player: &crate::drone::Player, ch: &mut Channels, frame: u32) {
        if let Some(s) = self.list.get_mut(index) {
            s.update(world, player, ch, frame);
        }
    }

    /// The sensor of a placement index.
    pub fn of_placement(&self, placement: usize) -> Option<usize> {
        self.list.iter().position(|s| s.placement == placement)
    }
}
