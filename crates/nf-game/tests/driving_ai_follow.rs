//! The driving AI's headless follow tests: an AI car entering the road network follows its spline
//! at the game's speed numbers — the target speed from the segment's limit (rs+0x4c
//! [M FUN_0012e808 0x12e808:165]), the drive-control ramps [M FUN_00133ab0 0x133ab0], the position
//! along the segment [M FUN_00222468 0x222468] and the next-segment filters [M FUN_002247b0
//! 0x2247b0]. The fixture is a hand-built format-exact `.crp` (the RNgp article with RNhd +
//! rn/rs/rr short-tag records exactly as FUN_00220c28 0x220c28 reads them). Sister assertions:
//! the malformed-fixture test proves the parse rejects what the format forbids, and the gated
//! test runs the same walk against the real `MIS01.VIV` track when `NF_DRIVING` points at the
//! extracted DRIVING folder (the slim ISO zeroes the driving archives' bytes).

use nf_formats::bigf::Bigf;
use nf_formats::crp::Crp;
use nf_formats::refpack;

use nf_game::driving::ai::{
    drive_control, target_speed_traffic, AiCar, SimCar, STATE_CHASE, STATE_TRAFFIC,
};
use nf_game::driving::road::{RoadNet, RSF_TURN_RESTR_A};

// ---------------------------------------------------------------- the fixture builder

/// A little-endian u32 writer over a growing buffer.
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
    fn u32(&mut self, v: u32) -> u32 {
        let o = self.off();
        self.b.extend_from_slice(&v.to_le_bytes());
        o
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
/// flags = size<<8 | skip<<5 | (short?1:0) | (relative?2:0), count, dref.
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

/// The synthetic track: node0 (0,0,0) — node1 (100,0,0) — node2 (200,0,0), straight segments
/// seg0 (n0→n1, limit 25), seg1 (n1→n2, limit 40); seg2 (n0→n2, limit 90) is node0's second link,
/// turn-restricted (flags 0x200) so the walk must skip it; seg3 (n1→n0 back-link) carries a
/// different car group. Returns the buffer.
fn build_track() -> Vec<u8> {
    let mut b = Buf::new();
    // root CARP: table at 0x10, pool = 1 record (skip 1), empty window
    rec(&mut b, 0x4341_5250, false, 0, 1, 0, 0x10);

    // pool: the RNgp article; its window = 0x20.., count filled below
    let rngp_at = rec(&mut b, 0x524e_6770, false, 0, 0, 0, 0x20);

    // the RNgp window: RNhd + rn#0..2 + rs#0..3 + rr#0 (12 records)
    const WIN: i32 = 12;
    b.bytes(rngp_at + 8, &(WIN as u32).to_le_bytes());

    // --- window records (drefs patched after the payloads are placed)
    let at_rnhd = rec(&mut b, 0x524e_6864, false, 10, 0, 0, 0);
    let mut rn_at = [0u32; 3];
    let mut rs_at = [0u32; 4];
    for i in 0..3 {
        rn_at[i] = rec(&mut b, 0x726e_0000 | i as u32, true, 0x14, 0, 0, 0);
    }
    for i in 0..4 {
        rs_at[i] = rec(&mut b, 0x7273_0000 | i as u32, true, 0x64, 0, 0, 0);
    }
    let rr_at = rec(&mut b, 0x7272_0000, true, 0x30, 0, 0, 0);

    // --- payloads
    // RNhd: counts rn=3, rs=4, ri=0, rr=1, rj=0
    let rnhd_payload = b.off();
    b.u16(3);
    b.u16(4);
    b.u16(0);
    b.u16(1);
    b.u16(0);

    // nodes: n0 (0,0,0) links [seg0, seg2]; n1 (100,0,0) links [seg1, seg3]; n2 (200,0,0) links []
    let node_payload = |b: &mut Buf, pos: [f32; 3], links: [u16; 2]| {
        let at = b.off();
        b.f32(pos[0]);
        b.f32(pos[1]);
        b.f32(pos[2]);
        b.pad(2);
        b.u8(2); // link count at +0x0e
        b.pad(1);
        b.u16(links[0]); // +0x10
        b.u16(links[1]);
        (at, links)
    };
    let n0 = node_payload(&mut b, [0.0, 0.0, 0.0], [0, 2]);
    let n1 = node_payload(&mut b, [100.0, 0.0, 0.0], [1, 3]);
    let n2 = node_payload(&mut b, [200.0, 0.0, 0.0], [0xffff, 0xffff]);

    // segments: 0x64-byte payloads. off: a at +0x0c, b at +0x0e, id at +0x3e, dir at +0x40,
    // limit at +0x4c, flags at +0x50, lanes at +0x52/53, group at +0x5a.
    let seg_payload = |b: &mut Buf, a: u16, bb: u16, id: u16, limit: f32, flags: u16, group: i8| {
        let at = b.off();
        b.pad(0x0c);
        b.u16(a);
        b.u16(bb);
        b.pad(0x3e - 0x10);
        b.u16(id); // +0x3e
        b.pad(0x40 - 0x40);
        b.f32(1.0);
        b.f32(0.0);
        b.f32(0.0); // +0x40 dir A→B (unit +x for all fixtures)
        b.f32(limit); // +0x4c
        b.u16(flags); // +0x50
        b.u8(1); // +0x52 lanes −
        b.u8(1); // +0x53 lanes +
        b.pad(0x5a - 0x54);
        b.u8(group as u8); // +0x5a
        b.pad(0x64 - 0x5b);
        at
    };
    let s0 = seg_payload(&mut b, 0, 1, 0, 25.0, 0, 0);
    let s1 = seg_payload(&mut b, 1, 2, 1, 40.0, 0, 0);
    let s2 = seg_payload(&mut b, 0, 2, 2, 90.0, RSF_TURN_RESTR_A, 0);
    let s3 = seg_payload(&mut b, 1, 0, 3, 30.0, 0, 7); // group 7 ≠ the car's group 0

    // route: 0x30 B, occupancy at +0x2e/2f
    let rr_payload = b.off();
    b.pad(0x2e);
    b.u8(0);
    b.u8(0);
    b.pad(2);

    // --- patch the drefs
    b.bytes(at_rnhd + 12, &rnhd_payload.to_le_bytes());
    for i in 0..3 {
        let (at, _) = [n0, n1, n2][i];
        b.bytes(rn_at[i] + 12, &at.to_le_bytes());
    }
    for i in 0..4 {
        let at = [s0, s1, s2, s3][i];
        b.bytes(rs_at[i] + 12, &at.to_le_bytes());
    }
    b.bytes(rr_at + 12, &rr_payload.to_le_bytes());
    b.b
}

fn parse(buf: &[u8]) -> RoadNet<'_> {
    let crp = Crp::parse(buf).expect("fixture parses as CARP");
    RoadNet::parse_crp(&crp).expect("fixture RNgp parses")
}

