//! fix-ai-traces: the guards against the real game's recordings (tests/traces/*.trace).
//!
//! Replays Bond's recorded path through the port's sim (`nf_view::load` + `Sim::step_bond` with Bond
//! placed per frame from the trace, as fix-truck did), and compares every drone's state id (+0x10C)
//! and position (obj+0x30) with the recording, frame by frame, in DRONE_LIST order (`Drones::llist`).
//! The recorder (`tools/trace/trace_record.py`) reads each line when the in-game frame counter
//! (GameState+0x34) changes, i.e. the state at the START of that frame = the end of the previous
//! frame's update; so tick t of the replay (1-based) corresponds to trace line t−1, and during the
//! game's frame the drones saw Bond converge onto the recorded position — the harness sets Bond to
//! the CURRENT trace line's pos/yaw before the step and compares the post-step state to that line.
//!
//! Pad bits (docs/systems/input.md §2/§3, style 0): R1 0x0008 = action 9 fire (the `trigger`
//! parameter), L1 0x0004 = action 19 aim mode, ✕ 0x0040 = action 8 crouch (a press toggles),
//! △ 0x0010 = action 7 jump, □ 0x0080 = action 14 reload, ○ 0x0020 = action 12 use. The recorder's
//! cheats (`--god` refill health 100/armour 50 every frame; the clip topping) are mirrored by
//! `Dev::after_tick` like fix-pickups-trace did.
//!
//! Opt-in via NF_ISO. `NF_AI_FRAMES` caps the replay length for quick iterations.

use nf_view::dev::Dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The repo's traces dir (the test runs from the crate dir).
fn trace_path(name: &str) -> String { format!("{}/../../tests/traces/{name}", env!("CARGO_MANIFEST_DIR")) }

struct TFrame {
    frame: u32,
    pad: u16,
    pos: [f32; 3],
    yaw: f32,
    /// (state id = obj+0x10C, obj+0x30) in DRONE_LIST walk order.
    drones: Vec<(u16, [f32; 3])>,
}

/// Parse the trace's data lines (skips `#` headers, the `ch0` dump and the change-event lines
/// (`hit`/`inv0`/`spare`/`weap`/`ch`); those are consulted by the specific fixes that need them).
/// The recorder keeps recording across a level load, so the frames are split into per-level segments.
fn parse_trace(path: &str) -> Vec<(u32, Vec<TFrame>)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut level = 0u32;
    let mut segs: Vec<(u32, Vec<TFrame>)> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# nightfire trace v1 level=") {
            level = u32::from_str_radix(rest.split(' ').next().unwrap(), 16).unwrap();
            continue;
        }
        if line.starts_with('#') || line.starts_with("ch0 ") { continue; }
        let f = line.split(' ').collect::<Vec<_>>();
        if f.len() < 14 || f[13] != "|" { continue; } // data rows only (13 tokens before the first `|`)
        let frame: u32 = f[0].parse().unwrap();
        let lv = u32::from_str_radix(f[1], 16).unwrap();
        if lv != level {
            level = lv;
            segs.push((lv, Vec::new()));
        }
        let pad = u16::from_str_radix(f[2], 16).unwrap();
        let pos = [f[7].parse().unwrap(), f[8].parse().unwrap(), f[9].parse().unwrap()];
        let yaw: f32 = f[10].parse().unwrap();
        // After the first `|`: drone tuples `state x y z` (optional `a<val>` alert suffix skipped),
        // then a second `|` and the doors — stop at it.
        let mut drones = Vec::new();
        let mut k = 14;
        while k + 3 < f.len() && f[k] != "|" {
            if f[k].starts_with('a') && f[k].len() > 1 && f[k][1..].parse::<f32>().is_ok() { k += 1; continue; }
            let st: u16 = f[k].parse().unwrap();
            let p = [f[k + 1].parse().unwrap(), f[k + 2].parse().unwrap(), f[k + 3].parse().unwrap()];
            drones.push((st, p));
            k += 4;
        }
        if segs.last().is_none_or(|(l, _)| *l != lv) { segs.push((lv, Vec::new())); }
        segs.last_mut().unwrap().1.push(TFrame { frame, pad, pos, yaw, drones });
    }
    segs
}

/// One replay + comparison. Returns, per drone index (llist order), the number of leading frames
/// whose (state, pos) matched the recording, and the first divergence's description.
struct Diverge {
    matched: usize,
    kind: &'static str,
    at_frame: u32,
    detail: String,
}

