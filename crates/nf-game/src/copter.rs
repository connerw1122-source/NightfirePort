//! The helicopter (`Copter_Create` 0x17d340, `Copter_Update` 0x17dbb8, `Copter_Move` 0x17d628;
//! placement type 0x2F — fix-objects2, the full actor — helicopter). 07000008's `Helicopter-Body`
//! (#45) is the mission's last writer: gated on ch 5 (which the ride's end cinematic 0x06000078
//! writes), it flies its spline route and its **destruction** writes ch 0x4B — "Escape with Zoe in
//! gondola" completes and mux #276 fires ch 100, ending the level into 0x0730008 [L mission record
//! 3; M Mission_Update 0x182c60:85-116].
//!
//! The game's model [M unless noted]:
//! - `Copter_Create` (0x17d340): the level tag the block walker builds (`parsemap_block_map_data_dynamic`
//!   0x1d1b68 — the placement's params land at `tag+0x2c + 4·k`) gives the record: +0x70 gate = param 6,
//!   +0x72 out = param 7, +0x74 countdown gate = param 8, +0x76 countdown = param 9; +0x48/+0x54 hold
//!   the SP scripts (params 4/5); the path node (0x1c bytes, `Copter_Init` 0x17e4f0's list) takes
//!   node+0x08 = the framelist pointer (`table[extra[8] low]` — the level bin's 0x19 blocks,
//!   `parsemap_block_path_data` 0x1d24d0), node+0x10 = the key count (extra[7] low), node+0x0c = the
//!   per-tick advance `count·REC_FRAME_RATE·(FRAME_RATE/divisor)` = count/divisor with the divisor =
//!   param 10 (0 → extra[7] high) and `REC_FRAME_RATE = 1/FRAME_RATE` [M GS_SetRefreshRate 0x17d2b8],
//!   node+0x14 = the wrap frame `min(count, param 3)`.
//! - `Copter_Move` (0x17d628): `Spline_Interp(framelist, 0, count, t, out_matrix, out_quat, 2)` — the
//!   disasm 0x17d674-0x17d68c (a0 = node+0x08, a2 = node+0x10, f12 = record+0x5c, a3/t0 = the outs,
//!   t1 = 2) — then `t += node+0x0c`; past the count the single-node route wraps to node+0x14 (the
//!   search re-selects the only same-id node; a +0x74 channel number of 100 sets `obj+0xFE |= 1` at
//!   the wrap, 0x17d740). `SP_SetPosRot(sp, obj+0x90)` moves the SP base (the parts ride it).
//! - `Copter_Update` (0x17dbb8): the gate early-return (a non-zero gate channel that is off returns
//!   at once — the copter does nothing until the gate opens); then the +0x76 countdown pulses +0x74
//!   (:16-22). States: 0 counts the same-id route nodes (+0x78) → 1; 1 patrols (`Copter_Move`, the
//!   cone check every `FRAME_RATE_INT<<2 / DiffMod` = 240/DiffMod ticks: the player within 1e6 square
//!   units, the `COPT_COS/COPT_SIN` cone [M 0x17e468 static ctor: `PS2Sinf(3π/4)`, `PS2Sinf(π/4)` —
//!   both 0.70710678], `Collide_LineOfSight` clear) → 2; 2 attacks (`Music_Event(2,1)` every tick,
//!   `Copter_FireMissile` on `attackmode&1 == 0` else `Copter_FireGun`) back to 1 when the player is
//!   gone / 5·FRAME_RATE·DiffMod ticks / recent damage > 100.
//! - `Copter_FireGun` (0x17d828): both turrets (+0x44/+0x48) `Mat_LookAt` the player's eye (+0.5 y);
//!   the +0x44 turret's counter (+0xF4) at >4 fires `weapon_data[0x71]` (`0x61` on 07000008) from the
//!   turret muzzle, plays `Sound_Play3D(100.0, muzzle, -1.0, -1.0, 0x5df, 0, 0, 0)` (:85), resets,
//!   and readies the +0x48 turret's muzzle flash (+0xF4 = 2, the +0xF0 bit 0x10 cleared).
//! - `Copter_FireMissile` (0x17d9f0): every `60·DiffMod` ticks (`GameState time %` it), the pod by
//!   `Rand_Rand(0x86e6) & 1` (+0x50 left / +0x4c right), the bullet = `weapon_data[0x6d]` (`0x62` on
//!   07000008) from the pod's muzzle; on 07000008 a `Rand_Rand(8/DiffMod) == 0` roll aims at Bond.
//! - Damage (LAB_0017e180): per part i (0..8) the part's `obj+0xD0` HITDATA chain sums
//!   `record.damage · Bits[i].mult` into the total (+0x64) and the recent (+0x6c). `Bits` [L ELF
//!   0x2a37e8, the static .data table — only `Copter_Update` references it]: mult
//!   `[2.0, 3.0, 1.0, 2.0, 100.0, 1.0, 0, 0, 0]`, Bits[4] filter 16, the rest 0.
//! - Destroyed (states 0-3 — the unsigned `1 < state-4` — and `+0x64 ≥ +0x68` = `DiffMod·299`
//!   [M Copter_Create 0x17d340:76-94]): state 6 on 07000012/0700000b — `Explode_Create(10, 10, 67.4,
//!   obj, pos, 0x2d6210, 0x6000052, 0xff, 0xff, 100, 0)` — else state 4: the next `Copter_Move`
//!   completes, the path switches to the death segment (+0x58), `t = 0`, and
//!   **`switch_channels[+0x72] = 1`**; state 5 writes +0x72 again and sets `obj+0xFE |= 1`. The
//!   +0x72 channel is the destruction output — on 07000008 it is 0x4B, the level end.
//!
//! Port status: the channel machine is exact (gate, the countdown pulse, the out write — now
//! destroy-driven); the flight is the real spline; the visuals ride the port's script-player
//! machinery (a synthetic player animates the flight script's entities from the placement's matrix
//! [D fix-objects2]) — the rotor spin is the script's keyed part animation (the game's
//! `Copter_Update` holds no rotor state [M]); `Copter_GetBody` 0x17e4b8's gate check is
//! `visible()`.

