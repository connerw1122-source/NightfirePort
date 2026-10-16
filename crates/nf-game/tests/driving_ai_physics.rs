//! The AI brain wired onto 506's vehicle physics (`drive-ai-traffic2`): the drive command the
//! brain picks (FUN_0012e808 target speed [M 0x12e808] → FUN_00133ab0's four vtbl calls
//! [M 0x133ab0:135-180]) becomes 506's `CarInput`, `Car::step` integrates it [M 0x1ff3d8's
//! order], and the pre-integration sim speed drives the spline progress [M 0x222468]. The
//! fixture is the hand-built format-exact `.crp` (the same RNgp walk as driving_ai_follow.rs,
//! trimmed to the two straight segments the wiring tests need). Sister assertions: the replay
//! test proves the driver adds no physics of its own (a bare `Car` fed the recorded commands is
//! bit-identical), and the forced-stop test records the known 506 gap (`Car::step` does not read
//! `handbrake` yet).

use nf_formats::crp::Crp;

use nf_game::driving::ai::{AiDriver, DriveCommand, SimCar, STATE_STANDBY};
use nf_game::driving::{Car, CarInput, CarParams, DT, FlatWorld, Rigid, Tuning};

// ---------------------------------------------------------------- the fixture builder

/// A little-endian writer over a growing buffer.
struct Buf {
    b: Vec<u8>,
}

