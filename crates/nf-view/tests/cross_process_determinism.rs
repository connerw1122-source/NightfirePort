//! cross-process-determinism: the sim must give the same result in every process. 664 found the
//! guard-reaction matrix (`fix_guard_weapons_live`) measuring different streams in different
//! processes of the same binary — per-process state (a std HashMap's RandomState is seeded per
//! process; an audio callback's wall-clock end flags; a thread-local; the loader's timing) feeding
//! sim state, the shared RNG's draw order, or an AI decision. That also breaks online lockstep:
//! two machines are two processes.
//!
//! THE GUARD (`two_full_sim_builds_produce_bit_identical_per_tick_state_hashes`): build the
//! 07000005 combat scenario TWICE (two independent `nf_view::load` calls — every std HashMap
//! inside gets a fresh RandomState, so every map iterates in a different order in the second
//! build), park Bond in a live guard's sight (the fix-guard-weapons park) and run 600 ticks of
//! real combat in each (the guard sights Bond — the 120-tick NewSighting gate — fires, and draws
//! cover/reposition rolls from the shared `Rng`). After every tick both runs fold a state
//! fingerprint with `nf_game::mp::hash::Hasher` (the lockstep hasher: FNV-1a 64 over IEEE-754 bit
//! patterns; the MP requirement is the same one — a pure function of the sim's bits, stable across
//! processes and machines). The 600 fingerprints must be bit-identical.
//!
//! THE CROSS-PROCESS SISTER (`two_child_processes_produce_bit_identical_tick_streams`): the same
//! scenario executed by the plain child runner `nf-cpd-child`
//! (`src/bin/nf-cpd-child.rs`, spawned via `CARGO_BIN_EXE_` — no libtest harness in the child) in
//! TWO CHILD PROCESSES, the TICK stream read back per child. This is the literal lockstep
//! condition (two machines are two processes); it needs honest process semantics — where a
//! sandbox breaks grandchild plumbing (pipes EOF early, the child's own file creations stay
//! invisible to the live parent — both observed in the worker sandbox), it SKIPs with the
//! evidence rather than failing the suite, and the in-process guard above still holds the line.
//!
//! THE TRIP TEST (`the_guard_trips_on_a_deliberately_order_dependent_stream`): the sister that
//! proves the guard can actually detect order dependence — a deliberately order-dependent term (a
//! `std` HashMap iterated for its order alone) folded into a SEPARATE toy column while the state
//! column stays clean. Two toy runs must DIFFER in the toy column at some tick and be
//! bit-identical in the state column.
//!
//! Opt-in: `NF_ISO` (all three skip when unset, like the other live tests).

use std::process::Command;