use crate::door::{interp as spline_interp, Key};
use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::placement::{object_matrix, Placement};
use nf_formats::anim::Mat4;

/// The static `Bits` damage table [L ELF ACTION.ELF 0x2a37e8 — 9 × (s16 filter @+0, f32 mult @+4);
/// only `Copter_Update` references it]. `(filter, multiplier)` per part 0..8.
pub const BITS: [(u16, f32); 9] = [
    (0, 2.0), (0, 3.0), (0, 1.0), (0, 2.0), (16, 100.0), (0, 1.0), (0, 0.0), (0, 0.0), (0, 0.0),
];

/// `COPT_COS`/`COPT_SIN` [M 0x17e468 `__static_initialization_and_destruction_0`:
/// `PS2Sinf(0x4016cbe4)`, `PS2Sinf(0x3f490fdb)` — 3π/4 and π/4, both → 0.70710678].
const COPT_COS: f32 = 0.707_106_78;
const COPT_SIN: f32 = 0.707_106_78;
/// The attack cone's range squared [M 0x17dbb8 case 1: `Vec_SqDist3D <= 1e6`].
const SPOT_SQ_DIST: f32 = 1_000_000.0;

/// One route node (the game's 0x1c-byte record, the fields `Copter_Move` reads).
#[derive(Clone, Debug)]
pub struct Path {
    /// The framelist keys (28-byte pos + quat records — the port's `door::Key` parse).
    pub keys: Vec<Key>,
    /// node+0x10: the key count (`extra[7]` low).
    pub count: u16,
    /// node+0x0c: the per-tick spline advance = count/divisor [M 0x17d340:33-41].
    pub dt: f32,
    /// node+0x14: the wrap frame (`min(count, param 3)`).
    pub wrap: u16,
    /// node+0x18: the node type (2 = the death segment).
    pub kind: u16,
}

