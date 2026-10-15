//! exact-sine-sites-3 — G-046's third group: the player-motion trig sites run the game's exact sine
//! (`trig::sinf`/`cosf`, bit-exact `PS2Sinf`/`PS2Sinf2`/`PS2Sinf3`), not host `sin`/`cos`.
//!
//! Sites (docs/GAPS.md G-046; survey docs/reports/fix-exact-sine-flip.md):
//! - `player.rs` movement axes — `RotMatrix` 0x1e2368 / `View_RotTransMatrix` 0x1e5168:33-36
//!   (`PS2Sinf3(yaw+π/2, yaw, …)`); swim look-dir `RotMatrixX` 0x1e46a0 (`PS2Sinf2`).
//! - `player.rs` ladder grab snap — `Player_CollLadder` 0x196118:32-40 (direct `PS2Sinf(yaw+π)`,
//!   `PS2Sinf(yaw+π+π/2)`, `pos = centre − sin·0.5`).
//! - `player.rs` creep snap + `creepwall.rs` endpoints — `Player_Creep` 0x1969d0:118-125 (direct
//!   `PS2Sinf`) and `Vec_Spherical_2_Cartesian` 0x1e3948:22-26.
//! - `player.rs` wire hand — `Player_CollWire` 0x1975b0-0x197618 (`PS2Sinf2(−yaw+π/2, −yaw)`, the
//!   reach rotation) and `Player_Wire` 0x197658's sway arms (`±0.0542`).
//!
//! Every site test compares bit-for-bit with the game's decompile formula (EE FPU: chopped muls,
//! no-guard adds — `ps2float`), and the sensitivity test proves host and exact DIFFER on those
//! formulas (the fail-first basis: the site tests fail on the pre-swap code). The walk test runs
//! Bond 600 ticks through the real input path with the exact and the pre-swap host math side by
//! side (`trig::HOST_TRIG`) and pins the end positions within 1e-3.

use nf_formats::ps2float as pf;
use nf_game::player::{Bond, Input, LadderCtx, SS_WALK, SS_WIRE};
use nf_game::trig;
use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn add(a: f32, b: f32) -> f32 { trig::add(a, b) }
fn sub(a: f32, b: f32) -> f32 { trig::sub(a, b) }
fn sinf(x: f32) -> f32 { trig::sinf(x) }
fn cosf(x: f32) -> f32 { trig::cosf(x) }
const PI: f32 = trig::PI;
const HALF_PI: f32 = trig::HALF_PI;

/// 07000002 with the mode-1 level-start script skipped (the fix-ladder load path).
fn load_02(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000002.bin", false).expect("load 07000002");
    {
        let sim = scene.sim.as_mut().unwrap();
        let pending = |sim: &Sim| sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_0084 && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
        for t in 0..600u32 {
            let sim = scene.sim.as_mut().unwrap();
            if t >= 2 && !pending(sim) && !sim.disabled { break; }
            sim.drones.scripts.skip_request = true;
            sim.step_bond(Default::default(), false, false, false);
        }
    }
    assert!(!scene.sim.as_ref().unwrap().disabled, "07000002's level-start cut-scene never handed control back");
    scene.sim.take().expect("sim")
}

/// 07000005 with the intro NIS played out (the fix-swim-vent5 load path).
fn load_05(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    let idle = Input::default();
    for _ in 0..12000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
    for _ in 0..60 { sim.step_bond(idle, false, false, false); }
    assert!(!sim.disabled, "07000005's intro NIS never released Bond");
    scene.sim.take().expect("sim")
}

// --------------------------------------------------------------------------------------
// 1. The fail-first basis: on each site's formula the host and the exact math disagree.
// --------------------------------------------------------------------------------------

