//! The driving missions' car physics (DRIVING.ELF's CARP layer), headless — the tuning DB, the
//! per-car parameters, and the player car's per-tick step at the game's fixed 1/60 s.
//!
//! Evidence base (all [M] unless noted — DRIVING.ELF decompile under `work/decomp/DRIVING.ELF/`,
//! addresses = EE; docs/systems/drive-physics.md is the full map):
//! - Tick rate: `FUN_001feda0` ("Reset Sim") stores `fps = FUN_002d6b90()` (the {60,60,50,60}
//!   table [M 0x2d6b90]) at sim+0xdbc and `1/fps` at sim+0xdc0 = `DAT_00339f50` [M :9-12]; the
//!   frame tick `FUN_00159d10` adds 0.016666668 per pass [M :29]. NTSC = 60 Hz.
//! - Tuning: the `.tun` files are ASCII `KEY=VALUE` text [D], looked up per registered variable
//!   by substring (`FUN_001426a8`: find the name, next char `' '`/`'='`, skip `=#([`, sscanf
//!   `%d/%u/%f`); no match keeps the compiled-in default [M]. The Physics namespaces' registered
//!   defaults equal the shipped `data\tuning\Physics\{Rigid,Physical,Friction}\default.tun`
//!   values, which are byte-identical in all 8 mission VIVs [D].
//! - Engine/gears/steering: `FUN_0018cef8` + the auto-shift/RPM `FUN_0018aa88`; tyre/lateral
//!   forces `FUN_00190f88`; gravity + ground damping `FUN_001f7c60`; the force API
//!   `FUN_001f87d8` (accel·mass accumulators); the integration is VU0 microcode
//!   (`FUN_001f89e0` → `.vutext`) — modelled as semi-implicit Euler here [S], see the doc §6.
//! - Per-car parameters: the pvehicle attribute schema [M `FUN_001968f8`]; the shipped
//!   `data\sim\attrib\pvehicle\vanquish.atr` (Bond's Aston Martin) [D].

use std::collections::HashMap;

pub mod mission;

/// The game's fixed physics step [M `FUN_002d6b90`'s {60,60,50,60} table; sim+0xdc0 = 1/fps].
pub const DT: f32 = 1.0 / 60.0;

// ---------------------------------------------------------------------------
// The .tun text database — `KEY=VALUE` lines [M FUN_001426a8's lookup semantics]
// ---------------------------------------------------------------------------

/// One open tuning file: the parsed `KEY=VALUE` pairs. The game keeps the text in memory and
/// substring-searches it per registered variable name (`FUN_001790f8`), then sscanf's the number
/// [M 0x1426a8]; the parser here keeps the same case-sensitivity and the same "first line whose
/// start matches" semantics at the granularity the port's variables need.
#[derive(Debug, Default, Clone)]
pub struct Tuning {
    values: HashMap<String, f32>,
}

impl Tuning {
    /// Parses `KEY=VALUE` text ([D] layout: CRLF lines, no sections; `//` text after a value is
    /// not part of the .tun form — the sscanf `%f` stops at the first non-numeric char, so a
    /// trailing space or letter is ignored exactly like `%f` does).
    pub fn parse(text: &str) -> Tuning {
        let mut values = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            let Some((key, value)) = line.split_once('=') else { continue };
            let key = key.trim();
            // sscanf("%f") semantics: leading whitespace skip, then the longest float prefix.
            let v = value.trim();
            let num: &str = v.split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
                .next().unwrap_or("");
            if let Ok(f) = num.parse::<f32>() {
                values.insert(key.to_string(), f);
            }
        }
        Tuning { values }
    }

    /// The substring lookup the game does: the registered variable name matches when the file
    /// holds a key that starts with it followed by ` ` or `=` [M 0x1426a8/0x1790f8]. The port's
    /// keys are exact (the .tun files spell them the way the code registers them), so an exact
    /// hit first, then the longest registered-prefix match.
    pub fn get(&self, name: &str) -> Option<f32> {
        if let Some(v) = self.values.get(name) { return Some(*v); }
        let mut best: Option<(&String, &f32)> = None;
        for (k, v) in &self.values {
            if k.starts_with(name) && k.len() > name.len() {
                let rest = &k[name.len()..];
                if rest.starts_with(' ') || rest.starts_with('=') || rest.starts_with('[') {
                    if best.map_or(true, |(bk, _)| k.len() < bk.len()) { best = Some((k, v)); }
                }
            }
        }
        best.map(|(_, v)| *v)
    }
}