#[derive(Clone, Debug)]
pub struct Copter {
    pub placement: usize,
    /// The body's world matrix (the spline pose — the game's `obj+0x90`; row 3 = `obj+0xC0`).
    pub matrix: Mat4,
    /// The placement's object hash — the draw's mesh key [D the placement's hashcode].
    pub mesh_hash: u32,
    /// record+0x70 = param 6: the gate channel [M 0x17d340:0x17d4ac, 0x17dbb8 head].
    pub gate: u16,
    /// record+0x72 = param 7: written on the DESTRUCTION [M 0x17dbb8:249-268, cases 4/5/6].
    pub out: u16,
    /// record+0x74/+0x76 = params 8/9: the countdown pulse written when the countdown expires
    /// [M 0x17dbb8:16-22]. Both 0 on 07000008 [D #45] — inert there, implemented anyway.
    pub arrival: u16,
    pub countdown: u16,
    /// The scripts (params 4/5): [assembly, flight] [D #45 = 0x06000051 / 0x06000050].
    pub scripts: [u32; 2],
    /// Idle until the gate's rising edge (the game's early return).
    pub started: bool,
    /// Ticks since the flight began (the viewer's flight clock; the game's clock is the spline t).
    pub flight_t: u32,
    /// The spline path node (the game's record+0x54); `None` = the count was 0 (Move no-ops).
    pub path: Option<Path>,
    /// record+0x5c: the spline playhead.
    pub t: f32,
    /// obj+0xF4: 0 init, 1 patrol, 2 attack, 4 exit-switch, 5 exit, 6 explode [M 0x17dbb8].
    pub state: u16,
    /// obj+0xF6: the patrol/attack tick counter.
    pub timer: u16,
    /// 0x30d748: `Rand_Rand(10)` at the spot — bit 0 picks the missile/gun arm [M case 2].
    pub attackmode: u32,
    /// The +0x44 turret's shot counter (its +0xF4) [M 0x17d828:63-65].
    pub gun_shots: u16,
    /// record+0x64: the accumulated damage (the Bits-weighted HITDATA sums).
    pub damage: f32,
    /// record+0x6c: the recent damage (reset on the state edges, the >100 bail-out test).
    pub recent: f32,
    /// record+0x78: the same-id route node count (state 0's walk) [M 0x17dbb8 case 0].
    pub route_nodes: u16,
    /// obj+0xFE bit 0: the destruction finish (the state-5/6 arms; the wrap arm's flag is
    /// `wrap_flag` — the route loops forever, the +0x74 == 100 latch only marks the lap).
    pub done: bool,
    /// obj+0xFE bit 0 from the +0x74 == 100 wrap arm [M 0x17d740] — a per-lap latch, not terminal.
    pub wrap_flag: bool,
    /// `obj+0x68` = 299.0 x the difficulty multiplier [M `Copter_Create` 0x17d340:76-94]:
    /// d1 x1.0 (the decomp's fall-through default), d2 x1.5, d3 x2.0, d4 x4.0.
    pub hp: f32,
    /// The level code (`GameState+0x12`) — stamped by the caller at the first tick (`d.level`,
    /// unavailable at `attach_level`): the 0x08 weapon ids and the case-6 arm read it.
    pub level: u32,
    /// The part hashes: the flight script's streams = parts 0..n, the assembly script's (param 4)
    /// streams continue after (the SP loads both scripts; `Copter_Register` 0x17e530 indexes the
    /// SP's entities 0..8). 0 = none. The bullet pass attributes damage through these.
    pub part_hashes: [u32; 9],
    /// The (script, stream) source per part — the live entity lookup.
    pub part_src: [(u32, u8); 9],
    /// The parts' collide radii (the level's object defs, min 0.5).
    pub part_radii: [f32; 9],
}

/// The per-difficulty health multiplier (`Copter_Create` 0x17d340:76-93): 2 -> 1.5, 3 -> 2.0,
/// 4 -> 4.0, everything else (d1) -> 1.0 [M].
pub fn copter_hp_mult(difficulty: u32) -> f32 {
    match difficulty {
        2 => 1.5,
        3 => 2.0,
        4 => 4.0,
        _ => 1.0,
    }
}

/// The path node from a placement + the level's 0x19 framelist blocks (`extra[7]` low = the count,
/// `extra[8]` low = the block's per-map ordinal, param 10 / `extra[7]` high = the divisor,
/// param 3 = the wrap cap) [M 0x17d340:22-41; D the probe — count == the block's key count on all
/// five SP copters].
pub fn path_from(p: &Placement, path_blocks: &[&[u8]]) -> Option<Path> {
    let count = (p.extra[7] & 0xffff) as u16;
    if count == 0 { return None; }
    let idx = (p.extra[8] & 0xffff) as usize;
    let block = path_blocks.get(idx)?;
    let keys = crate::door::parse_keys(block);
    if keys.is_empty() { return None; }
    let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
    let divisor = if param(10) != 0 { param(10) } else { (p.extra[7] >> 16) as u32 };
    let dt = if divisor == 0 { 0.0 } else { count as f32 / divisor as f32 };
    let wrap = (param(3) as u32).min(count as u32) as u16;
    let kind = param(0) as u16;
    Some(Path { keys, count, dt, wrap, kind })
}

