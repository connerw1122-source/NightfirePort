//! The driving missions' AI car step: the state machine, the target-speed selection and the
//! drive-control applier, headless (no drawing, no physics body — the sim object plugs in through
//! [`SimCar`]). Port provenance per item; the full write-up with every reader is
//! docs/systems/driving-ai.md §3.
//!
//! - The per-frame Update is the world scheduler's vtable+0x1c call → `FUN_0012e350` 0x12e350
//!   (280 B): guards the enable flags (+0x58/+0x5c), honors the reset (+0x1a0), then switches on
//!   the state +0x70 (1 standby, 2 retarget, 3 teleport-to-marker, 4 traffic, 5 chase) [M].
//! - Target speed: `FUN_0012e808` 0x12e808 (2,408 B) writes +0x8c. The traffic/chase gates first:
//!   state 5 needs `FUN_00195a30(actor) > 0`, state 4 needs `≥ 0.5` (else 0) [M 0x12e808:9-29];
//!   the actor's stop query (vtbl+0x144) ≠ 0 or byte +0x170 ≠ 0 → 0 [M 0x12e808:41-52]; the base
//!   number is the current segment's speed limit **rs+0x4c** [M 0x12e808:165].
//! - Drive control: `FUN_00133ab0` 0x133ab0 (1,196 B) — steer = clamp(±1) of the alignment dots →
//!   SetSteer (vtbl+0xd4); SetThrottle(0)/SetBrake(0)/SetHandbrake(0) reset first
//!   [M 0x133ab0:135-143]; `speed = |car velocity|` (sim +0x20) [M :103-113];
//!   `target' = max(target·|alignment|, 0.5)` [M :100-102]; arrival: `radius = max(speed·0.5, 5.0)`,
//!   inside it target' = 0 [M :114-130]; forced stop (+0x19c ≠ 0) → throttle 0, brake 0, handbrake 1
//!   [M :173-180]; otherwise `ramp = min(|target' − speed|, 7.5)/7.5`, floored at 0.05, and
//!   `target' > speed` → SetThrottle(ramp), else the brake-side early-outs (speed ≤ 0 → return;
//!   alignment ≥ threshold and target' ≥ 2.5 → return) → SetBrake(ramp) [M :144-172].
//! - Position on the segment: `A + (B − A)·t` [M 0x222468:12-20] (straight; curved through
//!   FUN_00222d40, see `road::RoadNet::curve_point`).

use super::road::{RoadNet, RSF_TURN_RESTR_A, RSF_TURN_RESTR_B};

/// The AIVehicle states (+0x70) [M 0x12e350's switch; docs/systems/driving-ai.md §3].
pub const STATE_STANDBY: u32 = 1;
pub const STATE_RETARGET: u32 = 2;
pub const STATE_TELEPORT: u32 = 3;
pub const STATE_TRAFFIC: u32 = 4;
pub const STATE_CHASE: u32 = 5;

/// The FUN_00133ab0 constants [M — the literals in the decompile].
pub const THROTTLE_RAMP_SPAN: f32 = 7.5;
pub const THROTTLE_FLOOR: f32 = 0.05;
pub const TARGET_FLOOR: f32 = 0.5;
pub const ARRIVAL_MIN_RADIUS: f32 = 5.0;
pub const ARRIVAL_SPEED_FACTOR: f32 = 0.5;

/// The sim-object view the AI reads — `FUN_001ff088` 0x1ff088's fields the AI step touches
/// (+0x10/+0x20 positions, +0x5c facing rows, +0x20 velocity). 506's physics plugs in here.
pub struct SimCar {
    /// sim +0x10: the object position [M 0x133ab0:41].
    pub pos: [f32; 3],
    /// sim +0x20: the velocity [M 0x133ab0:103-113 — the speed is its length].
    pub vel: [f32; 3],
    /// sim +0x5c: the forward row (the facing the dots multiply) [M 0x133ab0:38,58].
    pub forward: [f32; 3],
    /// The actor's alive gate: FUN_00195a30's value [M 0x12e808:12-27].
    pub alive: f32,
}

impl SimCar {
    pub fn speed(&self) -> f32 {
        let s = self.vel[0] * self.vel[0] + self.vel[1] * self.vel[1] + self.vel[2] * self.vel[2];
        sqrt_ps2(s)
    }
}