// ---------------------------------------------------------------------------
// The Physics:Rigid globals [M FUN_001f6600 registers; .data initials 0x3390d0-0x339164]
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Rigid {
    pub gravity: f32,
    pub bodge_player_gravity: f32,
    pub sleep_vel: f32,
    pub ground_friction_coeff: f32,
    pub natural_angular_damping: f32,
    pub am_limit: f32,
    pub m_limit: f32,
    pub speed_force_limit: f32,
    pub speed_scale: f32,
    pub depth_force_scale: f32,
    pub total_force_limit: f32,
}

impl Rigid {
    /// The compiled-in initials — verified equal to the shipped `Rigid\default.tun` [M .data
    /// 0x3390d0-0x339164 via the decompile's DAT_0033xxxx reads; D the shipped file].
    pub fn game_defaults() -> Rigid {
        Rigid {
            gravity: -34.0,
            bodge_player_gravity: -13.0,
            sleep_vel: 5.0,
            ground_friction_coeff: 0.05,
            natural_angular_damping: 0.995,
            am_limit: 10.0,
            m_limit: 100.0,
            speed_force_limit: 10.0,
            speed_scale: 0.25,
            depth_force_scale: 4.5,
            total_force_limit: 40.0,
        }
    }

    /// The tun override pass — the game assigns each registered var from the file, keeping the
    /// initial when the name is absent [M 0x1426a8's no-match arm].
    pub fn from_tun(t: &Tuning) -> Rigid {
        let mut r = Rigid::game_defaults();
        let g = |t: &Tuning, k: &str| t.get(k);
        if let Some(v) = g(t, "GRAVITY") { r.gravity = v; }
        if let Some(v) = g(t, "BODGE_PLAYER_GRAVITY") { r.bodge_player_gravity = v; }
        if let Some(v) = g(t, "SLEEP_VEL") { r.sleep_vel = v; }
        if let Some(v) = g(t, "GROUND_FRICTION_COEFF") { r.ground_friction_coeff = v; }
        if let Some(v) = g(t, "NATURAL_ANGULAR_DAMPING") { r.natural_angular_damping = v; }
        if let Some(v) = g(t, "AM_LIMIT") { r.am_limit = v; }
        if let Some(v) = g(t, "M_LIMIT") { r.m_limit = v; }
        if let Some(v) = g(t, "SPEED_FORCE_LIMIT") { r.speed_force_limit = v; }
        if let Some(v) = g(t, "SPEED_SCALE") { r.speed_scale = v; }
        if let Some(v) = g(t, "DEPTH_FORCE_SCALE") { r.depth_force_scale = v; }
        if let Some(v) = g(t, "TOTAL_FORCE_LIMIT") { r.total_force_limit = v; }
        r
    }
}

// ---------------------------------------------------------------------------
// Physics:Friction — the 13-surface tables [M FUN_00185668's tail; .data 0x3319d8/0x331a18]
// ---------------------------------------------------------------------------

/// The `WSurface_k*` order the code registers: kNODRIVE, kPAVED, kGRAVEL, kGRASS, kCOBBLE,
/// kDIRT, kWATER, kWOOD, kICE, kSNOW, kPAVED_ROUGH, kPAVED_GRATE, kRAILROAD [M the enum's
/// registration order; the array bases 0x3319d8/0x331a18 stride 4].
pub const SURFACES: [&str; 13] = [
    "NODRIVE", "PAVED", "GRAVEL", "GRASS", "COBBLE", "DIRT", "WATER", "WOOD", "ICE", "SNOW",
    "PAVED_ROUGH", "PAVED_GRATE", "RAILROAD",
];

