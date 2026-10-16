//! cross-process-determinism: the sim must give the same result in every process. 664 found the
//! guard-reaction matrix (`fix_guard_weapons_live`) measuring different streams in different
//! processes of the same binary — per-process state (a std HashMap's RandomState is seeded per
//! process; an audio callback's wall-clock end flags; a thread-local; the loader's timing) feeding
//! sim state, the shared RNG's draw order, or an AI decision. That also breaks online lockstep:
//! two machines are two processes.
//!
//! The guard: the child (`det_child_tick_stream`, `#[ignore]`d so a plain run no-ops) loads
//! 07000005, parks Bond in a live guard's sight (the same park the fix-guard-weapons harness uses)
//! and runs 600 ticks of real combat — the guard sights Bond (the 120-tick NewSighting gate),
//! fires, draws cover/reposition rolls from the shared `Rng` — and after EVERY tick folds a state
//! fingerprint with `nf_game::mp::hash::Hasher` (the lockstep hasher: FNV-1a 64 over IEEE-754 bit
//! patterns; the MP design requirement is the same one: a pure function of the sim's bits, stable
//! across processes and machines). The parent spawns the binary TWICE as two child processes and
//! requires the 600 `TICK` fingerprints bit-identical. The TICK stream goes through a FILE
//! (`NF_DET_OUT`), not the test harness's stdout — libtest's own output capture must not sit
//! between the child and the parent. Two child processes already run with different `RandomState`
//! seeds — every `std` HashMap in the load and tick path iterates in a different order in each —
//! so any order-dependent site feeding the sim fails this.
//!
//! The sister proves the guard can actually trip: the child's `NF_DET_TOY=1` mode folds one
//! deliberately order-dependent term into a SEPARATE column (a `std` HashMap iterated for its
//! order alone; the sim state column stays clean). Two toy children must produce DIFFERENT toy
//! columns at some tick while their state columns stay bit-identical — the toy trip and the
//! non-interference, in one run pair.
//!
//! Opt-in: `NF_ISO` (both tests skip when unset, like the other live tests). The child inherits
//! the parent's env plus the det flags.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const LEVEL: &str = "07000005.bin";
const TICKS: u32 = 600;

static CHILD_SEQ: AtomicUsize = AtomicUsize::new(0);

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The per-tick state fingerprint: everything whose divergence would be a real desync — the shared
/// RNG's state and draw count, Bond's body, the health pool, and every drone's decision state
/// (mode/state machines, anim, sighting, aim, fire timers) plus the in-flight projectiles and
/// blasts. Field order is fixed by the code below (frame → rng → bond → drones → bullets →
/// grenades → explosions); a field left outhashed is a desync the hash cannot see.
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

/// One child run: load, park Bond in a live guard's sight, 600 combat ticks, one `TICK` line per
/// tick (`TICK <frame> <state_hash> <toy_hash>`) — into `NF_DET_OUT` when set, else stdout. The
/// toy column is the deliberately order-dependent term (the sister); without `NF_DET_TOY` it is
/// the frame alone.
fn run_child(toy: bool) {
    let iso = iso().expect("the child needs NF_ISO");
    let mut scene = nf_view::load(&iso, LEVEL, false).expect("load 07000005");
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
    let mut out: Vec<String> = Vec::with_capacity(TICKS as usize);
    for _ in 0..TICKS {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        sim.drones.health.health = 100.0; // Bond survives the firefight; the AI stays in combat
        let sh = state_hash(sim);
        let mut th = nf_game::mp::hash::Hasher::new();
        th.u32(sim.drones.frame);
        if toy {
            // The sister's deliberate order dependence: the SAME entries, iterated in whatever
            // order this process's RandomState picked.
            for k in toy_map.keys().copied().collect::<Vec<u32>>() { th.u32(k); th.u32(toy_map[&k]); }
        }
        out.push(format!("TICK {} {:016x} {:016x}", sim.drones.frame, sh, th.finish()));
    }
    if let Ok(path) = std::env::var("NF_DET_OUT") {
        std::fs::write(&path, out.join("\n") + "\n").expect("write the TICK file");
    } else {
        for l in &out { println!("{l}"); }
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[cpd] done: frame {}, draws {}, {} live drones, guard {guard} saw {} ticks",
        sim.drones.frame, sim.drones.rng.draws,
        sim.drones.list.iter().filter(|d| d.present).count(),
        sim.drones.list[guard].seen_ticks);
}

