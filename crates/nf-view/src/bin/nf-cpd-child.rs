//! The cross-process-determinism guard's child runner (a plain binary — no libtest harness in the
//! child, so nothing between the process and its output). Loaded by
//! `crates/nf-view/tests/cross_process_determinism.rs` through `CARGO_BIN_EXE_nf-cpd-child`.
//!
//! Protocol: env `NF_ISO` (the image), `NF_OUT` (the TICK file to write), optional `NF_TOY=1`
//! (the sister's deliberately order-dependent column). It loads 07000005, parks Bond in a live
//! guard's sight (the fix-guard-weapons park) and runs 600 ticks of real combat — the guard sights
//! Bond (the 120-tick NewSighting gate), fires, draws cover/reposition rolls from the shared `Rng`
//! — writing one line per tick: `TICK <frame> <state_hash> <toy_hash>`.
//!
//! `state_hash` is the same fingerprint discipline as `mp::hash::state_hash` (FNV-1a 64 over IEEE
//! bit patterns, fixed field order frame → rng → bond → drones → bullets → grenades →
//! explosions): a pure function of the sim's bits, stable across processes and machines. Two
//! runs of this binary in two processes already carry different std `RandomState` seeds — every
//! `std` HashMap iterates in a different order in each — so any order-dependent site feeding the
//! sim state, the RNG draw order or an AI decision changes the stream and the parent fails.

use nf_game::mp::hash::Hasher;
use nf_view::scene::Sim;

const LEVEL: &str = "07000005.bin";
const TICKS: u32 = 600;

/// The per-tick state fingerprint. Field order is fixed by the code below; a field left outhashed
/// is a desync the hash cannot see.
fn state_hash(sim: &Sim) -> u64 {
    let mut h = Hasher::new();
    let d = &sim.drones;
    h.u32(d.frame);
    h.u32(d.rng.x);
    h.u32(d.rng.y);
    h.u64(d.rng.draws);
    for c in sim.bond.pos { h.f32(c); }
    h.f32(sim.bond.yaw);
    h.f32(d.health.health);
    h.f32(d.health.armour);
    h.usize(d.list.len());
    for dr in &d.list {
        h.bool(dr.present);
        h.u32(dr.skin);
        h.u32(dr.weapon);
        for c in dr.pos { h.f32(c); }
        h.f32(dr.yaw);
        h.f32(dr.desired_yaw);
        h.f32(dr.aim_yaw);
        h.f32(dr.aim_pitch);
        h.u8(dr.mode);
        h.u8(dr.state);
        h.u8(dr.prev_state);
        h.f32(dr.health);
        h.u32(dr.seen_ticks);
        h.u32(dr.unseen_ticks);
        h.bool(dr.los);
        h.bool(dr.seen);
        h.f32(dr.alert);
        h.bool(dr.tracking);
        h.bool(dr.firing);
        h.i32(dr.burst);
        h.u32(dr.next_shot);
        h.u32(dr.anim.time);
        h.f32(dr.anim.ftime);
        h.u16(dr.anim.state);
        h.u16(dr.anim.danim);
        h.u16(dr.anim.variant);
    }
    h.usize(d.bullets.len());
    for b in &d.bullets {
        for c in b.pos { h.f32(c); }
        h.usize(b.owner);
        h.u16(b.weapon);
    }
    h.usize(d.grenades.len());
    for g in &d.grenades {
        for c in g.pos { h.f32(c); }
    }
    h.usize(d.explosions.len());
    for e in &d.explosions {
        for c in e.0 { h.f32(c); }
        h.f32(e.1);
    }
    h.finish()
}

fn main() {
    let iso = std::env::var("NF_ISO").expect("the child needs NF_ISO");
    let out_path = std::env::var("NF_OUT").expect("the child needs NF_OUT");
    let toy = std::env::var_os("NF_TOY").is_some();
    let mut scene = nf_view::load(&iso, LEVEL, false).unwrap_or_else(|e| panic!("load 07000005: {e}"));
    let guard = {
        let sim = scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len()).find(|&i| {
            let d = &sim.drones.list[i];
            d.present && d.weapon != 0 && d.aggression == 0
        }).expect("no live armed guard on 07000005")
    };
    // Park Bond 10 units from the guard along an axis, level floor, in the guard's sight — the
    // fix-guard-weapons park, so the engagement opens inside the 600 ticks (the sight gate alone
    // eats 120 of them).
    let mut placed = false;
    'dirs: for &(dx, dz) in &[(1.0f32, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.70710678, 0.70710678), (-0.70710678, 0.70710678), (0.70710678, -0.70710678), (-0.70710678, -0.70710678)] {
        let sim = scene.sim.as_mut().unwrap();
        let g = sim.drones.list[guard].pos;
        sim.bond.pos = [g[0] + dx * 10.0, g[1] + 0.14, g[2] + dz * 10.0];
        sim.bond.yaw = (g[0] - sim.bond.pos[0]).atan2(g[2] - sim.bond.pos[2]);
        let bp = sim.bond.pos;
        if !sim.world.line_of_sight([g[0], g[1] + 1.5, g[2]], [bp[0], bp[1] + 1.5, bp[2]]) { continue 'dirs; }
        for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
        let y = sim.bond.pos[1];
        if (y - (g[1] + 0.14)).abs() < 1.0 { placed = true; break 'dirs; }
    }
    assert!(placed, "no level parking spot in the guard's sight");
    // The toy map: 64 entries, iterated ONLY for its order (the values never change). One map for
    // the whole run, so the order is stable within the process and varies across processes — the
    // per-process state a real leak would carry, in isolation.
    let toy_map: std::collections::HashMap<u32, u32> = (0..64u32).map(|k| (k * 0x9E3779B9 ^ 0xA5A5A5A5, k)).collect();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&out_path).expect("create NF_OUT"));
    use std::io::Write;
    for _ in 0..TICKS {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        sim.drones.health.health = 100.0; // Bond survives the firefight; the AI stays in combat
        let sh = state_hash(sim);
        let mut th = Hasher::new();
        th.u32(sim.drones.frame);
        if toy {
            // The sister's deliberate order dependence: the SAME entries, iterated in whatever
            // order this process's RandomState picked.
            for k in toy_map.keys().copied().collect::<Vec<u32>>() { th.u32(k); th.u32(toy_map[&k]); }
        }
        // The TICK stream rides stderr (an inherited fd → the parent's log file): this sandbox
        // makes a child's own file creations invisible to the live parent, while writes to an
        // inherited descriptor land in the parent-created log at once. NF_OUT keeps a copy.
        let line = format!("TICK {} {:016x} {:016x}", sim.drones.frame, sh, th.finish());
        eprintln!("{line}");
        writeln!(out, "{line}").expect("write a TICK line");
    }
    out.flush().expect("flush NF_OUT");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[cpd] done: frame {}, draws {}, {} live drones, guard {guard} saw {} ticks",
        sim.drones.frame, sim.drones.rng.draws,
        sim.drones.list.iter().filter(|d| d.present).count(),
        sim.drones.list[guard].seen_ticks);
}