#[derive(Debug, Clone)]
pub struct Friction {
    pub friction: [f32; 13],
    pub lateral_loss: [f32; 13],
}

impl Friction {
    /// The compiled-in initials: all 1.0 [M .data 0x3319d8..; the shipped Friction\default.tun's
    /// `frictionCoefficients[WSurface_k*]` keys cannot match the registered `friction[k*]` names
    /// under the substring rule — dead values, defaults apply [S; live check requested]].
    pub fn game_defaults() -> Friction {
        Friction { friction: [1.0; 13], lateral_loss: [1.0; 13] }
    }
}

// ---------------------------------------------------------------------------
// The per-car parameters — the pvehicle attribute schema [M FUN_001968f8] + the shipped .atr [D]
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CarParams {
    pub mass: f32,
    pub maxacc: f32,
    pub maxrevacc: f32,
    pub maxbrake: f32,
    pub maxsteering: f32,
    pub modsteer_speed: f32,
    pub min_steer: f32,
    /// [GEAR_LIMITR, LIMIT1..LIMIT5] [M the +0xd0 base indexed by gear 0..5]
    pub gear_limit: [f32; 6],
    /// [GEAR_RATIOR, RATIO1..RATIO5] [M the +0xe8 base indexed by gear 0..5]
    pub gear_ratio: [f32; 6],
    pub friction_limit_front: f32,
    pub friction_limit_rear: f32,
    pub tyre_grip_factor: f32,
    pub wheel_radius: f32,
    pub braking_friction: f32,
    pub front_slipping_scale: f32,
    pub rear_slipping_scale: f32,
    pub slipping_range: f32,
    pub speed_cutoff_reverse: f32,
    pub speed_cutoff_rate_reverse: f32,
    pub handbrake_force: f32,
    pub total_handbrake_friction_scale: f32,
    pub boost_extra_acc: f32,
    pub wheel_zf_offset: f32,
    pub wheel_zr_offset: f32,
}

impl CarParams {
    /// The schema defaults [M FUN_001968f8's registration call list — the values in the doc §2].
    pub fn game_defaults() -> CarParams {
        CarParams {
            mass: 2000.0,
            maxacc: 30.0,
            maxrevacc: 30.0,
            maxbrake: 30.0,
            maxsteering: 0.2,
            modsteer_speed: 50.0,
            min_steer: 0.0,
            gear_limit: [100.0; 6],
            gear_ratio: [1.0; 6],
            friction_limit_front: 50.0,
            friction_limit_rear: 50.0,
            tyre_grip_factor: 16.0,
            wheel_radius: 1.0,
            braking_friction: 3.0,
            front_slipping_scale: 1.0,
            rear_slipping_scale: 1.0,
            slipping_range: 10.0,
            speed_cutoff_reverse: 200.0,
            speed_cutoff_rate_reverse: 1000.0,
            handbrake_force: 30.0,
            total_handbrake_friction_scale: 1.0,
            boost_extra_acc: 0.0,
            wheel_zf_offset: -3.0,
            wheel_zr_offset: 3.0,
        }
    }