impl Copter {
    /// `Copter_Create` over the main map's 0x2F placements. `difficulty` is the selected
    /// difficulty (`GameState+0x28`, 0x2a3790) — the HP scales with it [M 0x17d340:76-94].
    /// `path_blocks` = the main map's 0x19 payloads in block order (the framelist table).
    pub fn create(placements: &[Placement], difficulty: u32, path_blocks: &[&[u8]]) -> Vec<Copter> {
        let mut out = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0x2f { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            out.push(Copter {
                placement: i,
                matrix: object_matrix(p.rotation, p.translation),
                mesh_hash: p.hashcode,
                gate: param(6) as u16,
                out: param(7) as u16,
                arrival: param(8) as u16,
                countdown: param(9) as u16,
                scripts: [param(4), param(5)],
                started: false,
                flight_t: 0,
                path: path_from(p, path_blocks),
                t: 0.0,
                state: 0,
                timer: 0,
                attackmode: 0,
                gun_shots: 0,
                damage: 0.0,
                recent: 0.0,
                route_nodes: 0,
                done: false,
                wrap_flag: false,
                hp: copter_hp_mult(difficulty) * 299.0,
                level: 0,
                part_hashes: [0; 9],
                part_src: [(0, 0); 9],
                part_radii: [0.0; 9],
            });
        }
        out
    }

    /// `DiffMod` for this copter (`Copter_Create` 0x17d340:76-90 — the same ladder as the HP):
    /// the attack periods read it [M cases 1/2].
    pub fn diff_mod(&self) -> f32 { copter_hp_mult((self.hp / 299.0).round() as u32) }

    /// `Copter_GetBody` 0x17e4b8's gate check: a set-but-off show channel hides the actor.
    pub fn visible(&self, ch: &Channels) -> bool {
        if self.gate != 0 && !ch.on(self.gate as u32) { return false; }
        true
    }

    /// The copter's world position (the matrix row 3) — the viewer's draw/cull helpers.
    pub fn pos(&self) -> V3 { [self.matrix[3][0], self.matrix[3][1], self.matrix[3][2]] }
    /// The full spline pose (`obj+0x90`) — the draw's base.
    pub fn matrix(&self) -> &Mat4 { &self.matrix }

    /// `Copter_Move` 0x17d628 (the single-node route the SP levels carry): the spline step, the
    /// wrap to node+0x14, the +0x74 == 100 wrap flag. Returns the game's return (1 = moved).
    pub fn move_tick(&mut self) -> bool {
        let Some(path) = self.path.as_ref() else { return true };
        if path.count == 0 { return false; }
        if let Some(m) = spline_interp(&path.keys, 0, path.count, self.t, true) {
            self.matrix = m;
        }
        self.t += path.dt;
        if self.t > path.count as f32 {
            // The +0x74 channel number of 100 sets obj+0xFE bit 0 [M 0x17d740] — the lap latch.
            if self.arrival == 100 { self.wrap_flag = true; }
            self.t = path.wrap as f32;
        }
        true
    }