const TOL: f32 = 0.05; // x/z: the game's own idle sway jitters ~0.034 against the trace's 3 decimals
/// The converted y carries the hip-conversion error: the game's `DroneAnim_PreTransCallback` (0x13b490)
/// reads the hip from `animobj+0x5c`, which is NOT the anim root track's y the port's converter uses —
/// the [L] idle residuals are 0.023–0.033 and a walking drone's 0.102, with x/z exact to 0.001 and
/// every state matching, so the gap is the converter's, not AI drift. (Live check in the report: watch
/// `animobj+0x5c` against the root track to pin the exact source.)
const YTOL: f32 = 0.15;

/// The game's obj+0x30.y for a drone, from the port's convention (`pos[1]` = floor + 1.0): the game's
/// `DroneAnim_PreTransCallback` (0x13b490) adds the hip-height delta (`animobj+0x5c` − 0.02, min 0.2 =
/// `Drone+0xA0`'s update) to the root vector's y each tick, so the live object y = feet + `Drone+0xA0`
/// while an anim plays; the placement puts the object at feet + 1.0 (= the init `+0xA0`). The port keeps
/// its own `pos[1]` at the placement convention and re-adds the hip only at the draw (`draw_y`), so the
/// comparison converts: y_game = pos[1] − 1.0 + (root_y − 0.02).max(0.2) while a script plays (root y
/// from the current script frame, as `Drone::root_value` reads it); without an anim (and for the free
/// root of special deaths, which moves the object y directly) y_game = pos[1].
/// `lift`: whether the game's anim is ticking for this drone (a disabled/waiting drone's anim object
/// does not update — its obj y stays at the placement's feet + 1.0; the [L] proof is the trace's
/// WAIT_SWITCH rows, which match the port's raw y exactly for 600 frames).
fn game_obj_y(sim: &nf_view::scene::Sim, di: usize, lift: bool) -> f32 {
    let d = &sim.drones.list[sim.drones.llist[di]];
    if d.free_root || !lift { return d.pos[1]; }
    let Some(script) = d.anim.script.and_then(|s| sim.scripts.get(&s)) else { return d.pos[1] };
    let t = (1 + d.anim.time) as u16;
    let (_, _, q) = match script.sequences().find(|(a, b, _)| *a <= t && t <= *b) { Some(x) => x, None => return d.pos[1] };
    let Some(seq) = sim.seqs.get(&q) else { return d.pos[1] };
    if seq.channels.len() < 3 { return d.pos[1]; }
    let f = (t as u32).clamp(1, seq.frames.max(1) as u32);
    let root_y = seq.value(1, f);
    if root_y == 0.0 { return d.pos[1]; }
    d.pos[1] - 1.0 + nf_game::damage::drone_height(root_y)
}

