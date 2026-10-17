//! walkreal-a probe 2: 07000002's secret-bunker spawn room. Evidence dump + route search:
//!   1. EVERY kind-0x24 placement of the level (all maps), and which start `Player_Start` (0x190fd0)
//!      picks cold vs. warm (07000001's exit leaves ch 0x10 on — its LOAD_LEVEL #18 has gate 0x10 and
//!      no success/fallback channels [docs/levels/07000001.md]).
//!   2. The room's floor stack and a capsule probe grid at Bond's real capsule (standing: hip+0.275 ..
//!      hip−0.485, r 0.55, hip = floor + 1.1 — `Bond::capsule`), built exactly like scene.rs
//!      (`CollisionWorld::build(level, &level.instances(true).0)`).
//!   3. The sim's ladders (centre/radius/yaw/top/bottom), doors, triggers and pickups around the room,
//!      and for every ladder the standing cells inside its grab window.

use nf_formats::iso9660::IsoImage;
use nf_game::player::{pick_start, LevelStart, RamSave};
use nf_game::triggers::Channels;
use nf_game::world::{CollisionWorld, V3};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load_level(name: &str) -> nf_formats::level::Level {
    let img = IsoImage::open(&iso()).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("no {name}"));
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    nf_formats::level::Level::parse(&data).expect("parse")
}

fn dxz(a: V3, b: V3) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// Floor levels top-down at (x, z): repeated `floor_below` rays (each restarts just under the last hit).
fn floor_stack(world: &CollisionWorld, x: f32, z: f32, top: f32, depth: f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut y = top;
    for _ in 0..8 {
        let Some(f) = world.floor_below([x, y, z], depth) else { break };
        if out.last().is_some_and(|&p: &f32| (p - f).abs() < 0.05) { break; }
        out.push(f);
        y = f - 0.6;
    }
    out
}

/// Bond's standing capsule at hip height `hip` (hip = floor + 1.1): the segment `Bond::capsule` uses.
fn capsule_at(hip: f32) -> (V3, V3) { ([0.0, hip + 0.275, 0.0], [0.0, hip - 0.485, 0.0]) }

/// Whether a standing capsule at (x, hip, z) is pushed (blocked) — the probe grid's cell test.
fn cell_blocked(world: &CollisionWorld, x: f32, hip: f32, z: f32) -> bool {
    let (a, b) = capsule_at(hip);
    let (push, _) = world.capsule_push([x, a[1], z], [x, b[1], z], 0.55);
    push.iter().any(|v| v.abs() > 0.05)
}

// ── 1. The starts ────────────────────────────────────────────────────────────────────────────────