    /// One `Copter_Update` tick. The caller supplies the player's position/eye, the level code,
    /// the shared RNG draw (`attackmode`'s `Rand_Rand(10)` [M case 2] and `FireMissile`'s pod roll
    /// ride the game's shared RNG), the line-of-sight test and the tick's per-part damage sums.
    /// Side effects out: `(gun, missile, music_combat, exploded)` —
    /// gun = `(muzzle, dir, level08_weapon)` for the caller's `weapon_data[0x71/0x61]` bullet +
    /// the 0x5df sound; missile = `(muzzle, aims_at)` for `weapon_data[0x6d/0x62]`.
    #[allow(clippy::type_complexity)]
    pub fn update(
        &mut self,
        ch: &mut Channels,
        frame: u32,
        player_pos: Option<V3>,
        player_eye: V3,
        los: &dyn Fn(V3, V3) -> bool,
        part_damage: &[f32; 9],
        rand: &mut dyn FnMut(u32) -> u32,
    ) -> (Option<(V3, V3, bool)>, Option<(V3, Option<V3>)>, bool, bool) {
        // The gate: a non-zero gate channel that is off stops everything [M 0x17dbb8 head].
        if self.gate != 0 && !ch.on(self.gate as u32) { return (None, None, false, false); }
        // The countdown pulse [M 0x17dbb8:16-22].
        if self.countdown != 0 {
            self.countdown -= 1;
            if self.countdown == 0 && self.arrival != 0 { ch.set(self.arrival as u32, 1, frame); }
        }
        // The flight clock (the viewer's; the game's clock is the spline t).
        if !self.started { self.started = true; self.flight_t = 0; } else { self.flight_t = self.flight_t.saturating_add(1); }
        let mut music_combat = false;
        let mut gun: Option<(V3, V3, bool)> = None;
        let mut missile: Option<(V3, Option<V3>)> = None;

        // The state machine [M 0x17dbb8's switch on obj+0xF4].
        match self.state {
            0 => {
                // Count the same-id nodes [M case 0]; the SP levels carry exactly one.
                self.route_nodes = 1;
                self.state = 1;
            }
            1 | 2 => {
                self.move_tick();
                if self.state == 2 { music_combat = true; }
                let dm = self.diff_mod();
                if self.state == 1 {
                    // The patrol spot check lives inside the +0x74 != 100 conjunct [M 0x17dbb8:140]:
                    // a copter whose record+0x74 is 100 — the ambient patrols (07000005's #241/#242,
                    // 0700000b's #328, all +0x74 = 100 [L level params]) — never increments the
                    // timer, never spots, so it never draws `Rand_Rand(10)`, never fires, never
                    // plays 0x5df and never requests combat music. It just flies its loop forever.
                    if self.arrival != 100 {
                        self.timer = self.timer.saturating_add(1);
                        let period = (240.0 / dm) as u16; // [M FRAME_RATE_INT << 2 = 240]
                        if let Some(pp) = player_pos {
                            // Every 240/DiffMod ticks: the cone + the LOS [M case 1]. The timer is
                            // never reset on a failed check, so past the first period this tests
                            // every tick ([M case 1's fall-through keeps the count]).
                            if self.timer >= period && self.spots(player_eye, los) {
                                self.state = 2;
                                self.timer = 0;
                                self.recent = 0.0;
                                self.attackmode = rand(10);
                                music_combat = true;
                            }
                        }
                    }
                } else {
                    self.timer = self.timer.saturating_add(1);
                    if let Some(pp) = player_pos {
                        // The attack tick fires BEFORE the bail checks [M case 2 :191-198 → :199-242]:
                        // even the tick that loses the sight has already fired.
                        if self.attackmode & 1 == 0 {
                            if let Some(m) = self.fire_missile(frame, rand, dm, player_eye) { missile = Some(m); }
                        } else if let Some(g) = self.fire_gun(player_eye) {
                            gun = Some(g);
                        }
                        // The bail checks [M :199-242]: the timeout (5·60·DiffMod, strict), or
                        // recent > 100, or the sight lost → timer 0, back to patrol. A keep tick
                        // does NOT reset the timer — the stint ends mid-sight at 5·60·DiffMod.
                        let timeout = 5.0 * 60.0 * dm < self.timer as f32;
                        let seen = self.recent <= 100.0 && self.spots(player_eye, los);
                        if timeout || !seen {
                            self.state = 1;
                            self.timer = 0;
                            self.recent = 0.0;
                        }
                    } else {
                        // The game's `iVar4 == 0` arm [M :199-201] lands in the same reset.
                        self.state = 1;
                        self.timer = 0;
                        self.recent = 0.0;
                    }
                }
            }
            4 => {
                // The death-segment switch [M case 4]: the SP levels carry no type-2 node, so the
                // next Move completes, the path pointer goes NULL, `t = 0` and +0x72 writes.
                self.move_tick();
                self.t = 0.0;
                if self.out != 0 { ch.set(self.out as u32, 1, frame); }
                self.state = 5;
            }
            5 => {
                if self.out != 0 { ch.set(self.out as u32, 1, frame); }
                self.done = true;
                return (None, None, false, false);
            }
            6 => {
                // The 0x12/0x0b explode arm [M case 6]: the channel + the blast, then done.
                if self.out != 0 { ch.set(self.out as u32, 1, frame); }
                self.done = true;
                return (None, None, false, true);
            }
            _ => {}
        }

        // The damage scan [M LAB_0017e180]: the parts' Bits-weighted sums.
        let mut recent = 0.0f32;
        for (i, &d) in part_damage.iter().enumerate() {
            if d > 0.0 { recent += d * BITS[i].1; }
        }
        self.damage += recent;
        self.recent += recent;

        // The destruction [M the scan's tail]: states 0-3 (the unsigned `1 < state-4`) and
        // damage >= hp.
        let mut exploded = false;
        if self.state <= 3 && self.damage >= self.hp {
            if matches!(self.level, 0x0700_0012 | 0x0700_000b) {
                self.state = 6;
                exploded = true;
            } else {
                self.state = 4;
            }
            if self.out != 0 { ch.set(self.out as u32, 1, frame); }
        }
        (gun, missile, music_combat, exploded)
    }

