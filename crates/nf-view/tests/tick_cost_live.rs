//! Headless tick-cost probe (`NF_ISO`, release-only) — task sim-tick-cost.
//!
//! Loads each single-player story level, runs 600 ticks of a FIXED input script (the same script
//! for every level; the script itself draws nothing from the sim's RNG) through the real
//! `Sim::step_bond`, and records:
//!   - the per-tick wall cost (median / p95 / max over the 600 ticks),
//!   - the per-phase cost from `nf_game::prof`'s scope samples (the same scopes the NF_PROFILE=1
//!     averages print, sliced per tick so percentiles are possible),
//!   - a per-tick FNV-1a state digest over the sim state that later ticks read (bond, drones,
//!     bullets, grenades, doors, pickups, inventory, channels, triggers, script players, SS
//!     objects, world objects, the AI hearing list, the audio pool's live ids, mission, RNG).
//!
//! Determinism bar (task: identical per-tick state hash before and after): run the probe on the
//! pre-fix tree with `NF_TICKCOST_RECORD=<path>` (outside the repo — never commit it), then on
//! the post-fix tree with `NF_TICKCOST_CHECK=<path>`; the test asserts every level's 601 digests
//! match and prints the first divergent tick otherwise. The record holds input only + state
//! digests + timings — no game bytes — but it is build-specific by design (master legitimately
//! changes the sim), so it stays a scratch artifact, not a committed golden.
//!
//! Always-on assertions (each can fail):
//!   - the sim actually ran: 600 ticks → `drones.frame == 600`, and the RNG advanced;
//!   - the digest machinery is not vacuous: the tick-0 digest differs from the final digest;
//!   - in-process two-run determinism (second test): two fresh loads of 07000005 driven through
//!     the same 300-tick script produce identical digest traces — a real divergence fails.
//!
//! `NF_TICKCOST_ONLY=<name>` restricts the run to one level (comma-separated), e.g. 07000005.bin.

use std::collections::HashMap;
use std::time::Instant;

use nf_view::replay::TickInput;

/// The single-player story levels (the same list `level_live.rs` pins the counts for).
const LEVELS: [(&str, u32); 20] = [
    ("07000001.bin", 0x07000001),
    ("07000002.bin", 0x07000002),
    ("07000003.bin", 0x07000003),
    ("07000004.bin", 0x07000004),
    ("07000005.bin", 0x07000005),
    ("07000006.bin", 0x07000006),
    ("07000007.bin", 0x07000007),
    ("07000008.bin", 0x07000008),
    ("07000009.bin", 0x07000009),
    ("0700000a.bin", 0x0700000a),
    ("0700000b.bin", 0x0700000b),
    ("0700000c.bin", 0x0700000c),
    ("0700000d.bin", 0x0700000d),
    ("07000011.bin", 0x07000011),
    ("07000012.bin", 0x07000012),
    ("07000013.bin", 0x07000013),
    ("07000014.bin", 0x07000014),
    ("07000015.bin", 0x07000015),
    ("07000016.bin", 0x07000016),
    ("0700001b.bin", 0x0700001b),
];

const TICKS: usize = 600;

/// The fixed input script: identical for every level, no RNG involvement. Forward + slow turn
/// (movement, footstep sounds, the AI's hearing), a strafe leg, a 40-tick trigger hold (bullets,
/// casings, reload at 300), back-pedal, a second forward leg; use-key pulses (door activation
/// attempts), jumps, a crouch window.
fn input_script() -> Vec<TickInput> {
    let mut t = Vec::with_capacity(TICKS);
    for i in 0..TICKS {
        let mut k = TickInput::default();
        if i < 200 { k.forward = 1.0; k.turn = 0.008; }
        else if i < 260 { k.strafe = 1.0; }
        else if i < 300 { k.trigger = true; }
        else if i < 420 { k.forward = -1.0; }
        else { k.forward = 1.0; k.turn = -0.008; }
        if i == 300 { k.reload = true; }
        if i % 47 == 0 { k.use_key = true; }
        if i == 90 || i == 250 || i == 500 { k.jump = true; }
        if i >= 130 && i < 170 { k.crouch = true; }
        t.push(k);
    }
    t
}