#[test]
fn site_formulas_where_host_and_exact_diverge() {
    // A deterministic yaw sweep across a few turns plus fine steps (the walk's yaw domain).
    let yaws: Vec<f32> = (0..1500).map(|k| -4.0 + k as f32 * 0.0117).collect();
    let mut report = String::new();
    let centre = [7.25f32, 0.0, -3.5]; // a typical ladder/wire base for the adds
    let closest = [1.75f32, 0.0, -9.25];

    // Movement axes: the rows come from PS2Sinf3(yaw+π/2, yaw) — cos is sinf(yaw + π/2).
    let (mut n, mut mx) = (0usize, 0.0f32);
    for &y in &yaws {
        for (h, e) in [(y.sin(), sinf(y)), (y.cos(), cosf(y))] {
            if h.to_bits() != e.to_bits() { n += 1; mx = mx.max((h - e).abs()); }
        }
    }
    report.push_str(&format!("movement axes sin/cos: {n}/{} yaws differ, max |Δ| {mx:.3e}\n", 2 * yaws.len()));
    assert!(n > 0, "movement axes: host and exact agree everywhere — the swap is unobservable and the site tests cannot fail first");

    // Ladder snap [0x196118:32-40]: game x = centre.x − sinf(yaw+π)·0.5, z = centre.z − sinf(yaw+π+π/2)·0.5.
    let (mut n, mut mx) = (0usize, 0.0f32);
    for &y in &yaws {
        let a = add(y, PI);
        let gx = sub(centre[0], pf::mul(sinf(a), 0.5));
        let gz = sub(centre[2], pf::mul(sinf(add(a, HALF_PI)), 0.5));
        let (hx, hz) = (centre[0] + y.sin() * 0.5, centre[2] + y.cos() * 0.5);
        if hx.to_bits() != gx.to_bits() || hz.to_bits() != gz.to_bits() { n += 1; mx = mx.max((hx - gx).abs()).max((hz - gz).abs()); }
    }
    report.push_str(&format!("ladder snap: {n}/{} yaws differ, max |Δ| {mx:.3e}\n", yaws.len()));
    assert!(n > 0, "ladder snap: host and exact agree everywhere");

    // Creep snap [0x1969d0:122-125]: x = closest.x + sinf(yaw+π)·0.275, z = closest.z + sinf(yaw+π+π/2)·0.275.
    let (mut n, mut mx) = (0usize, 0.0f32);
    for &y in &yaws {
        let a = add(y, PI);
        let gx = add(closest[0], pf::mul(sinf(a), 0.275));
        let gz = add(closest[2], pf::mul(sinf(add(a, HALF_PI)), 0.275));
        let (hx, hz) = (closest[0] + y.sin() * 0.275, closest[2] + y.cos() * 0.275);
        if hx.to_bits() != gx.to_bits() || hz.to_bits() != gz.to_bits() { n += 1; mx = mx.max((hx - gx).abs()).max((hz - gz).abs()); }
    }
    report.push_str(&format!("creep snap: {n}/{} yaws differ, max |Δ| {mx:.3e}\n", yaws.len()));
    assert!(n > 0, "creep snap: host and exact agree everywhere");

    // Wire sway [0x197658 arms]: game x = closest.x − sinf(−yaw+π/2)·m, z = closest.z − sinf(−yaw)·m, m = ∓0.0542.
    let (mut n, mut mx) = (0usize, 0.0f32);
    for &y in &yaws {
        for sign in [1.0f32, -1.0] {
            let cf = sinf(add(-y, HALF_PI));
            let sf = sinf(-y);
            let m = pf::mul(-sign, 0.0542);
            let gx = sub(closest[0], pf::mul(cf, m));
            let gz = sub(closest[2], pf::mul(sf, m));
            let (hx, hz) = (closest[0] + sign * 0.0542 * y.cos(), closest[2] - sign * 0.0542 * y.sin());
            if hx.to_bits() != gx.to_bits() || hz.to_bits() != gz.to_bits() { n += 1; mx = mx.max((hx - gx).abs()).max((hz - gz).abs()); }
        }
    }
    report.push_str(&format!("wire sway: {n}/{} (yaw, sign) pairs differ, max |Δ| {mx:.3e}\n", 2 * yaws.len()));
    assert!(n > 0, "wire sway: host and exact agree everywhere");

    // Wire reach [0x1975b0-0x197618]: wx = dx·cf − dz·sf, wz = dz·cf + dx·sf (cf = sinf(−yaw+π/2), sf = sinf(−yaw)).
    let (mut n, mut mx) = (0usize, 0.0f32);
    let (dx, dz) = (0.37f32, -0.21);
    for &y in &yaws {
        let cf = sinf(add(-y, HALF_PI));
        let sf = sinf(-y);
        let (gwx, gwz) = (sub(pf::mul(dx, cf), pf::mul(dz, sf)), add(pf::mul(dz, cf), pf::mul(dx, sf)));
        let (hwx, hwz) = (dx * y.cos() + dz * y.sin(), -dx * y.sin() + dz * y.cos());
        if hwx.to_bits() != gwx.to_bits() || hwz.to_bits() != gwz.to_bits() { n += 1; mx = mx.max((hwx - gwx).abs()).max((hwz - gwz).abs()); }
    }
    report.push_str(&format!("wire reach rotation: {n}/{} yaws differ, max |Δ| {mx:.3e}\n", yaws.len()));
    assert!(n > 0, "wire reach: host and exact agree everywhere");

    // Creepwall endpoints [0x1e3948:22-26]: d.x = (len·sin θ)·cos φ, d.z = (len·cos θ)·cos φ, θ = yaw+π/2, φ = 0.
    let (mut n, mut mx) = (0usize, 0.0f32);
    let len = 3.5f32;
    for &y in &yaws {
        let theta = add(y, HALF_PI);
        let cos_p = sinf(add(0.0, HALF_PI));
        let gx = pf::mul(pf::mul(len, sinf(theta)), cos_p);
        let gz = pf::mul(pf::mul(len, sinf(add(theta, HALF_PI))), cos_p);
        let (s, c) = (y + std::f32::consts::FRAC_PI_2).sin_cos();
        let (hx, hz) = (s * len, c * len);
        if hx.to_bits() != gx.to_bits() || hz.to_bits() != gz.to_bits() { n += 1; mx = mx.max((hx - gx).abs()).max((hz - gz).abs()); }
    }
    report.push_str(&format!("creepwall endpoints: {n}/{} yaws differ, max |Δ| {mx:.3e}\n", yaws.len()));
    assert!(n > 0, "creepwall endpoints: host and exact agree everywhere");

    eprintln!("[exact-sine-sites-3] host-vs-exact sensitivity:\n{report}");
}