    /// Bond's Aston Martin Vanquish — the shipped `data\sim\attrib\pvehicle\vanquish.atr` [D;
    /// the file is RefPack-compressed in every mission VIV, byte-identical everywhere].
    pub fn vanquish() -> CarParams {
        CarParams {
            mass: 1660.0,
            maxacc: 3.0,
            maxrevacc: 2.4,
            maxbrake: 3.15,
            maxsteering: 0.09,
            modsteer_speed: 10.0,
            min_steer: 0.425,
            gear_limit: [10.0, 18.0, 29.0, 50.0, 54.0, 69.0],
            gear_ratio: [1.0, 0.65, 0.59, 0.52, 0.45, 0.44],
            friction_limit_front: 19.0,
            friction_limit_rear: 19.0,
            tyre_grip_factor: 8.0,
            wheel_radius: 0.365,
            braking_friction: 0.3,
            front_slipping_scale: 0.421,
            rear_slipping_scale: 0.425,
            slipping_range: 25.0,
            speed_cutoff_reverse: 10.0,
            speed_cutoff_rate_reverse: 0.075,
            handbrake_force: 0.1,
            total_handbrake_friction_scale: 0.15,
            boost_extra_acc: 1.75,
            wheel_zf_offset: -0.894,
            wheel_zr_offset: 1.006,
        }
    }

    /// Parses the pvehicle `.atr` INI text: `[section]` headers (ignored — one car per file),
    /// `KEY.suffix=VALUE` (the suffix `.f/.i/.u/.s/.v` selects the attribute's type [D]; the
    /// port reads the numeric ones), `//` comments, whitespace-padded values [D vanquish.atr].
    pub fn parse_atr(text: &str) -> CarParams {
        let mut p = CarParams::game_defaults();
        for line in text.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('[') { continue; }
            let Some((key, value)) = line.split_once('=') else { continue };
            // Strip the type suffix: the port reads the value the same way the attribute system
            // does regardless of .f/.i/.u (the .s strings and .v vec4s the port skips).
            let key = key.trim();
            let key = key.split('.').next().unwrap_or(key).trim();
            let num: &str = value.trim()
                .split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
                .next().unwrap_or("");
            let Ok(v) = num.parse::<f32>() else { continue };
            match key {
                "MASS" => p.mass = v,
                "MAXACC" => p.maxacc = v,
                "MAXREVACC" => p.maxrevacc = v,
                "MAXBRAKE" => p.maxbrake = v,
                "MAXSTEERING" => p.maxsteering = v,
                "MODSTEERSPEED" => p.modsteer_speed = v,
                "MINSTEER" => p.min_steer = v,
                "GEAR_LIMITR" => p.gear_limit[0] = v,
                "GEAR_LIMIT1" => p.gear_limit[1] = v,
                "GEAR_LIMIT2" => p.gear_limit[2] = v,
                "GEAR_LIMIT3" => p.gear_limit[3] = v,
                "GEAR_LIMIT4" => p.gear_limit[4] = v,
                "GEAR_LIMIT5" => p.gear_limit[5] = v,
                "GEAR_RATIOR" => p.gear_ratio[0] = v,
                "GEAR_RATIO1" => p.gear_ratio[1] = v,
                "GEAR_RATIO2" => p.gear_ratio[2] = v,
                "GEAR_RATIO3" => p.gear_ratio[3] = v,
                "GEAR_RATIO4" => p.gear_ratio[4] = v,
                "GEAR_RATIO5" => p.gear_ratio[5] = v,
                "FRICTIONLIMITFRONT" => p.friction_limit_front = v,
                "FRICTIONLIMITREAR" => p.friction_limit_rear = v,
                "TYREGRIPFACTOR" => p.tyre_grip_factor = v,
                "WHEEL_RADIUS" => p.wheel_radius = v,
                "BRAKING_FRICTION" => p.braking_friction = v,
                "FRONT_SLIPPING_SCALE" => p.front_slipping_scale = v,
                "REAR_SLIPPING_SCALE" => p.rear_slipping_scale = v,
                "SLIPPING_RANGE" => p.slipping_range = v,
                "SPEED_CUTOFF_REVERSE" => p.speed_cutoff_reverse = v,
                "SPEED_CUTOFF_RATE_REVERSE" => p.speed_cutoff_rate_reverse = v,
                "HANDBRAKE_FORCE" => p.handbrake_force = v,
                "TOTAL_HANDBRAKE_FRICTION_SCALE" => p.total_handbrake_friction_scale = v,
                "BOOST_EXTRA_ACC" => p.boost_extra_acc = v,
                "CAR_WHEEL_ZF_OFFSET" => p.wheel_zf_offset = v,
                "CAR_WHEEL_ZR_OFFSET" => p.wheel_zr_offset = v,
                _ => {}
            }
        }
        p
    }
}