// ── FNV-1a 64-bit ─────────────────────────────────────────────────────────────
struct Fnv(u64);
impl Fnv {
    fn new() -> Fnv { Fnv(0xcbf29ce484222325) }
    fn bytes(&mut self, b: &[u8]) {
        for &x in b { self.0 ^= x as u64; self.0 = self.0.wrapping_mul(0x100000001b3); }
    }
    fn f32(&mut self, v: f32) { self.bytes(&v.to_le_bytes()); }
    fn v3(&mut self, v: [f32; 3]) { self.f32(v[0]); self.f32(v[1]); self.f32(v[2]); }
    fn i64(&mut self, v: i64) { self.bytes(&v.to_le_bytes()); }
    fn u64v(&mut self, v: u64) { self.bytes(&v.to_le_bytes()); }
    fn usz(&mut self, v: usize) { self.i64(v as i64); }
    fn boolean(&mut self, v: bool) { self.bytes(&[v as u8]); }
    fn optu8(&mut self, v: Option<u8>) { self.bytes(&[v.is_some() as u8]); if let Some(x) = v { self.bytes(&[x]); } }
    fn optu32(&mut self, v: Option<u32>) { self.bytes(&[v.is_some() as u8]); if let Some(x) = v { self.i64(x as i64); } }
}

/// The per-tick state digest: everything a LATER tick reads. Field order is fixed; the FNV walk
/// is stable across builds, so a pre/post comparison is meaningful.
fn state_digest(sim: &nf_view::scene::Sim) -> u64 {
    let d = &sim.drones;
    let mut h = Fnv::new();
    h.usz(d.frame as usize);
    h.u64v(d.rng.x as u64); h.u64v(d.rng.y as u64);
    // Bond (the player's next tick reads pose, substate, the held weapon and the ammo).
    h.v3(sim.bond.pos); h.v3(sim.bond.vel); h.f32(sim.bond.yaw); h.f32(sim.bond.pitch);
    h.bytes(&[sim.bond.substate]); h.boolean(sim.bond.crouched); h.boolean(sim.bond.grounded);
    h.bytes(&[sim.bond.jump]); h.usz(sim.bond.lockout as usize);
    h.bytes(&[sim.gun.weapon]); h.usz(sim.gun.clip as usize); h.usz(sim.gun.spare as usize);
    for &x in &d.inventory.carried { h.i64(x as i64); }
    for &x in &d.inventory.clip { h.i64(x as i64); }
    for &x in &d.inventory.owned { h.boolean(x); }
    // The switch channels (triggers/scripts/doors/ss all read and write these) + their stamps.
    for &x in &d.channels.v { h.bytes(&[x]); }
    for &x in &d.channels.time { h.usz(x as usize); }
    // The drones.
    h.usz(d.list.len());
    for dr in &d.list {
        h.boolean(dr.present); h.boolean(dr.enabled);
        h.v3(dr.pos); h.f32(dr.yaw); h.f32(dr.desired_yaw);
        h.bytes(&[dr.state]); h.bytes(&[dr.prev_state]); h.optu8(dr.next_state);
        h.f32(dr.health); h.f32(dr.alert); h.f32(dr.visibility);
        h.usz(dr.seen_ticks as usize); h.usz(dr.unseen_ticks as usize);
        h.boolean(dr.los); h.boolean(dr.seen);
        h.usz(dr.weapon as usize); h.usz(dr.clip as usize);
        h.usz(dr.anim.danim as usize); h.usz(dr.anim.time as usize); h.f32(dr.anim.ftime);
        h.usz(dr.anim.state as usize); h.bytes(&[dr.anim.class]);
        h.f32(dr.aim_yaw); h.f32(dr.aim_pitch);
    }
    h.usz(d.llist.len());
    for &i in &d.llist { h.usz(i); }
    // Bullets: Bond's and the drones'.
    for b in &d.player_bullets { h.v3(b.pos); h.v3(b.dir); h.f32(b.speed); h.f32(b.travelled); h.usz(b.weapon as usize); h.boolean(b.dead); }
    for b in &d.bullets { h.v3(b.pos); h.v3(b.dir); h.f32(b.speed); h.f32(b.travelled); h.bytes(&[b.state]); h.usz(b.weapon as usize); h.usz(b.owner); }
    for g in &d.grenades { h.v3(g.pos); h.v3(g.velocity); h.f32(g.speed); h.f32(g.travelled); h.bytes(&[g.state]); }
    // Doors (state + the live pose), the AI hearing list, the audio pool's live voices.
    for dr in &d.doors.list { h.usz(dr.state as usize); h.f32(dr.pos01); h.v3(dr.pos); h.i64(dr.timer as i64); h.usz(dr.open_timer as usize); }
    h.usz(d.sounds.len());
    for s in &d.sounds { h.usz(s.id as usize); h.v3(s.pos); }
    h.usz(d.audio.live() as usize);
    for id in d.audio.active_ids() { h.usz(id as usize); }
    // Pickups, triggers, script players, SS objects, world objects, casings, gas, fx smoke.
    h.usz(d.pickups.len()); h.usz(d.taken.len());
    h.usz(d.triggers.len());
    for t in &d.triggers { h.boolean(t.gone); h.usz(t.kind as usize); h.usz(t.placement); }
    h.usz(d.scripts.players.len());
    for p in &d.scripts.players { h.usz(p.placement); h.boolean(p.deleted); h.boolean(p.activated); }
    h.usz(d.sss.list.len());
    for s in &d.sss.list { h.usz(s.state as usize); h.usz(s.mask as usize); h.usz(s.hash as usize); }
    h.usz(d.worldobjs.inverters.len() + d.worldobjs.shooters.len() + d.worldobjs.dynobjs.len() + d.worldobjs.thirdcams.len());
    h.usz(d.casings.list.len());
    h.usz(d.gas.puffs.len());
    h.usz(d.smoke_clouds.len());
    h.usz(d.scripts.objects.list.iter().filter(|o| o.is_some()).count());
    // Mission + the player-health path + the gates a later tick branches on.
    h.usz(d.mission.status() as usize);
    h.f32(d.health.health); h.f32(d.health.flash); h.f32(d.health.flashbang);
    h.boolean(d.npc_enabled); h.optu32(d.level_exit);
    h.usz(d.mission_events.len());
    for _e in &d.mission_events { /* the enum carries payloads; the count catches queueing changes */ }
    h.usz(d.player_hits.len());
    h.0
}