fn replay(level_file: &str, level: u32, frames: &[TFrame], limit: usize, fire_enabled: bool) -> Vec<Diverge> {
    let mut scene = nf_view::load(iso().as_deref().unwrap(), level_file, false).expect("load");
    // 07000001's level-start intro takes control; skip it the game's way (fix-pickups-trace).
    let intro: &[u32] = match level { 0x0700_0001 => &[0x0600_0087], _ => &[] };
    if !intro.is_empty() {
        for t in 0..600u32 {
            let sim = scene.sim.as_mut().unwrap();
            let pending = sim.drones.scripts.players.iter().any(|p| intro.contains(&p.scripts[0]) && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
            if t >= 2 && !pending && !sim.disabled { break; }
            sim.drones.scripts.skip_request = true;
            sim.step_bond(Default::default(), false, false, false);
        }
    }
    let mut dev = Dev { god_mode: true, infinite_ammo: true, ..Default::default() };
    if std::env::var_os("NF_AI_DEBUG").is_some() {
        let sim = scene.sim.as_ref().unwrap();
        for (ix, d) in sim.drones.list.iter().enumerate() {
            eprintln!("[dbg] list{ix}: state {} enabled {} present {} drawn {} in_llist {} channel {:#x} mode {} dtype {} ret {} alt_mode {} pos {:?}", d.state, d.enabled, d.present, d.drawn, sim.drones.llist.contains(&ix), d.switch_channel, d.mode, d.dtype, d.return_state, d.alt_mode, d.pos);
        }
    }
    let n = frames.len().min(limit);
    // The recorder starts mid-run: the level's frame counter (GameState+0x34) is already at 29322+ on
    // line 0, and the game's drones show their post-spawn states (the 07000005 sniper is already in
    // SNIPER_IDLE 0x29 — the port reaches it on tick 2, after the group-1 spawner's immediate respawn
    // [L ee_start_05: the live sniper's group +0x144 = 1, count 0xffff = DroneSpawner_Init 0x137aa0 ran
    // and the spawner re-created it]). So find the warm-up K: the number of ticks the port must run
    // (Bond held at line 0's position) before its drone states match line 0. The smallest K with the
    // best state match wins; the RNG (whose draws ride those same ticks) stays in game order.
    let first = &frames[0];
    let mut k = 0usize;
    {
        let mut best: Option<(usize, usize)> = None; // (misses, k)
        for kk in 0..12usize {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond.pos = first.pos;
            sim.bond.yaw = first.yaw;
            sim.step_bond(Default::default(), false, false, false);
            let mut misses = 0usize;
            for (i, (st, _)) in first.drones.iter().enumerate() {
                match sim.drones.llist.get(i).map(|&ix| &sim.drones.list[ix]) {
                    Some(pd) if pd.state as u16 == *st => {}
                    _ => misses += 1,
                }
            }
            if best.map_or(true, |(bm, _)| misses < bm) { best = Some((misses, kk + 1)); }
            if misses == 0 { break; }
        }
        if let Some((_, kk)) = best { k = kk; }
        eprintln!("[{level_file}] warm-up K = {k}");
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = first.pos;
        sim.bond.yaw = first.yaw;
    }
    let mut per_drone: Vec<Diverge> = (0..frames.iter().map(|f| f.drones.len()).max().unwrap_or(0))
        .map(|_| Diverge { matched: usize::MAX, kind: "?", at_frame: 0, detail: String::new() })
        .collect();
    let mut prev_pad = 0u16;
    let mut prev_pos = frames.first().map(|f| f.pos).unwrap_or([0.0; 3]);
    for t in 0..n {
        let f = &frames[t];
        if t > 0 {
            {
                let sim = scene.sim.as_mut().unwrap();
                // Bond placed per frame: the game's drones saw Bond converge onto the recorded end-of-frame
                // position during the frame (the player updates before the drones tick). Velocity follows the
                // recorded delta (the moved-detection `+0x1DC` and the footstep noise read it).
                sim.bond.pos = f.pos;
                sim.bond.yaw = f.yaw;
                let dt = 1.0 / 60.0;
                sim.bond.vel = [(f.pos[0] - prev_pos[0]) / dt, (f.pos[1] - prev_pos[1]) / dt, (f.pos[2] - prev_pos[2]) / dt];
                if f.pos[1] < prev_pos[1] - 0.4 || f.pos[1] > prev_pos[1] + 0.4 { sim.bond.grounded = false; }
                if f.pad & 0x0040 != 0 && prev_pad & 0x0040 == 0 { sim.bond.crouched = !sim.bond.crouched; }
            }
            // Pad → the real input path (docs/systems/input.md §3 style 0).
            let input = nf_game::player::Input {
                aiming: f.pad & 0x0004 != 0,
                jump: f.pad & 0x0010 != 0 && prev_pad & 0x0010 == 0,
                ..Default::default()
            };
            let trigger = fire_enabled && f.pad & 0x0008 != 0;
            let reload = f.pad & 0x0080 != 0;
            let use_key = f.pad & 0x0020 != 0;
            let sim = scene.sim.as_mut().unwrap();
            sim.step_bond(input, trigger, reload, use_key);
            drop(sim);
        }
        dev.after_tick(&mut scene);
        prev_pos = f.pos;
        prev_pad = f.pad;
        // Compare the post-step state with this trace line.
        let sim = scene.sim.as_mut().unwrap();
        // The user's kills are not reproducible from the trace alone (Bond's pitch is not recorded, so
        // the port's Bond cannot hit what the user hit): when the game's list loses a drone and the
        // rest aligns shifted by one, force-remove the port's drone at the shift (LList_Remove after
        // DEAD/FADE [M Drone_InitComms corpses]) so the OTHER drones' AI keeps being compared. The
        // forced removal is reported as the drone's first divergence.
        if sim.drones.llist.len() > f.drones.len() {
            // The game's row = the port's llist with the drones the game LList_Removed dropped: rows
            // align 1:1 up to the first removal, then +1. Find the first mismatch m and verify the tail
            // aligns shifted.
            let sim_ = &*sim;
            let row_matches = |ix: usize, row: (u16, [f32; 3])| {
                let pd = &sim_.drones.list[sim_.drones.llist[ix]];
                pd.state as u16 == row.0 && (pd.pos[0] - row.1[0]).abs() < 0.5 && (pd.pos[2] - row.1[2]).abs() < 0.5
            };
            let mut m = None;
            for (i, row) in f.drones.iter().enumerate() {
                if i >= sim.drones.llist.len() || !row_matches(i, *row) { m = Some(i); break; }
            }
            let mut removed_at: Option<usize> = None;
            if let Some(m) = m {
                let ok = f.drones.iter().enumerate().skip(m).all(|(i, row)| i + 1 < sim.drones.llist.len() && row_matches(i + 1, *row));
                if ok && f.drones.len() + (sim.drones.llist.len() - f.drones.len()) >= m + 1 { removed_at = Some(m); }
            }
            if let Some(r) = removed_at {
                // The game removed the port's llist[r]: everything before r matched 1:1, the tail aligns
                // at +1. Remove that port drone for the rest of the segment.
                let ix = sim.drones.llist.remove(r);
                let pd_state = sim.drones.list[ix].state;
                sim.drones.list[ix].present = false;
                sim.drones.list[ix].enabled = false;
                if let Some(d) = per_drone.get_mut(r) {
                    if d.matched == usize::MAX {
                        d.matched = t; d.kind = "user-kill"; d.at_frame = f.frame;
                        d.detail = format!("the game removed this drone (the user's kill; the port left it in state {} — Bond's shots cannot be reproduced from the trace, no pitch recorded); forced out to keep the others comparable", pd_state);
                    }
                }
                eprintln!("[{level_file}] line {t} (frame {}): the game's list lost a drone (user kill) — port llist[{r}] forced out", f.frame);
            }
        }
        for (i, (st, p)) in f.drones.iter().enumerate() {
            let d = &mut per_drone[i];
            if d.matched != usize::MAX { continue; }
            let Some(pd) = sim.drones.llist.get(i).and_then(|&ix| sim.drones.list.get(ix)) else {
                d.matched = t; d.kind = "missing"; d.at_frame = f.frame;
                d.detail = format!("the port has no drone {i} (list len {})", sim.drones.list.len());
                continue;
            };
            let st_match = pd.state as u16 == *st;
            // The game's hip lift starts with its anim (an enabled drone); WAIT_SWITCH/GLOBAL rows sit
            // at the placement height (see game_obj_y).
            let lift = *st != 0 && *st != 1;
            let py = game_obj_y(sim, i, lift);
            let dp = [pd.pos[0] - p[0], py - p[1], pd.pos[2] - p[2]];
            let dmax = dp.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let dxz = dp[0].abs().max(dp[2].abs());
            if st_match && dxz <= TOL && dmax <= YTOL { continue; }
            d.matched = t; d.at_frame = f.frame;
            if !st_match {
                d.kind = "state";
                d.detail = format!("state {} vs game {}", pd.state, st);
            } else {
                d.kind = "pos";
                d.detail = format!("pos ({:.3},{:.3},{:.3}) vs game ({:.3},{:.3},{:.3}) dmax {:.3} state {}", pd.pos[0], py, pd.pos[2], p[0], p[1], p[2], dmax, pd.state);
            }
        }
    }
    for d in per_drone.iter_mut() {
        if d.matched == usize::MAX { d.matched = n; d.kind = "full"; }
    }
    if std::env::var_os("NF_AI_DEBUG").is_some() {
        let sim = scene.sim.as_ref().unwrap();
        for (ix, d) in sim.drones.list.iter().enumerate() {
            if !sim.drones.llist.contains(&ix) && !d.present { continue; }
            eprintln!("[dbg-end] list{ix}: state {} enabled {} present {} drawn {} in_llist {} spawn_frame {} channel {:#x} mode {} pos {:?}", d.state, d.enabled, d.present, d.drawn, sim.drones.llist.contains(&ix), d.spawn_frame, d.switch_channel, d.mode, d.pos);
        }
    }
    per_drone
}

fn report(name: &str, level_file: &str, trace: &str, limit: usize, fire: bool) -> Vec<Vec<Diverge>> {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return Vec::new() };
    let segs = parse_trace(&trace_path(trace));
    let mut out = Vec::new();
    for (si, (level, frames)) in segs.iter().enumerate() {
        let file = format!("{level:08x}.bin");
        eprintln!("[{name}] segment {si}: level {level:08x} ({file}), {} trace frames ({}..{}), replaying {} (fire {fire})",
            frames.len(), frames.first().map(|f| f.frame).unwrap_or(0), frames.last().map(|f| f.frame).unwrap_or(0), limit);
        let per = replay(&file, *level, frames, limit, fire);
        let mut first_global = usize::MAX;
        for (i, d) in per.iter().enumerate() {
            if d.kind == "full" { eprintln!("[{name}] seg{si} drone {i:2}: matched ALL {} frames", d.matched); continue; }
            first_global = first_global.min(d.matched);
            eprintln!("[{name}] seg{si} drone {i:2}: first divergence at trace line {} (frame {}): {} — {}", d.matched, d.at_frame, d.kind, d.detail);
        }
        eprintln!("[{name}] seg{si} first global divergence: trace line {}", if first_global == usize::MAX { per.first().map(|d| d.matched).unwrap_or(0) } else { first_global });
        out.push(per);
    }
    out
}