#[test]
fn probe2_starts() {
    let level = load_level("07000002.bin");
    let main = level.main_map();
    eprintln!("== 07000002: {} maps, main map = {:?}", level.maps.len(), main);
    eprintln!("== every kind-0x24 placement (all maps) ==");
    for (mi, m) in level.maps.iter().enumerate() {
        for (pi, p) in m.placements.iter().enumerate() {
            if p.flags & 0xff != 0x24 { continue; }
            eprintln!(
                "map {mi}{} placement #{pi} at ({:.3},{:.3},{:.3}) yaw {:.3} flags {:#x} params {:?} hash {:#x}",
                if Some(mi) == main { " [MAIN]" } else { "" },
                p.translation[0], p.translation[1], p.translation[2], p.rotation[1], p.flags, p.params, p.hashcode,
            );
        }
    }
    // `level_starts` (nf-view lib.rs): the MAIN map's start placements — this probe rebuilds the
    // kind-1 (0x24) set only; the full registration incl. the 0x2d kind-0 set is start_slot_0x2d's.
    let starts: Vec<LevelStart> = main.map(|m| {
        level.maps[m].placements.iter()
            .filter(|p| p.flags & 0xff == 0x24)
            .map(|p| LevelStart {
                pos: p.translation,
                yaw: p.rotation[1],
                gate: p.params.iter().find(|(i, _)| *i == 0).map_or(0, |&(_, v)| v as u16),
                p3: 0, // fix-world-gravity's LevelStart.p3 (this test's starts are not the load path)
                kind: 1, // type 0x24 registers as kind 1 [M 0x1d04f0 case 0x24, disasm 0x1d0a00]
            })
            .collect()
    }).unwrap_or_default();
    eprintln!("== level_starts (main map only): {} entries ==", starts.len());
    for (i, s) in starts.iter().enumerate() {
        eprintln!("  start[{i}] pos {:?} yaw {:.3} gate {:#x}", s.pos, s.yaw, s.gate);
    }
    let cold = Channels::default();
    eprintln!("pick_start COLD (no channels)          -> {:?}",
        pick_start(&starts, &cold).map(|i| (i, &starts[i])));
    let mut warm = Channels::default();
    warm.set(0x10, 1, 1); // 07000001's Touch #180 "LoadPointActivator" out 0x10 stays on [docs]
    eprintln!("pick_start WARM (ch 0x10 on)           -> {:?}",
        pick_start(&starts, &warm).map(|i| (i, &starts[i])));
    for g in [0x10u16, 0x11, 0x12, 0x35, 0x47, 0x66, 0x81] {
        let mut ch = Channels::default();
        ch.set(g as u32, 1, 1);
        if let Some(i) = pick_start(&starts, &ch) {
            if starts[i].gate != 0 { eprintln!("pick_start with ONLY ch {g:#x} on -> start[{i}] {:?}", starts[i]); }
        }
    }

    // The real loads: cold (nf_view::load) and warm (load_with_carry with ch 0x10 on).
    let scene = nf_view::load(&iso(), "07000002.bin", false).expect("cold load");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("COLD load bond.pos = {:?} yaw {:.3} grounded {} disabled {}", sim.bond.pos, sim.bond.yaw, sim.bond.grounded, sim.disabled);
    let mut save = RamSave::default();
    save.health = 100.0;
    save.channels = vec![0u8; 256];
    save.channels[0x10] = 1;
    save.ch_time = vec![0u32; 256];
    let scene = nf_view::load_with_carry(&iso(), "07000002.bin", false, Some(&save)).expect("warm load");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("WARM load (ch 0x10 carried) bond.pos = {:?} yaw {:.3} grounded {}", sim.bond.pos, sim.bond.yaw, sim.bond.grounded);
    let f = sim.world.floor_below(sim.bond.pos, 4.0);
    eprintln!("WARM floor below spawn = {f:?}");

    // The gate writer: 07000001's LOAD_LEVEL (0xE8) placements. Params [M Trigger_Activate 0x1b2318 /
    // triggers.rs build]: p0 = next level, p1 = key gate, p2 = gate, p3 = fallback, p4 = success.
    let level01 = load_level("07000001.bin");
    for (mi, m) in level01.maps.iter().enumerate() {
        for (pi, p) in m.placements.iter().enumerate() {
            if p.flags & 0xff != 0xe8 { continue; }
            eprintln!("07000001 map {mi} placement #{pi} kind 0xE8 LOAD_LEVEL at ({:.2},{:.2},{:.2}) params {:?}",
                p.translation[0], p.translation[1], p.translation[2], p.params);
        }
    }
}

// ── 2. The room's geometry: floor stack + capsule grid ──────────────────────────────────────────