// ── stats + record format ─────────────────────────────────────────────────────

fn sorted(v: &[u64]) -> Vec<u64> { let mut s = v.to_vec(); s.sort_unstable(); s }

/// (median, p95, max) in milliseconds over the per-tick nanosecond values.
fn stats(v: &[u64]) -> (f64, f64, f64) {
    let s = sorted(v);
    let n = s.len();
    let med = if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2 } as f64 / 1e6;
    let p95 = s[((n as f64) * 0.95).ceil() as usize - 1] as f64 / 1e6;
    (med, p95, s[n - 1] as f64 / 1e6)
}

const RECORD_TICKNS: &str = "TICKNS";
const RECORD_PHASE: &str = "PHASE";
const RECORD_DIGEST: &str = "DIGEST";

fn write_record(path: &str, out: &str) { std::fs::write(path, out).expect("write record file"); }

/// The recorded file's per-level rows: (tick_ns, phase name → per-tick ns, digest trace).
struct Recorded {
    tick_ns: Vec<u64>,
    phases: Vec<(String, Vec<u64>)>,
    digest: Vec<u64>,
}

fn read_record(path: &str) -> HashMap<String, Recorded> {
    let text = std::fs::read_to_string(path).expect("read record file");
    let mut out: HashMap<String, Recorded> = HashMap::new();
    let mut cur: Option<(String, Recorded)> = None;
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match (it.next(), cur.is_some()) {
            (Some("LEVEL"), _) => {
                if let Some((k, v)) = cur.take() { out.insert(k, v); }
                let name = it.next().expect("LEVEL line needs a name").to_string();
                cur = Some((name, Recorded { tick_ns: Vec::new(), phases: Vec::new(), digest: Vec::new() }));
            }
            (Some(RECORD_TICKNS), true) => if let Some((_, r)) = cur.as_mut() { r.tick_ns = it.filter_map(|x| x.parse().ok()).collect(); },
            (Some(RECORD_DIGEST), true) => if let Some((_, r)) = cur.as_mut() { r.digest = it.filter_map(|x| u64::from_str_radix(x, 16).ok()).collect(); },
            (Some(RECORD_PHASE), true) => if let Some((_, r)) = cur.as_mut() {
                let name = it.next().expect("PHASE line needs a name").to_string().replace("~", " ");
                r.phases.push((name, it.filter_map(|x| x.parse().ok()).collect()));
            },
            _ => {}
        }
    }
    if let Some((k, v)) = cur.take() { out.insert(k, v); }
    out
}