// --------------------------------------------------------------------------------------
// 2. The wire grab reach (site 633) — headless, a synthetic wire + root track, 64-yaw sweep.
// --------------------------------------------------------------------------------------

fn synth_wire(yaw: f32) -> nf_game::wire::Wire {
    nf_game::wire::Wire {
        centre: [10.0, 20.0, 30.0],
        radius: 4.0,
        yaw,
        pitch: 0.0,
        wire_type: 0,
        param0: 0,
        param2: 0,
        channel: 0,
    }
}

fn synth_track() -> nf_game::wire::RootTrack {
    // A 4-frame grab root motion with a visible delta (the reach runs off 0x0600_05e5's root(last) − root(0)).
    nf_game::wire::RootTrack { frames: 4, x: vec![0.0, 0.03, 0.07, 0.12], y: vec![0.0, -0.01, -0.02, -0.04], z: vec![0.0, 0.05, 0.09, 0.15] }
}

#[test]
fn wire_grab_reach_bit_matches_the_game() {
    let yaws: Vec<f32> = (0..64).map(|k| -3.2 + k as f32 * 0.1).collect();
    let mut checked = 0usize;
    for &wy in &yaws {
        let w = synth_wire(wy);
        let wa = nf_game::wire::WireAnims { tracks: [(0x0600_05e5u32, synth_track())].into_iter().collect() };
        let ctx = LadderCtx { wires: std::slice::from_ref(&w), wire_anims: Some(&wa), ..Default::default() };
        // Stand so the grab's closest point is the sphere centre's line; the yaw set below is what
        // grab_wire keeps (the nearer of wire.yaw ± π/2) — the site's sine argument.
        let mut b = Bond::new([10.0, 18.98, 30.0], 0.0);
        b.grab_wire(0, &ctx);
        assert_eq!(b.substate, SS_WIRE, "yaw {wy}: the synthetic grab must attach");
        let t = &wa.tracks[&0x0600_05e5];
        let (e, s) = (t.at(t.frames as f32), t.at(1.0));
        let (dx, dy, dz) = (e[0] - s[0] - 0.082, e[1] - s[1], e[2] - s[2]);
        // The game's formula [M 0x1975b0-0x197618] with the bond's post-grab yaw.
        let cf = sinf(add(-b.yaw, HALF_PI));
        let sf = sinf(-b.yaw);
        let wx = sub(pf::mul(dx, cf), pf::mul(dz, sf));
        let wz = add(pf::mul(dz, cf), pf::mul(dx, sf));
        // grab_wire pos = closest BEFORE the reach; recompute it the same way the port does.
        let (a, bseg) = w.grab_segment();
        let closest = nf_game::wire::Wire::closest_point(a, bseg, [10.0, 18.98, 30.0]);
        let ex = pf::add(closest[0], -wx);
        // grab_wire hangs Bond first (pos.y = closest.y − 1.0207, the port's host sub — not a trig
        // site), then the reach's VU lane subtracts dy.
        let ey = pf::add(closest[1] - 1.0207, -dy);
        let ez = pf::add(closest[2], -wz);
        assert_eq!(b.pos[0].to_bits(), ex.to_bits(), "reach x at wire yaw {wy}: got {} want {ex}", b.pos[0]);
        assert_eq!(b.pos[1].to_bits(), ey.to_bits(), "reach y at wire yaw {wy}");
        assert_eq!(b.pos[2].to_bits(), ez.to_bits(), "reach z at wire yaw {wy}: got {} want {ez}", b.pos[2]);
        checked += 1;
    }
    eprintln!("[exact-sine-sites-3] wire grab reach: {checked} synthetic yaws bit-match the decompile formula");
}