#[test]
fn probe2_room_capsules() {
    let level = load_level("07000002.bin");
    let world = CollisionWorld::build(&level, &level.instances(true).0);

    // Named points first: the spawn, the four known ladders' xz.
    let named: &[(&str, f32, f32)] = &[
        ("spawn", -27.17, 29.80),
        ("ladder S (bunker?)", -26.9, 42.2),
        ("ladder dock", -15.3, 35.0),
        ("ladder hall", -50.4, 5.5),
        ("ladder west", -74.8, 22.2),
        ("shaft E (−26,42.5)", -26.0, 42.5),
        ("shaft E2 (−25,42.5)", -25.0, 42.5),
        ("trig#341 (−25,42)", -25.0, 42.0),
        ("S of shaft (−26,44)", -26.0, 44.0),
        ("SE ledge (−24,43)", -24.0, 43.0),
        ("W of shaft (−28.5,43)", -28.5, 43.0),
        ("SW ledge (−30,44.5)", -30.0, 44.5),
        ("corridor mid (−27.2,37)", -27.2, 37.0),
        ("pocket N (−27,30)", -27.0, 30.0),
        ("grab snap (−26.87,41.73)", -26.87, 41.73),
        ("lunge path (−26.9,42.6)", -26.9, 42.6),
        ("lunge path (−26.9,42.8)", -26.9, 42.8),
        ("lunge path (−26.9,43.0)", -26.9, 43.0),
        ("lunge path (−26.9,43.2)", -26.9, 43.2),
        ("lunge path (−26.9,43.5)", -26.9, 43.5),
        ("garden W (−27.3,43)", -27.3, 43.0),
        ("garden E (−26.5,43)", -26.5, 43.0),
        ("garden E2 (−26,43)", -26.0, 43.0),
        ("garden S (−26.9,44)", -26.9, 44.0),
        ("garden SSE (−25.5,44.5)", -25.5, 44.5),
    ];
    eprintln!("== floor stacks (top 8.0, 14 deep) ==");
    for (n, x, z) in named {
        eprintln!("{n:20} ({x},{z}): {:?}", floor_stack(&world, *x, *z, 8.0, 14.0));
    }

    // The grid: x −31..−9, z 23..45, 0.5 steps. One ASCII map per distinct floor level.
    let (x0, x1, z0, z1, st) = (-31.0f32, -9.0f32, 23.0f32, 45.0f32, 0.5f32);
    let cols = ((x1 - x0) / st) as usize + 1;
    let rows = ((z1 - z0) / st) as usize + 1;
    let mut levels: Vec<f32> = Vec::new();
    let mut stacks: Vec<Vec<Vec<f32>>> = Vec::new();
    for r in 0..rows {
        let mut row = Vec::new();
        for c in 0..cols {
            let s = floor_stack(&world, x0 + c as f32 * st, z0 + r as f32 * st, 8.0, 14.0);
            for &f in &s {
                let f = (f * 4.0).round() / 4.0;
                if !levels.iter().any(|l| (*l - f).abs() < 0.13) { levels.push(f); }
            }
            row.push(s);
        }
        stacks.push(row);
    }
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("== distinct floor levels under the grid: {levels:?} ==");
    for &lv in &levels {
        eprintln!("-- floor y = {lv:.2} ('.' free capsule, '#' blocked, ' ' no floor) x {x0}..{x1} step {st}, z rows {z0}..{z1} --");
        for r in 0..rows {
            let z = z0 + r as f32 * st;
            let mut row = String::new();
            for c in 0..cols {
                let s = &stacks[r][c];
                let hit = s.iter().find(|f| ((**f * 4.0).round() / 4.0 - lv).abs() < 0.13);
                row.push(match hit {
                    None => ' ',
                    Some(&f) => if cell_blocked(&world, x0 + c as f32 * st, f + 1.1, z) { '#' } else { '.' },
                });
            }
            eprintln!("z={z:5.1}: {row}");
        }
    }
}

// ── 3b. The south corridor's push field: what does the capsule feel along x −28..−25.5, z 29..43?

#[test]
fn probe2_corridor_push() {
    let level = load_level("07000002.bin");
    let world = CollisionWorld::build(&level, &level.instances(true).0);
    let f = -0.049f32;
    eprintln!("== push magnitude (deciles, 0 = free .. 9 = push >= 0.45) at floor {f}, x −28.25..−25.5 step 0.25, z 29..43 step 0.25 ==");
    let mut z = 29.0f32;
    while z <= 43.0 {
        let mut row = String::new();
        let mut x = -28.25f32;
        while x <= -25.5 {
            let (a, b) = capsule_at(f + 1.1);
            let (push, _) = world.capsule_push([x, a[1], z], [x, b[1], z], 0.55);
            let m = (push[0].powi(2) + push[2].powi(2)).sqrt();
            let ch = if m < 1e-9 { '.' } else { format!("{}", (m * 20.0).round().min(9.0) as u8).pop().unwrap() };
            row.push(ch);
            x += 0.25;
        }
        eprintln!("z={z:5.2}: {row}   (x −28.25..−25.5, 0.25 steps; last col x=-25.5)");
        z += 0.25;
    }
    // The vertical pushes too (head bumps / step-ups) along the corridor centre.
    eprintln!("== vertical push (y) along x −27.4..−26.4 ==");
    let mut z = 29.0f32;
    while z <= 43.0 {
        let mut row = String::new();
        let mut x = -27.4f32;
        while x <= -26.4 {
            let (a, b) = capsule_at(f + 1.1);
            let (push, _) = world.capsule_push([x, a[1], z], [x, b[1], z], 0.55);
            row.push(if push[1] > 0.05 { 'u' } else if push[1] < -0.05 { 'd' } else { '.' });
            x += 0.25;
        }
        eprintln!("z={z:5.2}: {row}   (u = pushed up >0.05, d = down, . = none; x −27.4..−26.4 step 0.25)");
        z += 0.25;
    }
}

// ── 4. BFS over the capsule grid: is the spawn pocket connected to a ladder grab cell / the main floor?