/// Pins (fix-ai-traces): per trace, each drone's matched-frames count can only go UP. `full` = every
/// frame of the segment matched. The pinned numbers come from the tables in docs/atlas/drone-port-check.md.
fn pin(seg: &[Diverge], expect: &[(usize, usize, &str)]) {
    for (i, want, kind) in expect {
        let Some(d) = seg.get(*i) else { panic!("drone {i} missing from the table") };
        let got = if d.kind == "full" { usize::MAX } else { d.matched };
        assert!(got >= *want, "drone {i}: matched {got} frames, pinned at least {want} (kind {}, detail {})", d.kind, d.detail);
        if *kind != "" { assert_eq!(d.kind, *kind, "drone {i}: kind {} ({})", d.kind, d.detail); }
    }
}

#[test]
fn ai_05_truck() {
    let limit = std::env::var("NF_AI_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let segs = report("05-truck", "07000005.bin", "05-truck.trace", limit, true);
    if segs.is_empty() { return; }
    // 07000005: drone 2 matched the WHOLE level (7851 frames); the others held until the user's first
    // kill (llist removal at line 885/2168) or the hip-conversion noise; the sniper (12) is state-matched
    // from line 0 (the group-1 spawner's immediate respawn).
    pin(&segs[0], &[(2, 7851, "full"), (0, 1342, ""), (12, 0, ""), (13, 7049, "")]);
    for i in 4..=10 { pin(&segs[0], &[(i, 2168, "")]); }
}

#[test]
fn ai_01_doors() {
    let limit = std::env::var("NF_AI_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let segs = report("01-doors", "07000001.bin", "01-doors.trace", limit, true);
    if segs.is_empty() { return; }
    // The PlayScript fix: the first guard's state matches the game from line 0 (its first divergence is
    // the script's root-motion step, a pos one, never the state).
    pin(&segs[0], &[(0, 0, "pos"), (1, 0, "state"), (15, 63, ""), (16, 64, "")]);
}

#[test]
fn ai_01_raptor() {
    let limit = std::env::var("NF_AI_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let segs = report("01-raptor", "07000001.bin", "01-raptor.trace", limit, true);
    if segs.is_empty() { return; }
    pin(&segs[0], &[(0, 0, "pos"), (1, 0, "state"), (15, 63, ""), (16, 64, "")]);
}

#[test]
fn ai_02_civilian() {
    let limit = std::env::var("NF_AI_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let segs = report("02-civilian", "07000002.bin", "02-civilian.trace", limit, true);
    if segs.is_empty() { return; }
    // The civilian level replays PERFECTLY: 24 of 25 drones match all 1083 frames; the ally (21, the
    // ALLY_LEAD escort) matches 295 before its pos drifts past the hip-conversion tolerance.
    let n = segs[0].len();
    for i in 0..n { if i != 21 { pin(&segs[0], &[(i, 1083, "full")]); } }
    pin(&segs[0], &[(21, 295, "pos")]);
}