/// A car sitting on the segment start at `speed`, 60 Hz.
fn sim_at(pos: [f32; 3], speed: f32) -> SimCar {
    SimCar { pos, vel: [speed, 0.0, 0.0], forward: [1.0, 0.0, 0.0], alive: 1.0 }
}

const DT: f32 = 1.0 / 60.0;

// ---------------------------------------------------------------- the tests

#[test]
fn road_net_parses_the_fixture_counts_and_fields() {
    let buf = build_track();
    let net = parse(&buf);
    assert_eq!(net.nodes.len(), 3, "rn count");
    assert_eq!(net.segs.len(), 4, "rs count");
    assert_eq!(net.routes.len(), 1, "rr count");
    assert!(net.ri.is_empty() && net.rj.is_empty(), "ri/rj counts");
    assert!(net.has_plain_road, "seg0 has (flags & 0x8108) == 0 [M 0x220c28:115-116]");

    let n0 = net.node(0).unwrap();
    assert_eq!(n0.pos(), [0.0, 0.0, 0.0], "node position at +0x00 [M 0x225c48:179-186]");
    assert_eq!(n0.link_count(), 2, "link count at +0x0e [M 0x2247b0:99]");
    assert_eq!(n0.link(0), Some(0), "links at +0x10 [M 0x2247b0:153,173]");
    let s0 = net.seg(0).unwrap();
    assert_eq!((s0.node_a(), s0.node_b()), (0, 1), "segment endpoints at +0x0c/+0x0e [M 0x2215f8:7-8]");
    assert_eq!(s0.speed_limit(), 25.0, "speed limit at +0x4c [M 0x12e808:165]");
    assert_eq!(s0.flags(), 0, "flags at +0x50");
    assert_eq!((s0.lanes_neg(), s0.lanes_pos()), (1, 1), "lanes at +0x52/53 [M 0x221f20:28-29]");
    assert_eq!(net.seg(3).unwrap().car_group(), 7, "car group at +0x5a [M 0x225490:133]");
}