// ---------------------------------------------------------------------------
// The car state and the per-tick step
// ---------------------------------------------------------------------------

/// One pad's car input, the fields the controller reads [M actor+0x240 steer / +0x250 throttle /
/// +0x254 brake; the handbrake rides the +0x29b byte path in FUN_0018cef8:288-309].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarInput {
    pub throttle: f32,
    pub brake: f32,
    /// −1..1, the pad's X [M +0x240; FUN_002cc590(steer) at 0x18cef8:475]
    pub steer: f32,
    pub handbrake: bool,
}

impl Default for CarInput {
    fn default() -> Self { CarInput { throttle: 0.0, brake: 0.0, steer: 0.0, handbrake: false } }
}

/// The flat test surface: the road's surface kind (the Friction table row) [S — the port's
/// stand-in for the track road data; the game's car↔world pass FUN_001fc2f0 tests the level's
/// own quads, which the port does not load].
#[derive(Debug, Clone, Copy)]
pub struct FlatWorld {
    /// The `SURFACES` index the car drives on (2 = GRAVEL, 3 = GRASS, 9 = SNOW…).
    pub surface: usize,
}

impl FlatWorld {
    pub fn paved() -> FlatWorld { FlatWorld { surface: 1 } }
}

/// The player car's state — the fields the port's step integrates, named for the game's
/// offsets (actor+0x… / instance+0x… per docs/systems/drive-physics.md §2).
#[derive(Debug, Clone)]
pub struct Car {
    pub params: CarParams,
    /// instance+0x10 (the sim position; x/y/z, y = height)
    pub pos: [f32; 3],
    /// the car's heading (the class orientation matrix's yaw, class+0x00-0x3f)
    pub yaw: f32,
    /// instance+0x20 (the integrated linear velocity)
    pub vel: [f32; 3],
    /// the yaw rate the tyre forces feed (instance+0x50's spin, y lane)
    pub yaw_rate: f32,
    /// actor+0x2d8 (the auto gearbox's gear, 0 = reverse, 1..5 forward)
    pub gear: u8,
    /// actor+0x2d9 (last tick's gear — the anti-hunt hold [M 0x18aa88:53-56])
    pub last_gear: u8,
    /// actor+0x2d7 — the shift inhibit: 0xf on every gear change [M 0x18cef8:412], decremented
    /// once per tick [M 0x18cef8:911-912]; the auto walk's DOWNshift is held while it stands
    /// [M 0x18aa88:53-56].
    pub shift_inhibit: u8,
    /// actor+0x28c (the steer/slip angle the engine pass writes)
    pub steer_angle: f32,
    /// actor+0x288 (|velocity|, recomputed per tick [M 0x190f88:394])
    pub speed: f32,
    /// the engine rpm FUN_0018aa88 returns
    pub rpm: f32,
    /// true while the reverse state stands [M actor+0x29c]
    pub reversing: bool,
    /// grounded (the flat surface holds the car when true)
    pub grounded: bool,
    /// the working gravity this car integrates under (−13 bodge / −34 named) [M 0x1f7c60:136-144]
    pub gravity: f32,
}

impl Car {
    pub fn new(params: CarParams, pos: [f32; 3], yaw: f32) -> Car {
        let gravity = Rigid::game_defaults().bodge_player_gravity;
        Car {
            params, pos, yaw, vel: [0.0; 3], yaw_rate: 0.0, gear: 1, last_gear: 1, shift_inhibit: 0,
            steer_angle: 0.0, speed: 0.0, rpm: 0.0, reversing: false, grounded: true, gravity,
        }
    }