#[test]
fn probe2_room_bfs() {
    let level = load_level("07000002.bin");
    let world = CollisionWorld::build(&level, &level.instances(true).0);
    let ladders = nf_game::ladder::all(&level);
    for (i, l) in ladders.iter().enumerate() {
        eprintln!("ladder[{i}] centre ({:.2},{:.2},{:.2}) r {:.2} yaw {:.3} top {:.2} bottom {:.2}",
            l.centre[0], l.centre[1], l.centre[2], l.radius, l.yaw, l.top(), l.bottom());
    }

    // Grid nodes: (x, z, floor) with 0.5 steps. Passability: strict (capsule unpushed) or lenient
    // (any floor — tight gaps the capsule barely fits may still be walkable in the push engine).
    let (x0, x1, z0, z1, st) = (-40.0f32, -6.0f32, 10.0f32, 50.0f32, 0.5f32);
    let cols = ((x1 - x0) / st) as i32 + 1;
    let rows = ((z1 - z0) / st) as i32 + 1;
    let cx = |ix: i32| x0 + ix as f32 * st;
    let cz = |iz: i32| z0 + iz as f32 * st;

    let mut stacks: Vec<Vec<Vec<f32>>> = Vec::new();
    for iz in 0..rows {
        let mut row = Vec::new();
        for ix in 0..cols {
            row.push(floor_stack(&world, cx(ix), cz(iz), 8.0, 14.0));
        }
        stacks.push(row);
    }
    let blocked_strict: Vec<Vec<Vec<bool>>> = stacks.iter().enumerate().map(|(iz, r)| r.iter().enumerate().map(|(ix, s)| {
        s.iter().map(|f| cell_blocked(&world, cx(ix as i32), f + 1.1, cz(iz as i32))).collect()
    }).collect()).collect();

    // Goal predicate: ladder grab cells + the main floor band.
    let grab_goal = |ix: i32, iz: i32, f: f32| -> Option<(usize, f32)> {
        let p = [cx(ix), 0.0, cz(iz)];
        let mut best = None;
        for (li, l) in ladders.iter().enumerate() {
            let reach = l.radius.clamp(0.0, 1.0).max(0.35) + 0.55;
            let d = ((p[0] - l.centre[0]).powi(2) + (p[2] - l.centre[2]).powi(2)).sqrt();
            let root = f + 1.1;
            if d <= reach && root > l.centre[1] - l.radius - 0.5 && root < l.centre[1] + l.radius + 0.5 {
                if best.map_or(true, |(_, bd)| d < bd) { best = Some((li, d)); }
            }
        }
        best
    };

    for strict in [true, false] {
        // BFS over (ix, iz, floor-slot). Edge kinds: S step (|df|<=0.65), J jump (<=1.35), F fall (down).
        #[derive(Clone, Copy, PartialEq)]
        struct N { ix: i32, iz: i32, fl: u8 }
        let start = (((-27.17 - x0) / st) as i32, ((29.80 - z0) / st) as i32);
        let (six, siz) = start;
        let start_fl = stacks[siz as usize][six as usize].iter().enumerate()
            .min_by(|a, b| (a.1 - -0.049).abs().total_cmp(&(b.1 - -0.049).abs())).map(|(k, _)| k as u8).expect("spawn floor");
        let key = |n: N| (n.ix, n.iz, n.fl);
        let mut prev: std::collections::HashMap<(i32, i32, u8), (N, char)> = std::collections::HashMap::new();
        let mut q = std::collections::VecDeque::new();
        let s0 = N { ix: six, iz: siz, fl: start_fl };
        q.push_back(s0);
        let mut goal: Option<(N, usize, f32)> = None;
        let mut mainf: Option<N> = None;
        let mut visited = 0usize;
        while let Some(n) = q.pop_front() {
            visited += 1;
            let f = stacks[n.iz as usize][n.ix as usize][n.fl as usize];
            if mainf.is_none() && f >= 5.5 { mainf = Some(n); }
            if goal.is_none() { if let Some((li, d)) = grab_goal(n.ix, n.iz, f) { goal = Some((n, li, d)); } }
            let free = |ix: i32, iz: i32, fl: u8| -> bool {
                !(strict && blocked_strict[iz as usize][ix as usize][fl as usize])
            };
            // neighbours: 4-way, any floor slot within the step/jump/fall bands
            let mut push = |n: N, pn: N, kind: char, prev: &mut std::collections::HashMap<_, _>, q: &mut std::collections::VecDeque<N>| {
                if prev.contains_key(&key(pn)) || !free(pn.ix, pn.iz, pn.fl) { return; }
                prev.insert(key(pn), (n, kind));
                q.push_back(pn);
            };
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (n.ix + dx, n.iz + dz);
                if nx < 0 || nx >= cols || nz < 0 || nz >= rows { continue; }
                for (fl2, f2) in stacks[nz as usize][nx as usize].iter().enumerate() {
                    let df = f2 - f;
                    let kind = if df.abs() <= 0.65 { 'S' } else if df > 0.65 && df <= 1.35 { 'J' } else if df < -0.65 { 'F' } else { continue };
                    push(n, N { ix: nx, iz: nz, fl: fl2 as u8 }, kind, &mut prev, &mut q);
                }
                // drop through a hole in the SAME cell (the stack's lower floors)
                for (fl2, f2) in stacks[n.iz as usize][n.ix as usize].iter().enumerate() {
                    let df = f2 - f;
                    if df < -0.65 {
                        push(n, N { ix: n.ix, iz: n.iz, fl: fl2 as u8 }, 'F', &mut prev, &mut q);
                    }
                }
            }
        }
        eprintln!("== BFS strict={strict}: visited {visited} nodes ==");
        let unf = |n: N| format!("({:.1},{:.1},f{:.2})", cx(n.ix), cz(n.iz), stacks[n.iz as usize][n.ix as usize][n.fl as usize]);
        if let Some(mn) = mainf {
            let mut path = Vec::new();
            let mut k = key(mn);
            while k != key(s0) { let (p, kind) = prev[&k]; path.push((k, kind)); k = key(p); }
            path.reverse();
            eprintln!("MAIN-FLOOR node {} — path {} nodes: {}", unf(mn), path.len(),
                path.iter().map(|(k, kind)| {
                    let (ix, iz, fl) = *k;
                    let tag = if *kind == 'S' { String::new() } else { kind.to_string() };
                    format!("{tag}({:.1},{:.1},{:.2})", cx(ix), cz(iz), stacks[iz as usize][ix as usize][fl as usize])
                }).collect::<Vec<_>>().join(" "));
        } else {
            eprintln!("MAIN-FLOOR (f>=5.5): NOT reachable (strict={strict})");
        }
        match goal {
            Some((gn, li, d)) => {
                let mut path = Vec::new();
                let mut k = key(gn);
                while k != key(s0) { let (p, kind) = prev[&k]; path.push((k, kind)); k = key(p); }
                path.reverse();
                eprintln!("LADDER[{li}] grab node {} (d {d:.2}) — path {} nodes:", unf(gn), path.len());
                for (k, kind) in &path {
                    let (ix, iz, fl) = *k;
                    eprintln!("   {} ({:.2},{:.2},{:.2})", kind, cx(ix), cz(iz), stacks[iz as usize][ix as usize][fl as usize]);
                }
            }
            None => eprintln!("LADDER grab cells: NOT reachable (strict={strict})"),
        }
    }
}