impl Buf {
    fn new() -> Buf {
        Buf { b: Vec::new() }
    }
    fn off(&self) -> u32 {
        self.b.len() as u32
    }
    fn u32(&mut self, v: u32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn u16(&mut self, v: u16) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn u8(&mut self, v: u8) {
        self.b.push(v);
    }
    fn f32(&mut self, v: f32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn pad(&mut self, n: usize) {
        self.b.extend(std::iter::repeat(0u8).take(n));
    }
    fn bytes(&mut self, at: u32, v: &[u8]) {
        self.b[at as usize..at as usize + v.len()].copy_from_slice(v);
    }
}

/// One 16-byte record: tag word (with the short-tag index in the low half when `short`),
/// flags = size<<8 | skip<<5 | (short?1:0), count, dref.
fn rec(buf: &mut Buf, tag: u32, short: bool, size: usize, skip: usize, count: i32, dref: i32) -> u32 {
    let mut flags = (size as u32) << 8;
    flags |= (skip as u32) << 5;
    if short {
        flags |= 1;
    }
    buf.u32(tag);
    buf.u32(flags);
    buf.u32(count as u32);
    buf.u32(dref as u32);
    buf.off() - 16
}

/// The two-segment straight track: node0 (0,0,0) — node1 (100,0,0) — node2 (200,0,0);
/// seg0 (n0→n1, limit 25) and seg1 (n1→n2, limit 40), both clean, group 0, one route record.
fn build_track() -> Vec<u8> {
    let mut b = Buf::new();
    // root CARP: table at 0x10, pool = 1 record (skip 1), empty window
    rec(&mut b, 0x4341_5250, false, 0, 1, 0, 0x10);
    // pool: the RNgp article; its window = 0x20..
    let rngp_at = rec(&mut b, 0x524e_6770, false, 0, 0, 0, 0x20);
    // the RNgp window: RNhd + rn#0..2 + rs#0..1 + rr#0 (7 records)
    const WIN: i32 = 7;
    b.bytes(rngp_at + 8, &(WIN as u32).to_le_bytes());

    let at_rnhd = rec(&mut b, 0x524e_6864, false, 10, 0, 0, 0);
    let mut rn_at = [0u32; 3];
    for i in 0..3 {
        rn_at[i] = rec(&mut b, 0x726e_0000 | i as u32, true, 0x14, 0, 0, 0);
    }
    let mut rs_at = [0u32; 2];
    for i in 0..2 {
        rs_at[i] = rec(&mut b, 0x7273_0000 | i as u32, true, 0x64, 0, 0, 0);
    }
    let rr_at = rec(&mut b, 0x7272_0000, true, 0x30, 0, 0, 0);

    // RNhd: counts rn=3, rs=2, ri=0, rr=1, rj=0
    let rnhd_payload = b.off();
    b.u16(3);
    b.u16(2);
    b.u16(0);
    b.u16(1);
    b.u16(0);

    let node_payload = |b: &mut Buf, pos: [f32; 3], links: [u16; 2], count: u8| {
        let at = b.off();
        b.f32(pos[0]);
        b.f32(pos[1]);
        b.f32(pos[2]);
        b.pad(2);
        b.u8(count); // link count at +0x0e
        b.pad(1);
        b.u16(links[0]); // +0x10
        b.u16(links[1]);
        at
    };
    let n0 = node_payload(&mut b, [0.0, 0.0, 0.0], [0, 0xffff], 1);
    let n1 = node_payload(&mut b, [100.0, 0.0, 0.0], [1, 0xffff], 1);
    let n2 = node_payload(&mut b, [200.0, 0.0, 0.0], [0xffff, 0xffff], 0);

    let seg_payload = |b: &mut Buf, a: u16, bb: u16, id: u16, limit: f32| {
        let at = b.off();
        b.pad(0x0c);
        b.u16(a);
        b.u16(bb);
        b.pad(0x3e - 0x10);
        b.u16(id); // +0x3e
        b.f32(1.0);
        b.f32(0.0);
        b.f32(0.0); // +0x40 dir A→B (+x)
        b.f32(limit); // +0x4c
        b.u16(0); // +0x50 flags: plain road, clean
        b.u8(1); // +0x52 lanes −
        b.u8(1); // +0x53 lanes +
        b.pad(0x5a - 0x54);
        b.u8(0); // +0x5a car group 0
        b.pad(0x64 - 0x5b);
        at
    };
    let s0 = seg_payload(&mut b, 0, 1, 0, 25.0);
    let s1 = seg_payload(&mut b, 1, 2, 1, 40.0);

    // route: 0x30 B, occupancy at +0x2e/2f
    let rr_payload = b.off();
    b.pad(0x2e);
    b.u8(0);
    b.u8(0);
    b.pad(2);

    // patch the drefs
    b.bytes(at_rnhd + 12, &rnhd_payload.to_le_bytes());
    for (i, at) in [n0, n1, n2].into_iter().enumerate() {
        b.bytes(rn_at[i] + 12, &at.to_le_bytes());
    }
    for (i, at) in [s0, s1].into_iter().enumerate() {
        b.bytes(rs_at[i] + 12, &at.to_le_bytes());
    }
    b.bytes(rr_at + 12, &rr_payload.to_le_bytes());
    b.b
}

fn parse(buf: &[u8]) -> nf_game::driving::road::RoadNet<'_> {
    let crp = Crp::parse(buf).expect("fixture parses as CARP");
    RoadNet::parse_crp(&crp).expect("fixture RNgp parses")
}

use nf_game::driving::road::RoadNet;

/// Bond's shipped pvehicle numbers [D vanquish.atr].
fn vanquish() -> CarParams {
    CarParams::vanquish()
}

// ---------------------------------------------------------------- the tests

#[test]
fn the_506_public_paths_survive_the_module_move() {
    // The layout fix moved 506's `driving.rs` to `driving/physics.rs`; every public path 506's
    // tests and callers use must stay. The imports above ARE the compile-time assertion
    // (`nf_game::driving::{Car, CarInput, CarParams, DT, FlatWorld, Rigid, Tuning}`).
    assert_eq!(DT, 1.0 / 60.0, "the fixed step [M 0x2d6b90's {{60,60,50,60}} table]");
    assert_eq!(Rigid::game_defaults().gravity, -34.0, "Physics:Rigid gravity [M 0x3390d0]");
    assert_eq!(Rigid::game_defaults().bodge_player_gravity, -13.0, "the player bodge [M]");
    assert_eq!(vanquish().mass, 1660.0, "the shipped vanquish mass [D vanquish.atr]");
    assert_eq!(vanquish().maxacc, 3.0, "the shipped maxacc [D vanquish.atr]");
    // The .tun database still parses through the re-exported path.
    let t = Tuning::parse("ROLLING_RESISTANCE=0.1\r\n");
    assert_eq!(t.get("ROLLING_RESISTANCE"), Some(0.1), "the .tun lookup [M 0x1426a8]");
}

#[test]
fn the_sim_view_mirrors_the_physics_car() {
    let mut car = Car::new(vanquish(), [3.0, 0.0, 4.0], 0.5);
    car.vel = [1.0, 0.0, 2.0];
    let sim = SimCar::of_car(&car);
    assert_eq!(sim.pos, car.pos, "pos = instance+0x10 [M 0x133ab0:41]");
    assert_eq!(sim.vel, car.vel, "vel = instance+0x20 [M 0x133ab0:103-113]");
    let (s, c) = 0.5f32.sin_cos();
    assert_eq!(sim.forward, [s, 0.0, c], "the forward row is the yaw's [sin, 0, cos] [M]");
    let len = (sim.forward[0] * sim.forward[0] + sim.forward[2] * sim.forward[2]).sqrt();
    assert!((len - 1.0).abs() < 1e-6, "the facing the dots multiply is unit [M]");
    assert_eq!(sim.alive, 1.0, "a live actor [S — FUN_00195a30 is the actor layer]");
}

#[test]
fn the_wired_driver_accelerates_on_506s_model() {
    let buf = build_track();
    let net = parse(&buf);
    let mut driver = AiDriver::on_segment(vanquish(), &net, 0, 0, 0).unwrap();
    let start_pos = driver.car.pos;
    let start_yaw = driver.car.yaw;

    // Tick 1: target 25 (the segment limit [M 0x12e808:165]), speed 0 → the ramp saturates
    // [M 0x133ab0:145-147] and lands on the throttle pedal; the aim on the nose → steer 1.0.
    let first = driver.step(&net, DT).unwrap();
    assert_eq!(first.throttle, 1.0, "Δ=25 > 7.5 → full throttle [M 0x133ab0]");
    assert_eq!(first.brake, 0.0, "no brake at the start");
    assert!(!first.handbrake, "no handbrake at the start");
    assert_eq!(first.steer, 1.0, "the aim on the nose → the dot is 1 [M 0x133ab0:135-137]");

    // 300 ticks = 5 s of clean throttle (still on seg0, outside the arrival radius). Record the
    // commands and the rail snaps for the replay.
    let mut tape: Vec<(DriveCommand, [f32; 3], f32)> = vec![(first, driver.car.pos, driver.car.yaw)];
    for _ in 0..299 {
        let c = driver.step(&net, DT).unwrap();
        tape.push((c, driver.car.pos, driver.car.yaw));
    }

    // The speed grew on 506's model: the drive accel is MAXACC·RATIO[gear]·throttle
    // [M 0x18cef8:414-421] → vanquish gear 1: 3.0·0.65 = 1.95/s at the pedal 1.0.
    assert!(driver.car.speed > 5.0, "the car accelerated, speed = {}", driver.car.speed);
    assert!((1..=5).contains(&driver.car.gear), "the gearbox ran [M 0x18aa88]");
    // The car is on the spline [M 0x222468] pointed down the road.
    let exact = net.seg_point(driver.brain.segment, driver.brain.t).unwrap();
    assert_eq!(driver.car.pos, exact, "the wired car rides the spline");
    let (s, c) = driver.car.yaw.sin_cos();
    assert!((s - 1.0).abs() < 1e-6 && c.abs() < 1e-6, "yaw faces +x (atan2(dir.x, dir.z))");

    // Sister: the driver adds NO physics of its own — a bare Car fed the recorded commands
    // (and the recorded rail snaps) must be bit-identical after the same 300 ticks.
    let mut replay = Car::new(vanquish(), start_pos, start_yaw);
    let world = FlatWorld::paved();
    let rigid = Rigid::game_defaults();
    for (c, pos, yaw) in &tape {
        let input = CarInput {
            throttle: c.throttle,
            brake: c.brake,
            steer: c.steer,
            handbrake: c.handbrake,
        };
        replay.step(&input, &world, &rigid, DT);
        replay.pos = *pos;
        replay.yaw = *yaw;
    }
    assert_eq!(replay.vel, driver.car.vel, "the replay's velocity is bit-identical");
    assert_eq!(replay.speed, driver.car.speed, "so is the recomputed speed");
    assert_eq!(replay.gear, driver.car.gear, "the gearbox states match");
    assert_eq!(replay.steer_angle, driver.car.steer_angle, "the steering state matches");
}

#[test]
fn the_steer_command_drives_506s_steering_angle() {
    let buf = build_track();
    let net = parse(&buf);
    let mut driver = AiDriver::on_segment(vanquish(), &net, 0, 0, 0).unwrap();

    // At rest: the steer dot is 1.0 [M 0x133ab0:135-137], the falloff scale is 1.0 while
    // speed ≤ MODSTEERSPEED [M 0x18cef8:458-474] → steer_angle = MAXSTEERING exactly.
    driver.step(&net, DT).unwrap();
    assert_eq!(driver.car.steer_angle, vanquish().maxsteering,
        "steer 1.0 · MAXSTEERING 0.09 · scale 1.0 [M 0x18cef8:458-474]");

    // At speed 20 (> MODSTEERSPEED 10): scale = 1 − (20−10)/25 = 0.6.
    driver.car.vel = [20.0, 0.0, 0.0];
    driver.step(&net, DT).unwrap();
    let p = vanquish();
    let scale = 1.0 - (20.0 - p.modsteer_speed) / 25.0;
    assert_eq!(driver.car.steer_angle, p.maxsteering * scale,
        "the speed falloff scale [M 0x18cef8:458-474]");
}

#[test]
fn the_brake_ramp_rests_the_car_at_the_aimed_node() {
    // A no-exit variant: node1's link count patched to 0 [M 0x2247b0 → None]. The car
    // accelerates, enters the arrival radius [M 0x133ab0:114-130], brakes (the ramp
    // [M :145-147]) and rests at the node — the brake never backs it up [S: reverse is the
    // R-gear throttle path; at rest the AI's own speed ≤ 0 early-out [M 0x133ab0:153] stops
    // commanding brake].
    let mut b2 = build_track();
    // Locate node1's payload by its position triple (100,0,0) and zero the count byte at +0x0e.
    let needle = 100.0f32.to_le_bytes();
    let mut at = None;
    for i in 0..b2.len() - 12 {
        if b2[i..i + 4] == needle && b2[i + 4..i + 12] == [0u8; 8] {
            at = Some(i);
            break;
        }
    }
    let n1 = at.expect("node1 payload found");
    b2[n1 + 0x0e] = 0;

    let net = parse(&b2);
    assert_eq!(net.next_segment(0, 1, 0).unwrap(), None, "the severed exit");
    let mut driver = AiDriver::on_segment(vanquish(), &net, 0, 0, 0).unwrap();

    let mut saw_brake = false;
    let mut max_speed_after_braking = 0.0f32;
    for _ in 0..2400 {
        let cmd = driver.step(&net, DT).unwrap();
        if cmd.brake > 0.0 {
            saw_brake = true;
            max_speed_after_braking = max_speed_after_braking.max(driver.car.speed);
        }
    }
    assert!(saw_brake, "the car braked on arrival [M 0x133ab0:114-130,167-171]");
    assert!(max_speed_after_braking > 5.0, "it was moving when the brakes came on");
    assert_eq!(driver.brain.t, 1.0, "the car holds at the node (no exit)");
    assert!(driver.car.speed < 0.1,
        "the brake ramp rests the car, speed = {} (no reverse runaway [S])", driver.car.speed);
    assert!(driver.car.vel[0] >= 0.0, "the velocity never points backwards");
}

#[test]
fn the_forced_stop_handbrakes_and_the_speed_holds_until_506s_handbrake_lands() {
    let buf = build_track();
    let net = parse(&buf);
    let mut driver = AiDriver::on_segment(vanquish(), &net, 0, 0, 0).unwrap();
    // Get moving.
    for _ in 0..120 {
        driver.step(&net, DT).unwrap();
    }
    assert!(driver.car.speed > 1.0, "rolling before the stop");
    let speed_at_stop = driver.car.speed;

    driver.brain.forced_stop = true; // the AI's +0x19c [M 0x133ab0:144]
    let cmd = driver.step(&net, DT).unwrap();
    assert_eq!(cmd.throttle, 0.0, "throttle 0 [M 0x133ab0:173-180]");
    assert_eq!(cmd.brake, 0.0, "brake 0 [M 0x133ab0:173-180]");
    assert!(cmd.handbrake, "handbrake 1 [M 0x133ab0:173-180]");
    assert_eq!(cmd.steer, 1.0, "the alignment dot still steers [M :135-137]");

    // 60 ticks of handbrake: 506's `Car::step` does not read `handbrake` yet (506's report lists
    // it as its own work) → the speed decays by the rolling resistance only [M 0x3319a4 = 0.1/s],
    // never by a handbrake force. Sister: this assertion FAILS the day 506's handbrake lands —
    // then update it to assert the decel.
    for _ in 0..60 {
        let c = driver.step(&net, DT).unwrap();
        assert!(c.handbrake, "the forced stop keeps handbraking");
    }
    assert!(driver.car.speed > speed_at_stop * 0.7,
        "the speed holds (only the 0.1/s rolling decay [M 0x3319a4]): {} → {}",
        speed_at_stop, driver.car.speed);
    assert!(driver.car.speed < speed_at_stop, "not frozen either — the rolling decay shows");
}

#[test]
fn the_crossing_takes_the_clean_link_at_the_physics_speed() {
    let buf = build_track();
    let net = parse(&buf);
    let mut driver = AiDriver::on_segment(vanquish(), &net, 0, 0, 0).unwrap();

    // ~15 s: throttle up seg0 (limit 25), cross at node1 onto seg1 (limit 40) — the crossing
    // uses the pre-integration sim speed [M 0x133ab0:103-113] in the progress walk.
    let mut crossed = false;
    for _ in 0..900 {
        driver.step(&net, DT).unwrap();
        if driver.brain.segment == 1 {
            crossed = true;
            break;
        }
    }
    assert!(crossed, "the car crossed onto seg1 within 15 s");
    assert_eq!(driver.brain.end, 0, "seg1 leaves n1 through its A side → A→B [M 0x225c48]");
    let t = driver.brain.t;
    assert!(t > 0.0 && t < 1.0, "mid-segment on seg1, t = {}", t);
    let exact = net.seg_point(1, t).unwrap();
    assert_eq!(driver.car.pos, exact, "the wired car rides seg1's spline [M 0x222468]");
    assert!(driver.car.pos[0] > 100.0, "past node1: x = {}", driver.car.pos[0]);
    // On seg1 the limit is 40 (rs+0x4c [M 0x12e808:165]) → the throttle comes back.
    let cmd = driver.step(&net, DT).unwrap();
    assert!(cmd.throttle > 0.0, "seg1's limit 40 > speed → throttle [M 0x133ab0:148-151]");
    assert!(driver.car.speed > 10.0, "still rolling at the physics speed");
}

#[test]
fn the_standby_brain_does_not_move_the_physics_car() {
    let buf = build_track();
    let net = parse(&buf);
    let mut driver = AiDriver::on_segment(vanquish(), &net, 0, 0, 0).unwrap();
    driver.brain.state = STATE_STANDBY; // [M 0x12e350's case 1]
    for _ in 0..60 {
        let cmd = driver.step(&net, DT).unwrap();
        assert_eq!(cmd.throttle, 0.0, "state 1 commands nothing");
    }
    assert_eq!(driver.car.speed, 0.0, "the physics car never integrated a tick");
    assert_eq!(driver.brain.t, 0.0, "no progress in standby");
}