    /// The cone + radius + LOS spot test [M case 1's `Vec_SqDist3D`/`Intersect_ConeSphere`/
    /// `Collide_LineOfSight` block]. The cone opens along the body's facing (the matrix row 2,
    /// `Mat_GetDir` 0x1e3398 = row 2 [M door.rs's note]).
    fn spots(&self, player_eye: V3, los: &dyn Fn(V3, V3) -> bool) -> bool {
        let c = self.pos();
        let d = [player_eye[0] - c[0], player_eye[1] - c[1], player_eye[2] - c[2]];
        let sq = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        if sq > SPOT_SQ_DIST { return false; }
        let len = sq.sqrt();
        if len <= 0.0 { return false; }
        // Mat_GetDir: row 2 of the body matrix.
        let fwd = [self.matrix[2][0], self.matrix[2][1], self.matrix[2][2]];
        let cosang = (d[0] * fwd[0] + d[1] * fwd[1] + d[2] * fwd[2]) / len;
        // Intersect_ConeSphere(COS, SIN, ...) — the 45° half-angle cone on the eye point.
        if cosang < COPT_COS { return false; }
        let perp2 = (sq - (cosang * cosang) * sq).max(0.0);
        // The sphere at the eye, radius = the game's `player+0x80` sphere (1 unit stand-in [S]).
        if perp2.sqrt() - COPT_SIN * len > 1.0 { return false; }
        los(c, player_eye)
    }

    /// `Copter_FireGun` 0x17d828: the turret tracks (the port aims from the body — the turret
    /// matrices are the script parts'), the counter at >4 fires `weapon_data[0x71]`/`[0x61]` from
    /// the muzzle, plays 0x5df, resets [M :63-93]. Returns `(muzzle, dir, level08_weapon)`.
    fn fire_gun(&mut self, player_eye: V3) -> Option<(V3, V3, bool)> {
        self.gun_shots += 1;
        if self.gun_shots <= 4 { return None; }
        self.gun_shots = 0;
        let c = self.pos();
        let mut dir = [player_eye[0] - c[0], player_eye[1] - c[1], player_eye[2] - c[2]];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len <= 0.0 { return None; }
        for v in &mut dir { *v /= len; }
        // The muzzle below the nose (the game's turret cell offset; the body nose stands in [S]).
        let muzzle = [c[0] + dir[0] * 2.0, c[1] + dir[1] * 2.0 - 0.5, c[2] + dir[2] * 2.0];
        let level08 = self.level == 0x0700_0008;
        Some((muzzle, dir, level08))
    }

    /// `Copter_FireMissile` 0x17d9f0: every `60·DiffMod` ticks, the pod by the shared RNG
    /// (`Rand_Rand(0x86e6) & 1` — even → +0x50 left), the weapon `0x6d`/`0x62`. Returns
    /// `(muzzle, aims_at)` — `aims_at` = the eye on the 07000008 direct-aim roll (:66-76).
    fn fire_missile(&mut self, frame: u32, rand: &mut dyn FnMut(u32) -> u32, dm: f32, player_eye: V3) -> Option<(V3, Option<V3>)> {
        // The pod pick draws on EVERY call, before the launch gate [M 0x17d9f0:47 `Rand_Rand(0x86e6)`
        // precedes :58-64's frame-divisibility early return] — every state-2 even-attackmode tick
        // steps the shared stream even when nothing launches.
        let left = rand(0x86e6) & 1 == 0;
        let period = (60.0 * dm) as u32;
        if period == 0 || frame % period != 0 { return None; }
        let c = self.pos();
        // The pods ride the wings (the game's part cells; the body's ±2.2 lateral stands in [S]).
        let muzzle = if left { [c[0] - 2.2, c[1], c[2]] } else { [c[0] + 2.2, c[1], c[2]] };
        let aims = self.level == 0x0700_0008 && rand((8.0 / dm) as u32) == 0;
        Some((muzzle, if aims { Some(player_eye) } else { None }))
    }
}