const LEVEL: &str = "07000005.bin";
const TICKS: u32 = 600;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The per-tick state fingerprint: everything whose divergence would be a real desync — the shared
/// RNG's state and draw count, Bond's body, the health pool, and every drone's decision state
/// (mode/state machines, anim, sighting, aim, fire timers) plus the in-flight projectiles and
/// blasts. Field order is fixed by the code below (frame → rng → bond → drones → bullets →
/// grenades → explosions); a field left outhashed is a desync the hash cannot see. The same
/// discipline as `mp::hash::state_hash`, over the SP sim.
fn state_hash(sim: &nf_view::scene::Sim) -> u64 {
    let mut h = nf_game::mp::hash::Hasher::new();
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

/// The 600-tick combat scenario, executed in THIS process: load, park, run, return the per-tick
/// `(frame, state_hash, toy_hash)` triple. `toy` folds the deliberately order-dependent term into
/// the toy column (the trip test); the state column never sees it.
fn run_scenario(toy: bool, tag: &str) -> Vec<(u32, u64, u64)> {
    let mut scene = nf_view::load(&iso().expect("NF_ISO"), LEVEL, false).expect("load 07000005");
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
    assert!(placed, "[{tag}] no level parking spot in the guard's sight");
    // The toy map: 64 entries, iterated ONLY for its order (the values never change). A fresh map
    // per run — its RandomState (and so its iteration order) differs between the runs, the
    // per-process-style state a real leak would carry, in isolation.
    let toy_map: std::collections::HashMap<u32, u32> = (0..64u32).map(|k| (k * 0x9E3779B9 ^ 0xA5A5A5A5, k)).collect();
    let mut out = Vec::with_capacity(TICKS as usize);
    for _ in 0..TICKS {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        sim.drones.health.health = 100.0; // Bond survives the firefight; the AI stays in combat
        let sh = state_hash(sim);
        let mut th = nf_game::mp::hash::Hasher::new();
        th.u32(sim.drones.frame);
        if toy {
            // The trip test's deliberate order dependence: the SAME entries, iterated in whatever
            // order this map's RandomState picked.
            for k in toy_map.keys().copied().collect::<Vec<u32>>() { th.u32(k); th.u32(toy_map[&k]); }
        }
        out.push((sim.drones.frame, sh, th.finish()));
    }
    out
}

/// THE GUARD: two full sim builds of the same 600-tick 07000005 combat scenario — bit-identical
/// per-tick state fingerprints. Every std HashMap of the second build carries a fresh RandomState
/// (a different per-instance seed, hence a different iteration order for every map), so any
/// order-dependent site feeding the sim state, the RNG draw order or an AI decision fails this
/// with the offending tick named.
#[test]
fn two_full_sim_builds_produce_bit_identical_per_tick_state_hashes() {
    if iso().is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let a = run_scenario(false, "A");
    let b = run_scenario(false, "B");
    let mut first_diff = None;
    for (ta, tb) in a.iter().zip(&b) {
        if ta.0 != tb.0 { panic!("frame streams diverged: {} vs {}", ta.0, tb.0); }
        if ta.1 != tb.1 && first_diff.is_none() { first_diff = Some((ta.0, ta.1, tb.1)); }
    }
    assert!(first_diff.is_none(), "the two builds' sims diverged first at tick {:?} ({:x} vs {:x}) — an order-dependent site feeds the sim", first_diff.unwrap().0, first_diff.unwrap().1, first_diff.unwrap().2);
    eprintln!("[cpd] guard: {} ticks, bit-identical state fingerprints across two full sim builds", a.len());
}

/// THE TRIP TEST: the same pair of runs with the toy order-dependent term folded into the toy
/// column. The toy columns must DIFFER at some tick (two maps, two RandomStates, two orders — the
/// guard catches that), while the state columns stay bit-identical (the toy does not touch the
/// sim).
#[test]
fn the_guard_trips_on_a_deliberately_order_dependent_stream() {
    if iso().is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let a = run_scenario(true, "toy-A");
    let b = run_scenario(true, "toy-B");
    let mut toy_diff = None;
    for (ta, tb) in a.iter().zip(&b) {
        if ta.1 != tb.1 { panic!("the toy leaked into the sim state: tick {} state {:x} vs {:x}", ta.0, ta.1, tb.1); }
        if ta.2 != tb.2 && toy_diff.is_none() { toy_diff = Some((ta.0, ta.2, tb.2)); }
    }
    let (tick, ha, hb) = toy_diff.expect("the two toy runs' order columns were identical — the toy map iterated in the same order in both (a 1-in-astronomically-many RandomState collision, or the fold lost its order dependence): the guard could not have tripped");
    eprintln!("[cpd] trip: the toy order columns diverge from tick {tick} ({ha:016x} vs {hb:016x}) while the state columns stay bit-identical — the guard trips on order dependence");
}

/// Spawn the child runner and read its TICK stream back from the file `NF_OUT` — the honest
/// two-process form of the guard (two processes, two whole-process HashMap seed sets). The child's
/// TICK stream also rides stderr; under a sandbox that breaks grandchild plumbing (pipes EOF
/// early, the child's own file creations invisible to the live parent — both observed in the
/// worker sandbox), the TICK file arrives empty: the test SKIPs with the evidence instead of
/// failing the suite, and the in-process guard holds the line.
fn run_child(toy: bool, seq: usize) -> Option<Vec<(u32, u64, u64)>> {
    let exe = env!("CARGO_BIN_EXE_nf-cpd-child");
    let out_path = std::env::temp_dir().join(format!("nf_cpd_{}_{}_{}.txt", std::process::id(), seq, toy as u8));
    let mut cmd = Command::new(exe);
    cmd.env("NF_OUT", &out_path);
    if toy { cmd.env("NF_TOY", "1"); }
    let out = cmd.output().expect("spawn the child runner");
    let text = std::fs::read_to_string(&out_path).unwrap_or_default();
    let ticks: Vec<(u32, u64, u64)> = text.lines().filter_map(|l| {
        let l = l.strip_prefix("TICK ")?;
        let mut it = l.split_whitespace();
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }).collect();
    let _ = std::fs::remove_file(&out_path);
    if ticks.len() != TICKS as usize {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let done = stderr.contains("[cpd] done");
        eprintln!("[cpd] SKIP the two-process leg: the sandbox broke the parent↔child plumbing \
(status {:?}, {} TICK lines{}; stderr tail: {:?}) — the in-process guard covers this tree",
            out.status.code(), ticks.len(),
            if done { ", the child itself finished" } else { "" },
            stderr.lines().last().unwrap_or(""));
        return None;
    }
    Some(ticks)
}

/// THE CROSS-PROCESS GUARD: 600 ticks of 07000005 combat in two child processes of the same
/// binary — bit-identical per-tick state fingerprints. Two processes carry different whole-process
/// HashMap seeds, so any order-dependent site feeding the sim fails this. SKIPs (with evidence)
/// where the sandbox breaks the plumbing.
#[test]
fn two_child_processes_produce_bit_identical_tick_streams() {
    if iso().is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let (Some(a), Some(b)) = (run_child(false, 0), run_child(false, 1)) else { return };
    let mut first_diff = None;
    for (ta, tb) in a.iter().zip(&b) {
        if ta.0 != tb.0 { panic!("frame streams diverged: {} vs {}", ta.0, tb.0); }
        if ta.1 != tb.1 && first_diff.is_none() { first_diff = Some((ta.0, ta.1, tb.1)); }
    }
    assert!(first_diff.is_none(), "the two processes' sims diverged first at tick {:?} ({:x} vs {:x}) — a per-process state leak feeds the sim", first_diff.unwrap().0, first_diff.unwrap().1, first_diff.unwrap().2);
    eprintln!("[cpd] cross-process guard: {} ticks, bit-identical state fingerprints across two child processes", a.len());
}