    /// `FUN_0018aa88` [M]: |velocity|, then the RPM/gearbox. `driving` = the game's +0x299 byte
    /// (0 = not yet driving → the neutral rev `throttle·7000`).
    pub fn update_rpm_gear(&mut self, throttle: f32, driving: bool) {
        let speed = (self.vel[0] * self.vel[0] + self.vel[1] * self.vel[1] + self.vel[2] * self.vel[2]).sqrt();
        self.speed = speed;
        if !driving {
            self.rpm = throttle * 7000.0; // [M 0x18aa88:32]
            return;
        }
        // The stateless auto walk [M 0x18aa88:36-48].
        let mut gear: u8 = 1;
        let mut limit = self.params.gear_limit[1];
        if limit <= speed {
            loop {
                gear += 1;
                if gear > 4 { break; }
                limit = self.params.gear_limit[gear as usize];
                if speed < limit { break; }
            }
        }
        // The anti-hunt hold [M 0x18aa88:53-56]: a recomputed DOWNshift while the inhibit byte
        // (+0x2d7) stands keeps the last gear. The game sets the inhibit to 0xf on every change
        // [M 0x18cef8:410-413] and decrements it per tick [M 0x18cef8:911-912] — the port keeps
        // that counter and applies the hold only while it stands.
        if gear < self.last_gear && self.shift_inhibit != 0 {
            gear = self.last_gear;
        }
        self.gear = gear;
        // The change detection and the inhibit reload [M 0x18cef8:410-413]; the per-tick
        // decrement rides the same function's tail [M :911-912] — the port decrements on the
        // ticks without a change so the hold lasts 15 ticks after an upshift.
        if gear != self.last_gear {
            self.last_gear = gear;
            self.shift_inhibit = 0xf;
        } else if self.shift_inhibit != 0 {
            self.shift_inhibit -= 1;
        }
        // rpm = |v|·6000/LIMIT[gear] [M 0x18aa88:62] (the walk's last-read limit; gear 5 uses
        // LIMIT4 per the loop's break order).
        let gl = if gear >= 5 { self.params.gear_limit[4] } else { self.params.gear_limit[gear as usize] };
        self.rpm = if gl > 0.0 { speed * 6000.0 / gl } else { 0.0 };
    }

    /// `FUN_0018cef8`'s steering block [M :458-474]: the speed falloff between MODSTEERSPEED and
    /// +25, floored at MINSTEER, scaled by MAXSTEERING.
    pub fn update_steering(&mut self, steer_input: f32) {
        let p = &self.params;
        let scale = if self.speed <= p.modsteer_speed {
            1.0
        } else {
            let t = (self.speed - p.modsteer_speed) / 25.0;
            (1.0 - t.min(1.0)).max(p.min_steer)
        };
        self.steer_angle = steer_input * p.maxsteering * scale;
    }

    /// The engine's drive acceleration [M 0x18cef8:414-431]: MAXACC·RATIO[gear] per unit
    /// throttle, minus brake·MAXBRAKE; reverse decays MAXREVACC by the speed cutoff.
    pub fn drive_accel(&mut self, input: &CarInput) -> f32 {
        let p = &self.params;
        if self.reversing {
            let cut = self.speed - p.speed_cutoff_reverse;
            let rev = if cut > 0.0 { (p.maxrevacc - cut * p.speed_cutoff_rate_reverse).max(0.0) } else { p.maxrevacc };
            // [M 0x18cef8:428-429 — BOOST + throttle·reverse_accel − brake·reverse_accel]
            return input.throttle * rev - input.brake * rev;
        }
        let ratio = p.gear_ratio[self.gear as usize];
        let engine = p.maxacc * ratio; // [M :414-416 — +0x41c · GEAR_RATIO[gear]]
        // [M :420-421 — +0x428(BOOST) + throttle·engine − brake·MAXBRAKE]
        input.throttle * engine - input.brake * p.maxbrake
    }