#[test]
fn ai_car_follows_the_spline_at_the_segment_speed_limit() {
    let buf = build_track();
    let mut net = parse(&buf);
    // The game units are the game's; the fixture's seg0 is 100 units long with the limit 25.
    let mut car = AiCar::on_segment(0, 0, 0);
    let mut sim = sim_at([0.0, 0.0, 0.0], 25.0);

    let target = car.select_target_speed(&net, &sim).unwrap();
    assert_eq!(target, 25.0, "the target speed IS the segment limit rs+0x4c [M 0x12e808:165]");

    // 60 steps = 1 s at 60 Hz [M docs/atlas/driving-elf.md §1.3] → 25 units along +x.
    for _ in 0..60 {
        car.step(&mut net, &mut sim, DT).unwrap();
    }
    assert_eq!(car.segment, 0, "still on seg0 after 25 of 100 units");
    let p = sim.pos;
    assert!((p[0] - 25.0).abs() < 1e-3, "pos.x = 25 after 1 s at 25/s, got {}", p[0]);
    assert!(p[1].abs() < 1e-6 && p[2].abs() < 1e-6, "dead on the spline line [M 0x222468]");

    // 240 more steps = 4 s → 100 more units → seg0 ends, the walk takes seg1 (the clean link)
    // and the position continues along it [M 0x2247b0].
    for _ in 0..240 {
        car.step(&mut net, &mut sim, DT).unwrap();
    }
    assert_eq!(car.segment, 1, "crossed at the shared node onto seg1");
    assert_eq!(car.end, 0, "seg1 leaves n1 through its A side → driven A→B [M 0x225c48]");
    let p = sim.pos;
    assert!((p[0] - 125.0).abs() < 5e-2, "pos.x = 125 after 5 s total, got {}", p[0]);

    // The position is exactly A + (B−A)·t [M 0x222468:12-20].
    let t = car.t;
    let exact = net.seg_point(car.segment, t).unwrap();
    assert_eq!(sim.pos, exact, "the sim position is the segment evaluation");
}

#[test]
fn ai_car_stops_inside_the_arrival_radius() {
    let buf = build_track();
    let net = parse(&buf);
    // speed 20 → radius = max(10, 5) = 10; aim 8 units ahead → target forced 0 [M 0x133ab0:114-130]
    let sim = sim_at([92.0, 0.0, 0.0], 20.0);
    let aim = [100.0, 0.0, 0.0];
    let cmd = drive_control(&sim, aim, 25.0, false);
    assert_eq!(cmd.throttle, 0.0, "no throttle inside the arrival radius");
    assert!(cmd.brake > 0.0, "braking to the stop: ramp = min(20,7.5)/7.5 = 1.0, got {}", cmd.brake);
    assert_eq!(cmd.brake, 1.0, "the full-stop ramp saturates at 1.0 [M 0x133ab0:145-147]");

    // outside the radius: the same target gives throttle [M 0x133ab0:148-151]
    let sim = sim_at([80.0, 0.0, 0.0], 20.0);
    let cmd = drive_control(&sim, aim, 25.0, false);
    assert!(cmd.throttle > 0.0 && cmd.brake == 0.0, "throttle when the aim is far");
}