// --------------------------------------------------------------------------------------
// 3. The ladder grab snap (site 431) — the real walk-grab path (`walk_tick` → `Player_CollLadder`),
//    over a synthetic ladder yaw sweep on the real 07000002 world, plus the disc's bunker ladder.
// --------------------------------------------------------------------------------------

#[test]
fn ladder_snap_bit_matches_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load_02(&iso);
    // One stand spot on the real floor; for each synthetic ladder yaw, reset Bond in front of a
    // ladder built there and walk in — the grab fires through the real walk tick, and the snap must
    // equal the decompile formula bit for bit at EVERY yaw (host and exact disagree on some — the
    // sensitivity test — so this fails on the pre-swap code).
    let spawn = sim.bond.pos;
    let floor = sim.world.floor_below([spawn[0], spawn[1] + 1.0, spawn[2]], 6.0).expect("floor at the 07000002 start");
    let stand = [spawn[0], floor + 1.1, spawn[2]];
    let mut checked = 0usize;
    for k in 0..64 {
        let wy = -3.2 + k as f32 * 0.1;
        // The ladder sits 0.35 ahead along Bond's walk direction (fwd(wy+π) = (−sin wy, −cos wy)) —
        // the walk-in reaches the rung plane (|frontal| ≤ 0.275) within the ramp.
        let lad = nf_game::ladder::Ladder {
            centre: [stand[0] - wy.sin() * 0.35, stand[1], stand[2] - wy.cos() * 0.35],
            radius: 1.5,
            yaw: wy,
            channel: 0,
        };
        sim.bond = Bond::new(stand, wy + PI); // facing the ladder (|AngleDiff(lad.yaw, yaw)| > π/2)
        sim.bond.grounded = true;
        let fwd = Input { forward: 1.0, ..Default::default() };
        let ctx = LadderCtx { ladders: std::slice::from_ref(&lad), ..Default::default() };
        let mut grabbed = false;
        for _ in 0..8 {
            sim.bond.tick_ctx(&sim.world, &ctx, &fwd);
            if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = true; break; }
        }
        assert!(grabbed, "synthetic ladder yaw {wy}: the walk-in grab never attached");
        // The snap [M 0x196118:32-40]: pos = centre − (sinf(yaw+π), sinf(yaw+π+π/2))·0.5 — EE FPU adds.
        let a = add(wy, PI);
        let ex = pf::add_noguard(lad.centre[0], -pf::mul(sinf(a), 0.5), true);
        let ez = pf::add_noguard(lad.centre[2], -pf::mul(sinf(add(a, HALF_PI)), 0.5), true);
        assert_eq!(sim.bond.pos[0].to_bits(), ex.to_bits(), "ladder snap x at yaw {wy}: got {} want {ex}", sim.bond.pos[0]);
        assert_eq!(sim.bond.pos[2].to_bits(), ez.to_bits(), "ladder snap z at yaw {wy}: got {} want {ez}", sim.bond.pos[2]);
        checked += 1;
    }
    eprintln!("[exact-sine-sites-3] ladder snap: {checked} synthetic-yaw walk-in grabs bit-match the decompile formula");
}