/// The child worker (`--ignored --exact`, spawned by the parent tests below). A no-op unless
/// `NF_DET_CHILD` is set, so nothing here runs in a normal suite pass.
#[test]
#[ignore]
fn det_child_tick_stream() {
    if std::env::var_os("NF_DET_CHILD").is_none() { return; }
    run_child(std::env::var_os("NF_DET_TOY").is_some());
}

/// Spawn this test binary as a child process running only `det_child_tick_stream`, and read its
/// TICK stream back from the file `NF_DET_OUT` (no libtest stdout capture in between).
fn spawn_child(toy: bool) -> Vec<(u32, u64, u64)> {
    let exe = std::env::current_exe().expect("current_exe");
    let out_path = std::env::temp_dir().join(format!(
        "nf_cpd_{}_{}_{}.txt", std::process::id(), CHILD_SEQ.fetch_add(1, Ordering::SeqCst), toy as u8));
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "det_child_tick_stream", "--ignored", "--test-threads=1"]);
    cmd.env("NF_DET_CHILD", "1");
    cmd.env("NF_DET_OUT", &out_path);
    if toy { cmd.env("NF_DET_TOY", "1"); }
    let out = cmd.output().expect("spawn the child");
    let ticks: Vec<(u32, u64, u64)> = match std::fs::read_to_string(&out_path) {
        Ok(s) => s.lines().filter_map(|l| {
            let l = l.strip_prefix("TICK ")?;
            let mut it = l.split_whitespace();
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?))
        }).collect(),
        Err(e) => panic!("no TICK file at {}: {e}\nchild stdout:\n{}\nchild stderr:\n{}", out_path.display(),
            String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
    };
    let _ = std::fs::remove_file(&out_path);
    if ticks.len() != TICKS as usize {
        panic!("the child wrote {} TICK lines, wanted {}\nchild stdout:\n{}\nchild stderr:\n{}", ticks.len(), TICKS,
            String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    }
    ticks
}

/// THE GUARD: 600 ticks of 07000005 combat in two child processes of the same binary — bit-identical
/// per-tick state fingerprints. Two processes already carry different HashMap seeds (std's
/// RandomState), so any order-dependent site feeding the sim state, the RNG draw order or an AI
/// decision fails this with the offending tick named.
#[test]
fn two_processes_produce_bit_identical_per_tick_state_hashes() {
    if iso().is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let a = spawn_child(false);
    let b = spawn_child(false);
    let mut first_diff = None;
    for (ta, tb) in a.iter().zip(&b) {
        if ta.0 != tb.0 { panic!("frame streams diverged: {} vs {}", ta.0, tb.0); }
        if ta.1 != tb.1 && first_diff.is_none() { first_diff = Some((ta.0, ta.1, tb.1)); }
    }
    assert!(first_diff.is_none(), "the two processes' sims diverged first at tick {:?} ({:x} vs {:x}) — a per-process state leak feeds the sim", first_diff.unwrap().0, first_diff.unwrap().1, first_diff.unwrap().2);
    eprintln!("[cpd] guard: {} ticks, bit-identical state fingerprints across two processes", a.len());
}

/// THE SISTER: the same pair of child runs with the toy order-dependent term folded into the toy
/// column. The toy columns must DIFFER at some tick (two processes, two HashMap orders — the guard
/// catches that), while the state columns stay bit-identical (the toy does not touch the sim).
#[test]
fn the_guard_trips_on_a_deliberately_order_dependent_stream() {
    if iso().is_none() { eprintln!("skip: NF_ISO not set"); return; }
    let a = spawn_child(true);
    let b = spawn_child(true);
    let mut toy_diff = None;
    for (ta, tb) in a.iter().zip(&b) {
        if ta.1 != tb.1 { panic!("the toy leaked into the sim state: tick {} state {:x} vs {:x}", ta.0, ta.1, tb.1); }
        if ta.2 != tb.2 && toy_diff.is_none() { toy_diff = Some((ta.0, ta.2, tb.2)); }
    }
    let (tick, ha, hb) = toy_diff.expect("the two toy children's order columns were identical — the toy map iterated in the same order in both processes (a 1-in-astronomically-many RandomState collision, or the fold lost its order dependence): the guard could not have tripped");
    eprintln!("[cpd] sister: the toy order columns diverge from tick {tick} ({ha:016x} vs {hb:016x}) while the state columns stay bit-identical — the guard trips on order dependence");
}