#[test]
fn drive_control_matches_the_fun_00133ab0_numbers() {
    // ramp = min(|Δ|, 7.5)/7.5, floor 0.05 [M 0x133ab0:145-147,170-171]
    let sim = sim_at([0.0, 0.0, 0.0], 20.0);
    let cmd = drive_control(&sim, [100.0, 0.0, 0.0], 30.0, false); // Δ=10 → saturate
    assert_eq!(cmd.throttle, 1.0, "Δ=10 > 7.5 → full throttle");
    let cmd = drive_control(&sim, [100.0, 0.0, 0.0], 25.0, false); // Δ=5 → 5/7.5
    assert!((cmd.throttle - 5.0 / 7.5).abs() < 1e-6, "Δ=5 → 5/7.5, got {}", cmd.throttle);
    let slow = sim_at([0.0, 0.0, 0.0], 20.0);
    let cmd = drive_control(&slow, [100.0, 0.0, 0.0], 20.04, false); // Δ=0.04 → the 0.05 floor
    assert_eq!(cmd.throttle, 0.05, "the 0.05 pedal floor [M 0x133ab0:170-171]");

    // the target floor 0.5 [M 0x133ab0:100-102]: target 0 → target' = 0.5, speed 20 → the brake
    // path with Δ = 19.5 → the span saturates → brake 1.0
    let cmd = drive_control(&sim, [100.0, 0.0, 0.0], 0.0, false);
    assert_eq!(cmd.brake, 1.0, "target 0 → target' floor 0.5 → braking hard, got {:?}", cmd);

    // the brake-side early-outs [M 0x133ab0:153-168]: the span min(Δ,7.5) ≤ 1.0 with target ≥ 2.5
    let cmd = drive_control(&sim, [100.0, 0.0, 0.0], 19.5, false); // Δ=0.5, target 19.5 ≥ 2.5
    assert_eq!((cmd.throttle, cmd.brake), (0.0, 0.0), "no brake for a small overshoot");
    let cmd = drive_control(&sim, [100.0, 0.0, 0.0], 15.0, false); // Δ=5 > 1.0 → brake 5/7.5
    assert!((cmd.brake - 5.0 / 7.5).abs() < 1e-6, "real overshoot brakes, got {}", cmd.brake);

    // the forced stop [M 0x133ab0:173-180]
    let cmd = drive_control(&sim, [100.0, 0.0, 0.0], 30.0, true);
    assert_eq!(cmd, nf_game::driving::ai::DriveCommand {
        steer: 1.0,
        throttle: 0.0,
        brake: 0.0,
        handbrake: true
    }, "+0x19c → handbrake");

    // the steer clamp ±1 [M 0x133ab0:135-137]
    let off = sim_at([0.0, 0.0, 0.0], 0.0);
    let cmd = drive_control(&off, [0.0, 0.0, 100.0], 10.0, false);
    assert_eq!(cmd.steer, 0.0, "the aim perpendicular to the facing → the dot is 0");
}

#[test]
fn target_speed_gates_zero_it_like_fun_0012e808() {
    let buf = build_track();
    let net = parse(&buf);
    let limit = net.seg(0).unwrap().speed_limit();
    let alive = sim_at([0.0, 0.0, 0.0], 0.0);
    assert_eq!(target_speed_traffic(STATE_TRAFFIC, &alive, limit, 0, 0), 25.0);

    let dead = SimCar { alive: 0.4, ..alive };
    assert_eq!(target_speed_traffic(STATE_TRAFFIC, &dead, limit, 0, 0), 0.0,
        "state 4 needs alive ≥ 0.5 [M 0x12e808:20-29]");
    let dead5 = SimCar { alive: 0.0, ..alive };
    assert_eq!(target_speed_traffic(STATE_CHASE, &dead5, limit, 0, 0), 0.0,
        "state 5 needs alive > 0 [M 0x12e808:12-18]");
    assert_eq!(target_speed_traffic(STATE_TRAFFIC, &alive, limit, 1, 0), 0.0,
        "the vtbl+0x144 stop query ≠ 0 → 0 [M 0x12e808:41-45]");
    assert_eq!(target_speed_traffic(STATE_TRAFFIC, &alive, limit, 0, 1), 0.0,
        "the +0x170 byte ≠ 0 → 0 [M 0x12e808:48-52]");
}

#[test]
fn next_segment_walks_and_filters_like_fun_002247b0() {
    let buf = build_track();
    let net = parse(&buf);
    // From seg0 through node 1: the links are seg1 (clean) and seg3 (group 7) → seg1 wins.
    let next = net.next_segment(0, 1, 0).unwrap();
    assert_eq!(next, Some((1, true)), "the clean link, driven forward");

    // From seg0 through node 0: the links are seg2 (turn-restricted 0x200) → nothing clean.
    let next = net.next_segment(0, 0, 0).unwrap();
    assert_eq!(next, None, "the 0x200 filter excludes seg2 [M 0x2247b0:115-118]");

    // A car of group 7 from seg0 through node 1 takes seg3 (the group filter [M 0x225490:133]).
    let next = net.next_segment(0, 1, 7).unwrap();
    assert_eq!(next, Some((3, true)), "the car-group filter picks the group's segment");
}