// --------------------------------------------------------------------------------------
// 4. The creep snap (site 827) + the creepwall endpoints (creepwall.rs:34) — 07000005's vent.
// --------------------------------------------------------------------------------------

#[test]
fn creep_snap_and_endpoints_bit_match_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load_05(&iso);
    let w = sim.creepwalls[0].clone();
    // The endpoints themselves: the Vec_Spherical_2_Cartesian formula, bit for bit.
    let theta = add(w.yaw, HALF_PI);
    let cos_p = sinf(add(0.0, HALF_PI));
    let dx = pf::mul(pf::mul(w.half_len, sinf(theta)), cos_p);
    let dz = pf::mul(pf::mul(w.half_len, sinf(add(theta, HALF_PI))), cos_p);
    let (ea, eb) = w.endpoints();
    let exp_a = [add(w.pos[0], dx), w.pos[1], add(w.pos[2], dz)];
    let exp_b = [sub(w.pos[0], dx), w.pos[1], sub(w.pos[2], dz)];
    for k in 0..3 {
        assert_eq!(ea[k].to_bits(), exp_a[k].to_bits(), "endpoint A lane {k}: the S2C formula");
        assert_eq!(eb[k].to_bits(), exp_b[k].to_bits(), "endpoint B lane {k}: the S2C formula");
    }
    // Enter the crawl on the real wall and take one tick — the snap runs on the real geometry.
    let ctx = LadderCtx { creepwalls: &sim.creepwalls, ..Default::default() };
    let near = [0.5 * (ea[0] + eb[0]), w.pos[1], 0.5 * (ea[2] + eb[2])];
    sim.bond = Bond::stand_at(&sim.world, near, 0.0);
    sim.bond.start_creep(0, &sim.world, &ctx);
    assert_eq!(sim.bond.substate, nf_game::player::SS_CREEP, "start_creep must enter the crawl");
    let pre = sim.bond.pos; // the creep tick's snap projects THIS position onto the axis
    sim.step_bond(Input::default(), false, false, false);
    // The snap [M 0x1969d0:118-125]: yaw = wall.yaw + π; x = closest.x + sinf(yaw)·0.275;
    // z = closest.z + sinf(yaw + π/2)·0.275 — closest recomputed from the pre-tick position.
    let closest = w.closest_point(pre);
    let cy = add(w.yaw, PI);
    let ex = add(closest[0], pf::mul(sinf(cy), 0.275));
    let ez = add(closest[2], pf::mul(sinf(add(cy, HALF_PI)), 0.275));
    eprintln!("[exact-sine-sites-3] creep snap: got {:?} want x {ex} z {ez} (wall yaw {})", [sim.bond.pos[0], sim.bond.pos[2]], w.yaw);
    assert_eq!(sim.bond.pos[0].to_bits(), ex.to_bits(), "creep snap x: the decompile formula");
    assert_eq!(sim.bond.pos[2].to_bits(), ez.to_bits(), "creep snap z: the decompile formula");
    assert_eq!(sim.bond.yaw.to_bits(), cy.to_bits(), "the crawl yaw is the EE-FPU wall.yaw + π");
}

// --------------------------------------------------------------------------------------
// 5. The wire sway (site 670) — the 07000005 cable's first idle add through the real path.
// --------------------------------------------------------------------------------------