// ── 3. The room's objects: ladders, doors, triggers, pickups + grab windows ─────────────────────

#[test]
fn probe2_room_objects() {
    let scene = nf_view::load(&iso(), "07000002.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let p = sim.bond.pos;
    eprintln!("spawn bond.pos = {p:?}");

    eprintln!("== ladders (sim.ladders) ==");
    for (i, l) in sim.ladders.iter().enumerate() {
        eprintln!(
            "ladder[{i}] centre ({:.2},{:.2},{:.2}) radius {:.2} yaw {:.3} top {:.2} bottom {:.2} ch {} dxz(spawn) {:.1}",
            l.centre[0], l.centre[1], l.centre[2], l.radius, l.yaw, l.top(), l.bottom(), l.channel, dxz(l.centre, p)
        );
    }

    // Grab windows: horizontal reach = half_width + capsule (Ladder::capsule_hits), vertical =
    // centre.y − radius − 0.5 .. centre.y + radius + 0.5 on the ROOT (hip); standing hip = floor + 1.1.
    eprintln!("== ladder grab windows (which floors/cells can grab) ==");
    for (i, l) in sim.ladders.iter().enumerate() {
        let half_width = l.radius.clamp(0.0, 1.0).max(0.35);
        let reach = half_width + 0.55;
        let (ylo, yhi) = (l.centre[1] - l.radius - 0.5, l.centre[1] + l.radius + 0.5);
        eprintln!("ladder[{i}]: reach {reach:.2} root window ({ylo:.2}..{yhi:.2}) -> floor window ({:.2}..{:.2})", ylo - 1.1, yhi - 1.1);
        let mut best: Option<(f32, V3, f32)> = None; // (dxz from centre, cell, floor)
        let mut x = l.centre[0] - reach;
        while x <= l.centre[0] + reach + 1e-6 {
            let mut z = l.centre[2] - reach;
            while z <= l.centre[2] + reach + 1e-6 {
                let d = ((x - l.centre[0]).powi(2) + (z - l.centre[2]).powi(2)).sqrt();
                if d <= reach {
                    for f in floor_stack(&sim.world, x, z, 8.0, 14.0) {
                        let root = f + 1.1;
                        if root > ylo && root < yhi {
                            eprintln!("   cell ({x:.2},{z:.2}) floor {f:.2} root {root:.2} d {d:.2}");
                            if best.map_or(true, |(bd, _, _)| d < bd) { best = Some((d, [x, root, z], f)); }
                        }
                    }
                }
                z += 0.25;
            }
            x += 0.25;
        }
        if let Some((d, c, f)) = best {
            eprintln!("ladder[{i}]: BEST grab cell {:?} (floor {f:.2}, d {d:.2}); climb to top {:.2}", c, l.top());
        } else {
            eprintln!("ladder[{i}]: NO standing cell inside the grab window (on the port's collision)");
        }
    }

    // Doors and triggers around the room (room bbox padded).
    let (rlo, rhi) = ([-34.0f32, -3.0, 20.0], [-6.0f32, 9.0, 48.0]);
    let in_room = |p: V3| (rlo[0]..rhi[0]).contains(&p[0]) && (rlo[1]..rhi[1]).contains(&p[1]) && (rlo[2]..rhi[2]).contains(&p[2]);
    eprintln!("== doors with pos in room bbox {rlo:?}..{rhi:?} ==");
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        if in_room(d.pos) {
            eprintln!("door[{i}] kind {} pos ({:.2},{:.2},{:.2}) state {:#x} pos01 {:.2} ch_open {:#x} ch_lock {:#x} lock_flag {} approach {}",
                d.kind, d.pos[0], d.pos[1], d.pos[2], d.state, d.pos01, d.ch_open, d.ch_lock, d.locked_flag, d.approach);
        }
    }
    eprintln!("== triggers intersecting the room bbox ==");
    for t in sim.drones.triggers.iter() {
        if t.tris.is_empty() { continue; }
        let (lo, hi) = (t.lo, t.hi);
        let over = (0..3).all(|k| hi[k] >= rlo[k] && lo[k] <= rhi[k]);
        if over {
            eprintln!("trig kind {:#x} out {:#x} mask {:#x} lock {:#x} lo ({:.1},{:.1},{:.1}) hi ({:.1},{:.1},{:.1}) next_level {:#x} gate {:#x} key {:#x} succ {:#x} fb {:#x}",
                t.kind, t.out, t.mask, t.lock, lo[0], lo[1], lo[2], hi[0], hi[1], hi[2], t.next_level, t.gate, t.key_gate, t.success, t.fallback);
        }
    }
    eprintln!("== pickups in the room ==");
    for pk in sim.drones.pickups.iter() {
        let at = [pk.m[3][0], pk.m[3][1], pk.m[3][2]];
        if in_room(at) && !pk.gone {
            eprintln!("pickup kind {:#x} at ({:.2},{:.2},{:.2}) sw {:#x} hidden {}", pk.kind, at[0], at[1], at[2], pk.switch_channel, pk.hidden);
        }
    }
    // All placement kinds 0x20..0x2F and 0xE0.. anywhere near the room, from the raw level (all maps).
    let level = load_level("07000002.bin");
    eprintln!("== raw placements near the room (all maps, kinds 0x20-0x2F/0xD4/0xE0-0xEF) ==");
    for (mi, m) in level.maps.iter().enumerate() {
        for (pi, pl) in m.placements.iter().enumerate() {
            let k = pl.flags & 0xff;
            let t = pl.translation;
            if !(matches!(k, 0x20..=0x2f | 0xd4 | 0xe0..=0xef)) { continue; }
            if !(rlo[0]..rhi[0]).contains(&t[0]) || !(rlo[2]..rhi[2]).contains(&t[2]) { continue; }
            eprintln!("map {mi} #{pi} kind {k:#x} at ({:.2},{:.2},{:.2}) params {:?}", t[0], t[1], t[2], pl.params);
        }
    }
}