#[test]
fn malformed_rNgp_fixtures_are_rejected() {
    // Sister assertions: the parse must fail on what FUN_00220c28 would leave uninitialized
    // (a missing record) or double-define (a duplicate index).
    let mut buf = build_track();
    // Rename rn#2 → rn#9 (the tag word's low half IS the instance index [M 0x220c28:99-101]):
    // RNhd still says 3 nodes → the #2 lookup must fail. rn#2's record sits at 0x20 + 3*0x10.
    buf[0x20 + 3 * 0x10] = 9;
    let crp = Crp::parse(&buf).unwrap();
    assert!(RoadNet::parse_crp(&crp).is_err(), "a missing rn#2 must be rejected");

    let mut buf = build_track();
    // Point the RNhd payload dref out of range: the counts become unreadable.
    let dref_at = 0x20 + 12; // the RNhd record's dref field
    buf[dref_at..dref_at + 4].copy_from_slice(&0x7fff_ffffu32.to_le_bytes());
    let crp = Crp::parse(&buf).unwrap();
    assert!(RoadNet::parse_crp(&crp).is_err(), "a broken RNhd must be rejected");
}

#[test]
fn curve_segment_evaluates_through_the_control_point() {
    // A curved seg (flags & 0x4000 [M 0x222468:11 → FUN_00222d40]) with the control at (50,30,0):
    // the path bows toward +y and the endpoints stay pinned to the nodes.
    let mut b = Buf::new();
    rec(&mut b, 0x4341_5250, false, 0, 1, 0, 0x10);
    let rngp_at = rec(&mut b, 0x524e_6770, false, 0, 0, 0, 0x20);
    const WIN: i32 = 7;
    b.bytes(rngp_at + 8, &(WIN as u32).to_le_bytes());
    let at_rnhd = rec(&mut b, 0x524e_6864, false, 10, 0, 0, 0);
    let rn0 = rec(&mut b, 0x726e_0000, true, 0x14, 0, 0, 0);
    let rn1 = rec(&mut b, 0x726e_0001, true, 0x14, 0, 0, 0);
    let rs0 = rec(&mut b, 0x7273_0000, true, 0x64, 0, 0, 0);
    let rr0 = rec(&mut b, 0x7272_0000, true, 0x30, 0, 0, 0);

    let rnhd_payload = b.off();
    b.u16(2);
    b.u16(1);
    b.u16(0);
    b.u16(1);
    b.u16(0);

    let n0p = b.off();
    b.f32(0.0);
    b.f32(0.0);
    b.f32(0.0);
    b.pad(2);
    b.u8(1);
    b.pad(1);
    b.u16(0);
    b.u16(0xffff);
    let n1p = b.off();
    b.f32(100.0);
    b.f32(0.0);
    b.f32(0.0);
    b.pad(2);
    b.u8(1);
    b.pad(1);
    b.u16(0);
    b.u16(0xffff);

    let s0p = b.off();
    b.pad(0x0c);
    b.u16(0);
    b.u16(1);
    b.pad(0x3e - 0x10);
    b.u16(0);
    b.f32(1.0);
    b.f32(0.0);
    b.f32(0.0);
    b.f32(40.0); // +0x4c limit
    b.u16(0x4000); // +0x50 curved
    b.u8(1);
    b.u8(1);
    b.pad(0x5a - 0x54);
    b.u8(0);
    b.pad(0x64 - 0x5b);
    // +0x10 control point (50, 30, 0) — patched right after (it is inside the payload)
    let ctrl_at = s0p + 0x10;
    let rrp = b.off();
    b.pad(0x30);

    b.bytes(at_rnhd + 12, &rnhd_payload.to_le_bytes());
    b.bytes(rn0 + 12, &n0p.to_le_bytes());
    b.bytes(rn1 + 12, &n1p.to_le_bytes());
    b.bytes(rs0 + 12, &s0p.to_le_bytes());
    b.bytes(rr0 + 12, &rrp.to_le_bytes());
    let mut ctrl = Vec::new();
    ctrl.extend_from_slice(&50.0f32.to_le_bytes());
    ctrl.extend_from_slice(&30.0f32.to_le_bytes());
    ctrl.extend_from_slice(&0.0f32.to_le_bytes());
    b.bytes(ctrl_at, &ctrl);

    let net = parse(&b.b);
    let mid = net.curve_point(0, 0.5).unwrap();
    assert!(mid[1] > 5.0, "the path bows toward the control point, got y={}", mid[1]);
    let start = net.curve_point(0, 0.0).unwrap();
    assert_eq!(start, [0.0, 0.0, 0.0], "t=0 is node A");
    let end = net.curve_point(0, 1.0).unwrap();
    assert_eq!(end, [100.0, 0.0, 0.0], "t=1 is node B");
}