// ── the probe ─────────────────────────────────────────────────────────────────

fn run_probe() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    if cfg!(debug_assertions) { eprintln!("skip: the tick-cost probe is release-only (debug builds measure the unoptimised sim)"); return }
    nf_game::prof::force_enable();
    let only: Option<Vec<String>> = std::env::var("NF_TICKCOST_ONLY").ok().map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let record = std::env::var("NF_TICKCOST_RECORD").ok();
    let check = std::env::var("NF_TICKCOST_CHECK").ok();
    let expected: Option<HashMap<String, Recorded>> = check.as_ref().map(|p| read_record(p));
    let ticks = input_script();

    let mut record_out = String::new();
    let mut check_failed = false;
    println!("| level | median | p95 | max |");
    println!("|---|---|---|---|");
    for &(name, code) in &LEVELS {
        if let Some(list) = &only { if !list.iter().any(|x| name.starts_with(x.as_str())) { continue } }
        let t_load = Instant::now();
        let mut scene = nf_view::load(&iso, name, false).unwrap_or_else(|e| panic!("load {name}: {e:#}"));
        let load_s = t_load.elapsed().as_secs_f64();
        // One-time lazy inits (route-node → door bindings, mission init) belong to tick 1 of the
        // 600 — the median absorbs them; the max column says so in the report.
        nf_game::prof::reset();
        nf_game::prof::capture(true);
        let mut tick_ns: Vec<u64> = Vec::with_capacity(TICKS);
        let mut digest: Vec<u64> = Vec::with_capacity(TICKS + 1);
        let mut phase_acc: Vec<(&'static str, Vec<u64>)> = Vec::new();
        let sim = scene.sim.as_mut().expect("no sim");
        digest.push(state_digest(sim));
        let rng_start = (sim.drones.rng.x, sim.drones.rng.y);
        for t in &ticks {
            let (input, trigger, reload, use_key) = t.to_parts();
            let sim = scene.sim.as_mut().expect("no sim");
            let t0 = Instant::now();
            sim.step_bond(input, trigger, reload, use_key);
            tick_ns.push(t0.elapsed().as_nanos() as u64);
            // Slice this tick's scope samples into the per-phase accumulators.
            let mut sums: HashMap<&'static str, u64> = HashMap::new();
            for (n, ns) in nf_game::prof::take_samples() { *sums.entry(n).or_insert(0) += ns; }
            for (n, ns) in sums {
                match phase_acc.iter_mut().find(|(k, _)| *k == n) {
                    Some((_, v)) => v.push(ns),
                    None => phase_acc.push((n, vec![ns])),
                }
            }
            let sim = scene.sim.as_ref().expect("no sim");
            digest.push(state_digest(sim));
        }
        nf_game::prof::capture(false);
        let (med, p95, max) = stats(&tick_ns);
        println!("| {name} | {med:.2} | {p95:.2} | {max:.2} | (load {load_s:.1} s)");
        // The phase table: median ms/tick per scope, sorted by median, nestees marked.
        let mut rows: Vec<(&str, f64, f64)> = phase_acc.iter()
            .map(|(k, v)| { let (m, _, x) = stats(v); (*k, m, x) }).collect();
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        eprintln!("[tickcost] {name} phases (median ms/tick, max):");
        for (k, m, x) in &rows { eprintln!("  {k:<34} {m:7.3}  max {x:7.3}"); }
        // Record / check.
        if let Some(path) = &record {
            record_out.push_str(&format!("LEVEL {name} {code}\n"));
            record_out.push_str(&format!("{RECORD_TICKNS} {}\n", tick_ns.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")));
            for (k, v) in &phase_acc { record_out.push_str(&format!("{RECORD_PHASE} {} {}\n", k.replace(' ', "~"), v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "))); }
            record_out.push_str(&format!("{RECORD_DIGEST} {}\n", digest.iter().map(|d| format!("{d:016x}")).collect::<Vec<_>>().join(" ")));
        }
        if let Some(exp) = &expected {
            let Some(er) = exp.get(name) else { panic!("check: {} has no recorded baseline", name) };
            if er.digest.len() != digest.len() {
                println!("HASH MISMATCH {name}: digest trace length {} vs recorded {}", digest.len(), er.digest.len());
                check_failed = true;
            } else if let Some(i) = digest.iter().zip(er.digest.iter()).position(|(a, b)| a != b) {
                println!("HASH MISMATCH {name}: first divergent tick {i}: live {:016x} vs recorded {:016x}", digest[i], er.digest[i]);
                check_failed = true;
            } else {
                println!("hash ok: {name} ({} ticks identical)", digest.len() - 1);
            }
        }
        // Always-on sanity (each can fail): the sim actually ran 600 ticks, the RNG advanced,
        // and the digest is not vacuous.
        let sim = scene.sim.as_ref().expect("no sim");
        assert_eq!(sim.drones.frame, TICKS as u32, "{name}: the sim did not run {TICKS} ticks");
        let first = digest[0]; let last = digest[digest.len() - 1];
        assert!(last != first || digest.iter().any(|&d| d != first), "{name}: the digest never changed — it cannot detect a behaviour change");
        // The starting RNG (tick 0) must have advanced over 600 ticks of moving + firing.
        assert!(rng_start != (sim.drones.rng.x, sim.drones.rng.y), "{name}: the shared RNG did not advance — the replay did not actually run");
    }
    if let Some(path) = &record { write_record(path, &record_out); eprintln!("[tickcost] record written to {path}"); }
    if check_failed { panic!("the per-tick state hash diverged from the recorded baseline — see HASH MISMATCH above"); }
}

fn iso() -> Option<String> {
    std::env::var("NF_ISO").ok().filter(|p| std::path::Path::new(p).exists())
}

/// The main probe: every single-player level, 600 ticks, timings + phase table + the digest.
#[test]
fn tick_cost_per_level() { run_probe() }

/// Always-on determinism: two FRESH loads of 07000005 driven through the same 300-tick script
/// produce identical per-tick digests. (The recorded baseline check in `run_probe` is the
/// before/after bar; this catches in-process nondeterminism on any tree.)
#[test]
fn two_runs_same_digests() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    if cfg!(debug_assertions) { eprintln!("skip: release-only"); return }
    nf_game::prof::force_enable();
    nf_game::prof::capture(false); // the digest only; no sample capture here
    let ticks: Vec<TickInput> = input_script().into_iter().take(300).collect();
    let mut digests: Vec<Vec<u64>> = Vec::new();
    for run in 0..2 {
        let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap_or_else(|e| panic!("load run {run}: {e:#}"));
        let mut trace = Vec::with_capacity(ticks.len() + 1);
        trace.push(state_digest(scene.sim.as_ref().unwrap()));
        for t in &ticks {
            let (input, trigger, reload, use_key) = t.to_parts();
            scene.sim.as_mut().unwrap().step_bond(input, trigger, reload, use_key);
            trace.push(state_digest(scene.sim.as_ref().unwrap()));
        }
        digests.push(trace);
    }
    let (a, b) = (&digests[0], &digests[1]);
    assert_eq!(a.len(), b.len(), "trace lengths differ");
    if let Some(i) = a.iter().zip(b.iter()).position(|(x, y)| x != y) {
        panic!("DIVERGENCE at tick {i}: run A {:016x} vs run B {:016x}", a[i], b[i]);
    }
    let first = a[0]; let last = a[a.len() - 1];
    assert_ne!(first, last, "the digest never changed over 300 ticks — the sim did not run");
}