/// The sqrt the decompile's `_vsqrt` path computes (the port's f32 sqrt; the PS2-exact variant
/// lives in ps2math for the FPU-visible paths — this one feeds a pedal command, not a draw).
fn sqrt_ps2(s: f32) -> f32 {
    if s <= 0.0 {
        0.0
    } else {
        s.sqrt()
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// One drive-control output — the four actor-vtbl calls of FUN_00133ab0 [M :135-180].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveCommand {
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: bool,
}

/// The target-speed gates of FUN_0012e808 (the traffic path) [M 0x12e808:9-52,165].
/// `stop_query` = the actor vtbl+0x144 query result; `force_stop_byte` = the AI's +0x170.
pub fn target_speed_traffic(state: u32, sim: &SimCar, seg_limit: f32, stop_query: i32, force_stop_byte: i8) -> f32 {
    if state == STATE_CHASE && sim.alive <= 0.0 {
        return 0.0; // [M 0x12e808:12-18]
    }
    if state == STATE_TRAFFIC && sim.alive < 0.5 {
        return 0.0; // [M 0x12e808:20-29]
    }
    if stop_query != 0 || force_stop_byte != 0 {
        return 0.0; // [M 0x12e808:41-52]
    }
    seg_limit // the segment's speed limit rs+0x4c [M 0x12e808:165]
}

/// The FUN_00133ab0 drive-control applier [M 0x133ab0:31-181].
/// `aim` = the aim/heading point (*param_3), `target` = the AI's +0x8c target speed
/// (after the caller's profile multiplier), `forced_stop` = the AI's +0x19c.
pub fn drive_control(sim: &SimCar, aim: [f32; 3], target: f32, forced_stop: bool) -> DriveCommand {
    // dir = normalize(aim − pos); the three alignment dots against the facing [M :40-96].
    let d = [aim[0] - sim.pos[0], aim[1] - sim.pos[1], aim[2] - sim.pos[2]];
    let len = sqrt_ps2(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
    let dir = if len > 0.0 {
        [d[0] / len, d[1] / len, d[2] / len]
    } else {
        [0.0; 3]
    };
    let steer = dot(dir, sim.forward).clamp(-1.0, 1.0); // FUN_00210510's clamp [M :135-137]

    let speed = sim.speed(); // |vel| [M :103-113]
    // target' = max(target·|alignment|, 0.5) — the alignment here is the second dot pair's
    // absolute value [M :100-102]; with the aim on the nose it equals |dot(dir, forward)|.
    let aligned = dot(dir, sim.forward).abs();
    let target = (target * aligned).max(TARGET_FLOOR);
    // Arrival: radius = max(speed·0.5, 5.0); inside it the target is 0 [M :114-130].
    let radius = (speed * ARRIVAL_SPEED_FACTOR).max(ARRIVAL_MIN_RADIUS);
    let dist = len;
    let target = if dist <= radius { 0.0 } else { target };

    if forced_stop {
        // throttle 0, brake 0, handbrake 1 [M :173-180]
        return DriveCommand { steer, throttle: 0.0, brake: 0.0, handbrake: true };
    }

    let diff = (target - speed).abs();
    let ramp = (diff.min(THROTTLE_RAMP_SPAN) / THROTTLE_RAMP_SPAN).max(THROTTLE_FLOOR);
    if target > speed {
        DriveCommand { steer, throttle: ramp, brake: 0.0, handbrake: false } // [M :148-151,170-171]
    } else {
        // the brake-side early-outs: speed ≤ 0 → no brake; the brake amount (the pre-division
        // span min(diff, 7.5)) ≤ 1.0 with target' ≥ 2.5 → keep rolling [M :153-168]
        if speed <= 0.0 {
            return DriveCommand { steer, throttle: 0.0, brake: 0.0, handbrake: false };
        }
        if diff.min(THROTTLE_RAMP_SPAN) <= 1.0 && target >= 2.5 {
            return DriveCommand { steer, throttle: 0.0, brake: 0.0, handbrake: false };
        }
        DriveCommand { steer, throttle: 0.0, brake: ramp, handbrake: false } // [M :167-171]
    }
}

/// The headless AI car: the path-state subset (car+0x1d0: segment, end, progress) plus the AI
/// fields the step reads. The integration order mirrors the game's per-frame shape:
/// FUN_0012e808 → FUN_00130508 → FUN_00133ab0 → the position evaluation FUN_00222468.
pub struct AiCar {
    /// AI +0x70: the state [M 0x12e350].
    pub state: u32,
    /// path +0x3e: the current segment index [M 0x2247b0].
    pub segment: usize,
    /// path +0x3c: 0 = toward A, 1 = toward B [M 0x225c48 join].
    pub end: u8,
    /// path +0x98: the progress t along the segment, 0..1 [M 0x2257c0's progress cells; the exact
    /// packing of the three f32s +0x98/9c/a0 is [S] — this port keeps the normalized t].
    pub t: f32,
    /// AI +0x8c: the target speed [M 0x12e808].
    pub target_speed: f32,
    /// AI +0x19c: the forced-stop flag [M 0x133ab0:144].
    pub forced_stop: bool,
    /// The AI's +0x170 stop byte [M 0x12e808:48-52].
    pub force_stop_byte: i8,
    /// The car group (rs+0x5a filter) this car belongs to [M 0x225490:133].
    pub car_group: i8,
    /// The actor's stop query (vtbl+0x144) result [M 0x12e808:41-45].
    pub stop_query: i32,
}

impl AiCar {
    /// A car placed at the start of `segment`, heading A→B (`end` 0) or B→A (`end` 1).
    pub fn on_segment(segment: usize, end: u8, car_group: i8) -> AiCar {
        AiCar {
            state: STATE_TRAFFIC,
            segment,
            end,
            t: 0.0,
            target_speed: 0.0,
            forced_stop: false,
            force_stop_byte: 0,
            car_group,
            stop_query: 0,
        }
    }

    /// The world-space aim the control pass chases: the segment's far node, pulled a little past
    /// the car (the chase arms' aim point; the traffic case's `node B + dir·0.75` teleport offset
    /// is the director's spawn analogue [M 0x126c98:197-208]).
    pub fn aim_point(&self, net: &RoadNet) -> Result<[f32; 3], String> {
        let seg = net.seg(self.segment)?;
        let exit = if self.end == 0 { seg.node_b() } else { seg.node_a() };
        net.node(exit as usize).map(|n| n.pos())
    }

    /// The target speed for this frame [M 0x12e808].
    pub fn select_target_speed(&mut self, net: &RoadNet, sim: &SimCar) -> Result<f32, String> {
        let seg = net.seg(self.segment)?;
        let limit = seg.speed_limit();
        self.target_speed = target_speed_traffic(self.state, sim, limit, self.stop_query, self.force_stop_byte);
        Ok(self.target_speed)
    }

    /// One 60 Hz step: target speed → drive command → the position along the segment. `dt` is the
    /// fixed 1/60 s timestep (docs/atlas/driving-elf.md §1.3); the speed itself comes from `sim`
    /// (506's physics owns the throttle→acceleration conversion).
    pub fn step(&mut self, net: &RoadNet, sim: &mut SimCar, dt: f32) -> Result<DriveCommand, String> {
        if self.state == STATE_STANDBY {
            // state 1: the spawn delay path — no motion [M 0x12e350's case 1]
            return Ok(DriveCommand { steer: 0.0, throttle: 0.0, brake: 0.0, handbrake: false });
        }
        let target = self.select_target_speed(net, sim)?;
        let aim = self.aim_point(net)?;
        let cmd = drive_control(sim, aim, target, self.forced_stop);

        // The position advance: t moves by the distance covered over dt, normalized by the
        // segment's length (the geometric fact of FUN_00222468's evaluation + the progress cells).
        let seg = net.seg(self.segment)?;
        let a = net.node(seg.node_a() as usize)?.pos();
        let b = net.node(seg.node_b() as usize)?.pos();
        let seg_len = sqrt_ps2({
            let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
        });
        let speed = sim.speed();
        if seg_len > 0.0 {
            self.t += speed * dt / seg_len;
        }

        if self.t >= 1.0 {
            // Segment end: the next-segment choice [M 0x2247b0].
            let exit = if self.end == 0 { seg.node_b() } else { seg.node_a() };
            self.t -= 1.0;
            match net.next_segment(self.segment, exit, self.car_group)? {
                Some((next, forward)) => {
                    self.segment = next;
                    // The join's direction rule [M 0x225c48 — dot(dir, rs dir) picks the end]:
                    // leaving through the shared node with the candidate's A-side on it means the
                    // candidate is driven A→B when its node B is the far side.
                    let ns = net.seg(next)?;
                    let shared_is_a = ns.node_a() == exit;
                    self.end = match (shared_is_a, forward) {
                        (true, _) => 0,   // driving A→B
                        (false, _) => 1,  // driving B→A
                    };
                }
                None => {
                    // No exit: the car holds at the node (the game's director reassigns it;
                    // FUN_00126c98's re-teleport is the streaming side, out of the headless scope).
                    self.t = 1.0;
                }
            }
        }

        // The sim position follows the spline [M 0x222468].
        sim.pos = net.seg_point(self.segment, self.t)?;
        Ok(cmd)
    }
}

/// The turn-restriction flags the next-segment walk filters [M 0x2247b0:115-118] — re-exported
/// for the tests.
pub fn is_turn_restricted(flags: u16) -> bool {
    flags & (RSF_TURN_RESTR_A | RSF_TURN_RESTR_B) != 0
}