    /// One physics tick, in the game's order [M 0x1ff3d8 D→E→controller]: integrate the
    /// accumulated acceleration, then reset + gravity + ground damping, then the
    /// engine/steer/tyre pass whose forces land for the NEXT tick — the port folds them into
    /// one explicit sequence per tick (the same values, deterministic order).
    pub fn step(&mut self, input: &CarInput, world: &FlatWorld, rigid: &Rigid, dt: f32) {
        // 1. The RPM/gearbox and the steering (the controller's own state, [M 0x18aa88/0x18cef8]).
        let driving = self.speed > 0.5 || input.throttle > 0.0 || self.reversing;
        self.update_rpm_gear(input.throttle, driving);
        self.update_steering(input.steer);

        // 2. The drive + brake acceleration along the car's heading [M 0x18cef8:420-421].
        let mut accel = self.drive_accel(input);
        let (sin, cos) = self.yaw.sin_cos();
        let fwd = [sin, 0.0, cos];

        // 3. The tyre model [M 0x190f88]: the grip-boost curve and the lateral clamp.
        //    front boost = (1 − min(1, speed/15))·2.5 + 1.0 [M :395-397]; the axle lateral force
        //    clamps at ±12.5 [M :506-508] — the port's bicycle-model yaw rate is limited so the
        //    lateral acceleration stays inside the clamp [S form, M constants].
        let boost = (1.0 - (self.speed / 15.0).min(1.0)) * 2.5 + 1.0;
        let surface_grip = Friction::game_defaults().friction[world.surface.min(12)];
        let lat_limit = 12.5 * boost * surface_grip;
        let wheelbase = self.params.wheel_zr_offset - self.params.wheel_zf_offset;
        let mut yaw_rate = self.speed * self.steer_angle.tan() / wheelbase;
        let lat = (yaw_rate * self.speed).abs();
        if lat > lat_limit && lat > 1e-6 {
            yaw_rate *= lat_limit / lat; // the tyres give up: the clamp wins [M :507-508]
        }
        self.yaw_rate = yaw_rate;

        // 4. Integrate [M 0x1f89e0's VU0 contract, [S] Euler form]: vel += acc·dt; pos += vel·dt;
        //    yaw += yaw_rate·dt. The flat surface holds the ride height (the world-collider
        //    spring keeps y; the port zeroes vy on contact) [S].
        let acc = [fwd[0] * accel, self.gravity, fwd[2] * accel];
        if self.grounded {
            self.vel[0] += acc[0] * dt;
            self.vel[2] += acc[2] * dt;
            // Rolling resistance [M Physical tun ROLLING_RESISTANCE 0x3319a4 = 0.1, registered by
            // FUN_00185668; the tyre pass feeds it into the same accumulators]. The 0x1f7c60
            // whole-velocity ×0.8 damping is NOT applied here: its predicate (instance+0x6c > 2
            // && +0x6d > 3 [M :171-173]) reads as the prop classes, and applied to a car it would
            // cap the speed at ~0.16 m/s — see docs/systems/drive-physics.md §5's open question.
            let speed = (self.vel[0] * self.vel[0] + self.vel[2] * self.vel[2]).sqrt();
            if speed > 1e-4 {
                let roll = (0.1 * dt / speed).min(1.0);
                self.vel[0] -= self.vel[0] * roll;
                self.vel[2] -= self.vel[2] * roll;
            }
            self.vel[1] = 0.0;
        } else {
            self.vel[0] += acc[0] * dt;
            self.vel[1] += acc[1] * dt;
            self.vel[2] += acc[2] * dt;
        }
        self.pos[0] += self.vel[0] * dt;
        self.pos[1] += self.vel[1] * dt;
        self.pos[2] += self.vel[2] * dt;
        self.yaw += self.yaw_rate * dt;
        if self.pos[1] <= 0.0 {
            self.pos[1] = 0.0;
            if self.vel[1] < 0.0 { self.vel[1] = 0.0; }
            self.grounded = true;
        } else {
            self.grounded = false;
        }
    }
}