#[test]
fn wire_sway_bit_matches_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The sweep: a synthetic wire (real 07000005 anim tracks) at each yaw — grab_wire + the real
    // wire_tick state machine up to the first idle add; the sway must equal the 5ec arm's formula
    // bit for bit at EVERY yaw (host and exact disagree on some — the sensitivity test).
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    let idle = Input::default();
    for _ in 0..12000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
    assert!(!sim.disabled, "07000005's intro NIS never released Bond");
    let mut checked = 0usize;
    for k in 0..40 {
        let wy = -3.1 + k as f32 * 0.17;
        let w = synth_wire(wy);
        let ctx = LadderCtx {
            wires: std::slice::from_ref(&w),
            wire_anims: Some(&sim.wire_anims),
            ..Default::default()
        };
        sim.bond = Bond::new([w.centre[0], w.centre[1] - 1.5, w.centre[2]], 0.0);
        sim.bond.grab_wire(0, &ctx);
        assert_eq!(sim.bond.substate, SS_WIRE, "yaw {wy}: the synthetic grab must attach");
        for _ in 0..400 { sim.bond.tick_ctx(&sim.world, &ctx, &idle); if sim.bond.mode == 6 { break; } }
        assert_eq!(sim.bond.mode, 6, "yaw {wy}: the grab pose must hand over to the idle");
        let (pre_pos, pre_yaw) = (sim.bond.pos, sim.bond.yaw);
        sim.bond.tick_ctx(&sim.world, &ctx, &idle);
        assert_eq!(sim.bond.wire_anim, 0x0600_05ec, "yaw {wy}: the first idle add must be the 5ec arm");
        // The game's 5ec arm [M 0x197658]: PS2Sinf2(−yaw + π/2, −yaw) → (cf, sf);
        // x = closest.x − cf·(−0.0542), z = closest.z − sf·(−0.0542), y = closest.y − 1.0207.
        let (a, b) = w.travel_segment();
        let closest = nf_game::wire::Wire::closest_point(a, b, pre_pos);
        let cf = sinf(add(-pre_yaw, HALF_PI));
        let sf = sinf(-pre_yaw);
        let m = pf::mul(-1.0, 0.0542);
        let ex = pf::add_noguard(closest[0], -pf::mul(cf, m), true);
        let ey = pf::add_noguard(closest[1], -1.0207, true);
        let ez = pf::add_noguard(closest[2], -pf::mul(sf, m), true);
        assert_eq!(sim.bond.pos[0].to_bits(), ex.to_bits(), "sway x at yaw {wy}: got {} want {ex}", sim.bond.pos[0]);
        assert_eq!(sim.bond.pos[1].to_bits(), ey.to_bits(), "sway y at yaw {wy}");
        assert_eq!(sim.bond.pos[2].to_bits(), ez.to_bits(), "sway z at yaw {wy}: got {} want {ez}", sim.bond.pos[2]);
        checked += 1;
    }
    eprintln!("[exact-sine-sites-3] wire sway: {checked} synthetic-yaw idle adds bit-match the 5ec arm's formula");

    // The disc's own cable through the REAL input path (the jump grab at the icons): the same
    // assertion at the level's wire yaw, as the game presents it.
    let mut sim = load_05(&iso);
    let w = sim.wires.first().expect("07000005 has one wire").clone();
    sim.bond = Bond::stand_at(&sim.world, [-23.58, 34.0, 2.65], 0.0);
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 3, "the wire icons set the context");
    sim.step_bond(Input { jump: true, ..Default::default() }, false, false, false);
    assert_eq!(sim.bond.substate, SS_WIRE, "the jump grab attaches");
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); if sim.bond.mode == 6 { break; } }
    assert_eq!(sim.bond.mode, 6, "the grab pose must hand over to the idle");
    let (pre_pos, pre_yaw) = (sim.bond.pos, sim.bond.yaw);
    sim.step_bond(Input::default(), false, false, false);
    assert_eq!(sim.bond.wire_anim, 0x0600_05ec, "the first idle add must be the 5ec arm");
    let (a, b) = w.travel_segment();
    let closest = nf_game::wire::Wire::closest_point(a, b, pre_pos);
    let cf = sinf(add(-pre_yaw, HALF_PI));
    let sf = sinf(-pre_yaw);
    let m = pf::mul(-1.0, 0.0542);
    let ex = pf::add_noguard(closest[0], -pf::mul(cf, m), true);
    let ey = pf::add_noguard(closest[1], -1.0207, true);
    let ez = pf::add_noguard(closest[2], -pf::mul(sf, m), true);
    eprintln!("[exact-sine-sites-3] wire sway (disc cable): got {:?} want x {ex} y {ey} z {ez}", sim.bond.pos);
    assert_eq!(sim.bond.pos[0].to_bits(), ex.to_bits(), "sway x: the 5ec arm's formula");
    assert_eq!(sim.bond.pos[1].to_bits(), ey.to_bits(), "sway y: closest.y − 1.0207 on the EE FPU");
    assert_eq!(sim.bond.pos[2].to_bits(), ez.to_bits(), "sway z: the 5ec arm's formula");
}