// ------------------------------------------------- the real-data gate (NF_DRIVING)

fn driving_dir() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("NF_DRIVING") {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() {
            return Some(p);
        }
        eprintln!("skip: NF_DRIVING={p:?} is not a directory");
        return None;
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = manifest.join("../../DRIVING");
    if p.is_dir() {
        return Some(p);
    }
    eprintln!("skip: no DRIVING/ folder (set NF_DRIVING)");
    None
}

#[test]
fn real_mis01_track_rngp_walks_when_nf_driving_is_set() {
    let Some(dir) = driving_dir() else { return };
    let viv = std::fs::read(dir.join("MIS01.VIV")).expect("MIS01.VIV readable");
    let bigf = Bigf::parse(&viv, viv.len() as u64).expect("MIS01.VIV is a BIGF");
    // The track .crp is entry #4 in each mission archive [D docs/formats/crp.md §1].
    let entry = bigf
        .entries
        .get(4)
        .expect("MIS01.VIV entry #4 exists");
    let blob = &viv[entry.offset as usize..entry.offset as usize + entry.size as usize];
    let image = if refpack::is_refpack(blob) {
        refpack::decompress(blob).expect("the track crp RefPack-decompresses")
    } else {
        blob.to_vec()
    };
    let crp = Crp::parse(&image).expect("paris_mis01.crp parses");
    let net = RoadNet::parse_crp(&crp).expect("the paris RNgp parses");

    // The counts are the game's own array lengths after FUN_00220c28 (numbers only).
    assert!(net.nodes.len() > 0, "paris has road nodes");
    assert!(net.segs.len() > 0, "paris has road segments");
    assert!(net.segs.len() >= net.nodes.len() / 2, "segments join nodes: {} segs vs {} nodes",
        net.segs.len(), net.nodes.len());

    // Every segment's endpoints index real nodes, its direction is unit-length, and its limit
    // is a positive number [M 0x2216b0 (unit dir), 0x12e808:165 (the limit the AI reads)].
    let mut unit_dirs = 0usize;
    let mut positive_limits = 0usize;
    for s in &net.segs {
        assert!((s.node_a() as usize) < net.nodes.len(), "node A in range");
        assert!((s.node_b() as usize) < net.nodes.len(), "node B in range");
        let d = s.dir();
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if (len - 1.0).abs() < 1e-3 {
            unit_dirs += 1;
        }
        if s.speed_limit() > 0.0 {
            positive_limits += 1;
        }
    }
    assert!(unit_dirs * 2 > net.segs.len(), "most directions are unit: {unit_dirs}/{}", net.segs.len());
    assert!(positive_limits * 9 > net.segs.len() * 8, "most limits are positive: {positive_limits}/{}", net.segs.len());

    // An AI car walks the first segment's spline exactly [M 0x222468].
    let mut net = net;
    let mut car = AiCar::on_segment(0, 0, net.seg(0).unwrap().car_group());
    let mut sim = sim_at(net.node(net.seg(0).unwrap().node_a() as usize).unwrap().pos(), 10.0);
    for _ in 0..30 {
        car.step(&mut net, &mut sim, DT).unwrap();
    }
    let exact = net.seg_point(car.segment, car.t).unwrap();
    assert_eq!(sim.pos, exact, "the headless walk stays on the real spline");
}