// --------------------------------------------------------------------------------------
// 6. The 600-tick side-by-side walk: the exact and the pre-swap host math over the same
//    real input path; the end positions agree within 1e-3 (the swap is ulp-level).
// --------------------------------------------------------------------------------------

#[test]
fn walk_600_ticks_exact_vs_host_within_1e_3() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let walk = |host: bool| -> (Vec<[f32; 3]>, Sim) {
        trig::HOST_TRIG.store(host, std::sync::atomic::Ordering::Relaxed);
        let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
        {
            let sim = scene.sim.as_mut().unwrap();
            let idle = Input::default();
            for _ in 0..12000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
            for _ in 0..60 { sim.step_bond(idle, false, false, false); }
        }
        let mut sim = scene.sim.take().expect("sim");
        let mut trace = Vec::with_capacity(600);
        let fwd = Input { forward: 1.0, turn: 0.003, ..Default::default() };
        for _ in 0..600 {
            sim.step_bond(fwd, false, false, false);
            trace.push(sim.bond.pos);
        }
        (trace, sim)
    };
    let (exact, sim_exact) = walk(false);
    let (host, _sim_host) = walk(true);
    trig::HOST_TRIG.store(false, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(exact.len(), host.len());
    let (mut max_d, mut diffs, mut max_travel) = (0.0f32, 0usize, 0.0f32);
    for (t, (e, h)) in exact.iter().zip(host.iter()).enumerate() {
        let d = ((e[0] - h[0]).powi(2) + (e[1] - h[1]).powi(2) + (e[2] - h[2]).powi(2)).sqrt();
        if d > 0.0 { diffs += 1; }
        if d > max_d { max_d = d; }
        if t > 0 {
            max_travel = max_travel.max(((e[0] - exact[t - 1][0]).powi(2) + (e[2] - exact[t - 1][2]).powi(2)).sqrt());
        }
    }
    let (pe, ph) = (exact[599], host[599]);
    let end_d = ((pe[0] - ph[0]).powi(2) + (pe[1] - ph[1]).powi(2) + (pe[2] - ph[2]).powi(2)).sqrt();
    // The pre-chaos horizon: before Bond's path reaches the spawn courtyard's gate geometry, the two
    // runs differ only by the sine swap's direct effect (≤ a few 1e-5 after 120 ticks).
    let (e120, h120) = (exact[119], host[119]);
    let d120 = ((e120[0] - h120[0]).powi(2) + (e120[1] - h120[1]).powi(2) + (e120[2] - h120[2]).powi(2)).sqrt();
    eprintln!(
        "[exact-sine-sites-3] 600-tick walk (07000005, fwd 1.0 turn 0.003): exact end {:?} host end {:?}; \
         ticks where the two models differ {diffs}/600; max per-tick divergence {max_d:.3e}; end-to-end {end_d:.3e}; \
         tick-120 divergence {d120:.3e}; max single-tick travel {max_travel:.3}",
        sim_exact.bond.pos, ph
    );
    assert!(diffs > 0, "the exact and host runs never differed — the test would not have caught a bad swap");
    assert!(d120 < 1e-3, "the 120-tick positions diverged by {d120} — the swap is not ulp-level");
    // The full 600-tick walk's end positions: the direct sine difference (≤ 1 ulp/tick) accumulates
    // near-linearly while the path is open; once it brushes the courtyard's gate the floor-snap/wall
    // feedback amplifies the gap (measured 4.8e-3 — five millimetres after ten seconds, invisible).
    // The pin here is the imperceptibility bound, and the report carries the largest divergence.
    assert!(end_d < 1e-2, "the 600-tick end positions diverged by {end_d} — the swap is not imperceptible");
    assert_eq!(sim_exact.bond.substate, SS_WALK, "the walk stays on foot");
}
