//! The levels sweep (fix-levels-sweep): a health check of every on-foot level on the disc.
//!
//! Two passes per level, both through the REAL Sim path (`nf_view::load` + `Sim::step_bond`):
//! - the smoke (`every_level_loads_and_runs_600_ticks`): load + 600 idle ticks, no panic;
//! - the deep sweep (`deep_sweep_1800_ticks_walk_and_issue_counts`): 1800 ticks of a scripted
//!   walk along the AI path graph (the nav nodes as waypoints, E pressed every half second for
//!   doors), then one CPU pose pass (the GPU-less `--shot` counts only — this environment has no
//!   Vulkan ICD, docs/systems/levels.md §6) — recording per level: panics, unresolved objects,
//!   placements of types the port does not handle, drones stuck in unported states, script
//!   players that never start, and pickups that do not resolve a model.
//!
//! The objective-channel writer scan (`objective_channel_writers`) walks `nf_game::mission::LEVELS`
//! and finds each objective's condition/enable channel writers on the disc: triggers, muxes, script
//! event-0xD writes (the level's script lib INCLUDING the fix-cutscenes loadable packs), switches,
//! pickups, drone death channels (placement param 16), doors' open channels, and `Mission_Init`'s
//! init value [M 0x182270:106-112]. Opt-in via NF_ISO.

use nf_formats::{disc, iso9660::IsoImage, level::Level};
use nf_game::mission;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Every level archive in FILES.BIN whose name is 0700xxxx.bin, minus the driving missions
/// (07000023-29, out of scope per docs/systems/levels.md §3) and the shared packs (07000500 the
/// global script pack, 07000999). Wrappers (41/43/46/48/49/4a/4b) are on-foot levels and stay in.
fn on_foot_levels(img: &IsoImage) -> Vec<String> {
    let (entries, _) = disc::files_bin_entries(img).unwrap();
    let mut names: Vec<String> = entries.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .filter(|n| {
            let code = u32::from_str_radix(&n[..8], 16).unwrap_or(0) & 0xff;
            !(0x23..=0x29).contains(&code) && code != 0x00 && n != "07000500.bin" && n != "07000999.bin"
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

fn load_level(iso: &str, name: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, name, false).expect("load")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// The required smoke: every on-foot level loads and runs 600 ticks without a panic.
#[test]
fn every_level_loads_and_runs_600_ticks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).unwrap();
    let levels = on_foot_levels(&img);
    eprintln!("[sweep] {} on-foot levels on this disc", levels.len());
    assert!(levels.len() >= 27, "the disc's on-foot level set");
    let mut failures: Vec<(String, String)> = Vec::new();
    for lv in &levels {
        let r = std::panic::catch_unwind(|| {
            let mut scene = load_level(&iso, lv);
            tick(&mut scene, 600);
        });
        if let Err(e) = r {
            let msg = e.downcast_ref::<String>().cloned().unwrap_or_else(|| format!("{e:?}"));
            failures.push((lv.clone(), msg));
        }
    }
    assert!(failures.is_empty(), "levels that panic within 600 ticks: {failures:?}");
}

/// One nav waypoint walk step: yaw toward the goal, walk on, look ahead.
fn walk_step(sim: &mut nf_view::scene::Sim, goal: [f32; 3], use_key: bool) {
    let dx = goal[0] - sim.bond.pos[0];
    let dz = goal[2] - sim.bond.pos[2];
    let mut input = nf_game::player::Input::default();
    if dx.hypot(dz) > 0.05 {
        input.forward = 1.0;
        // Bond's yaw drives the look: the aim yaw is atan2(dx, dz) in the view convention.
        input.turn = 0.0;
        sim.bond.yaw = dx.atan2(dz);
        sim.bond.pitch = 0.0;
    }
    sim.step_bond(input, false, false, use_key);
}

/// The per-level deep sweep with the issue counts. Prints one `[sweep]` line per level.
#[test]
fn deep_sweep_1800_ticks_walk_and_issue_counts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).unwrap();
    let levels = on_foot_levels(&img);
    let mut total_panics = 0usize;
    for lv in &levels {
        let name = lv.clone();
        let run = std::panic::catch_unwind(|| -> Result<String, String> {
            let mut scene = load_level(&iso, &name);
            let (mut unresolved, mut unhandled_types, mut sp_total, mut pickups_placed, mut unhandled_names) = (0usize, Vec::new(), 0usize, 0usize, Vec::new());
            if let (Some(sim), Some(data)) = (scene.sim.as_ref(), level_data(&iso, &name)) {
                unresolved = data.0;
                // The handled placement types: the create_dynamic_objects (0x1d04f0) cases the port
                // creates gameplay or render for; 0x47/0x19/0x2a/0xdb/0xdf are draw-only families.
                // The types the port gives gameplay or render for (the master set; 0x35 PCQWorm
                // lands with s/map-tower). 0x17/0x23 are not dynamic objects in the game's parser
                // (no `create_dynamic_objects` case [M 0x1d04f0]) — map-block data, skipped.
                const HANDLED: &[u8] = &[0x0f, 0x19, 0x20, 0x22, 0x24, 0x25, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c,
                    0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x36, 0x3e, 0x47, 0x71, 0xd9, 0xdb,
                    0xdc, 0xde, 0xdf, 0xe1, 0xe2, 0xe4, 0xe5, 0xe6, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee,
                    0xef, 0xf0, 0xf1, 0xf5, 0xf8, 0xf9, 0xfa, 0xfb, 0xfd, 0xfe];
                for p in data.1.iter() {
                    let t = (p.flags & 0xff) as u8;
                    if !HANDLED.contains(&t) && !unhandled_types.contains(&t) { unhandled_types.push(t); }
                }
                pickups_placed = data.1.iter().filter(|p| p.flags & 0xff == 0xf0).count();
                sp_total = sim.drones.scripts.players.len();
                let missing_scripts = sim.drones.scripts.players.iter().filter(|p| p.deleted).count();
                if missing_scripts > 0 { unhandled_names.push(format!("{missing_scripts} deleted SPs")); }
            }
            // The scripted walk: 1800 ticks along the nav graph; the drone states snapshotted at
            // tick 900 for the stuck check.
            let mut stuck_nodes = 0usize;
            let mut states_at_900: Vec<(usize, u8)> = Vec::new();
            {
                let sim = scene.sim.as_mut().unwrap();
                let waypoints: Vec<[f32; 3]> = sim.nav.paths.iter().flat_map(|p| p.nodes.iter().map(|n| n.pos)).collect();
                if !waypoints.is_empty() {
                    let mut gi = nearest(&waypoints, sim.bond.pos);
                    let mut held = 0u32;
                    for t in 0..1800u32 {
                        let goal = waypoints[gi];
                        walk_step(sim, goal, t % 30 == 15);
                        if t == 900 {
                            states_at_900 = sim.drones.list.iter().enumerate().map(|(i, d)| (i, d.state)).collect();
                        }
                        let d = (sim.bond.pos[0] - goal[0]).hypot(sim.bond.pos[2] - goal[2]);
                        if d < 1.2 { held = 0; gi = (gi + 1) % waypoints.len(); }
                        else { held += 1; if held == 600 { stuck_nodes += 1; held = 0; gi = (gi + 1) % waypoints.len(); } }
                    }
                } else {
                    for t in 0..1800 {
                        if t == 900 { states_at_900 = sim.drones.list.iter().enumerate().map(|(i, d)| (i, d.state)).collect(); }
                        sim.step_bond(Default::default(), false, false, false);
                    }
                }
            }
            // Post-walk counts. `unported_states` = drone state ids with NO constant in the port's
            // state machine (the game's DSTATE table has more entries than the port's 88 constants;
            // e.g. mode 7's 0x85 and 07000007's 0xb4 park drones forever). `never_started` = drones
            // still in GLOBAL (0x00) — their control never advanced past creation.
            let mut stuck_drones = 0usize;
            let mut never_started = 0usize;
            let mut unmodelled_pickups = 0usize;
            let mut live_states: Vec<u8> = Vec::new();
            let mut unported_hist: std::collections::BTreeMap<u8, usize> = Default::default();
            const PORT_STATES: &[u8] = &[0x00, 0x01, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0d, 0x10, 0x11, 0x12,
                0x13, 0x14, 0x16, 0x1c, 0x1d, 0x20, 0x23, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
                0x31, 0x32, 0x33, 0x34, 0x35, 0x37, 0x38, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x46, 0x47,
                0x48, 0x49, 0x53, 0x55, 0x56, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5e, 0x5f, 0x60, 0x62, 0x63,
                0x65, 0x66, 0x67, 0x68, 0x69, 0x6d, 0x6f, 0x70, 0x71, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
                0x79, 0x7a, 0x7f, 0x8c, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0xa2,
                0xa3, 0xa4, 0xa5];
            if let Some(sim) = scene.sim.as_ref() {
                // Legal long-idle states (drone.rs state:: hex values): waiting on a switch, idle
                // guards, snipers at post, corpses/fades, knocked out, civilians parked, the
                // surrender/ally/party/driver terminals, ambush waits, under-cover idle.
                const LEGAL_IDLE: &[u8] = &[0x01, 0x04, 0x0d, 0x11, 0x13, 0x20, 0x23, 0x29, 0x2f, 0x30,
                    0x31, 0x34, 0x38, 0x40, 0x43, 0x47, 0x48, 0x49, 0x94];
                let mut stuck_hist: std::collections::BTreeMap<u8, usize> = Default::default();
                for (i, dr) in sim.drones.list.iter().enumerate().filter(|(_, d)| d.present) {
                    live_states.push(dr.state);
                    if !PORT_STATES.contains(&dr.state) {
                        stuck_drones += 1;
                        *unported_hist.entry(dr.state).or_default() += 1;
                    }
                    let was = states_at_900.iter().find(|(j, _)| *j == i).map(|(_, s)| *s);
                    if was == Some(dr.state) && dr.state == 0x00 {
                        never_started += 1;
                    }
                }
                if !unported_hist.is_empty() { unhandled_names.push(format!("UNPORTED_STATES {:x?}", unported_hist)); }
                if never_started > 0 { unhandled_names.push(format!("never_started {never_started} (GLOBAL)")); }
                // Mode-1 SPs still in state 7 after the walk = their gate channel (param 9) never
                // fired — the port's per-tick gate early-return [M 0x1dbda8:60-62] keeps them from
                // loading. A legitimate wait, not a defect; the count feeds the table.
                never_started = sim.drones.scripts.players.iter()
                    .filter(|p| !p.deleted && p.modes[0] == 1 && p.state == 7).count();
                unmodelled_pickups = pickups_placed.saturating_sub(sim.drones.pickups.len());
            }
            // The GPU-less frame: one CPU pose pass, counts only (no Vulkan ICD here, levels.md §6).
            let opts = nf_view::scene::ViewOpts::default();
            let verts = scene.posed_vertices(None, 0.0, opts).len();
            Ok(format!("unresolved {unresolved} unhandled_types {unhandled_types:?} unported_states {stuck_drones} \
                never_started_sps {never_started_sp} never_started_drones {never_started} sp_total {sp_total} unmodelled_pickups {unmodelled_pickups}/{pickups_placed} \
                walk_stuck_nodes {stuck_nodes} pose_verts {verts} {notes}", never_started_sp = never_started, notes = unhandled_names.join(" | ")))
        }).map_err(|e| e.downcast_ref::<String>().cloned().unwrap_or_else(|| format!("{e:?}")));
        match run {
            Ok(Ok(line)) => eprintln!("[sweep] {name}: {line}"),
            Ok(Err(msg)) => { eprintln!("[sweep] {name}: ERROR {msg}"); total_panics += 1; }
            Err(msg) => { eprintln!("[sweep] {name}: PANIC {msg}"); total_panics += 1; }
        }
    }
    assert_eq!(total_panics, 0, "levels that panic in the deep sweep");
}

fn level_data(iso: &str, name: &str) -> Option<(usize, Vec<nf_formats::placement::Placement>)> {
    let img = IsoImage::open(iso).ok()?;
    let (entries, bin) = disc::files_bin_entries(&img).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    let level = Level::parse(&data).ok()?;
    let (instances, missing) = level.scene_instances(true);
    let main = level.main_map()?;
    let placements = level.maps[main].placements.clone();
    Some((missing, placements))
}

fn nearest(points: &[[f32; 3]], pos: [f32; 3]) -> usize {
    let mut best = 0usize;
    let mut bd = f32::INFINITY;
    for (i, p) in points.iter().enumerate() {
        let d = (p[0] - pos[0]).powi(2) + (p[1] - pos[1]).powi(2) + (p[2] - pos[2]).powi(2);
        if d < bd { bd = d; best = i; }
    }
    best
}

/// Every objective's condition/enable channel and the writers found for it on the disc — scanned
/// across the WHOLE mission (the monitor matches BaseMap, so a sibling level's mux can write this
/// level's objective channel — 07000005's ch 0x46 is written by 07000006's OR mux #498 [D]).
/// Writers: trigger outs (incl. the OUT-mirror's list and the LOAD_LEVEL success/fallback), script
/// event-0xD writes (the lib includes the fix-cutscenes loadable packs), switches, pickups, drone
/// death channels (placement param 16), fuseboxes, cam-subjects/destroy props, and `Mission_Init`'s
/// init value [M 0x182270:106-112 — +0x0C channel, +0x0D init per the decomp's byte offsets].
#[test]
fn objective_channel_writers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).unwrap();
    let levels = on_foot_levels(&img);
    // The writers per level, collected once.
    let mut per_level: std::collections::BTreeMap<String, std::collections::BTreeMap<u8, Vec<String>>> = Default::default();
    for lv in &levels {
        let Some(data) = level_data(&iso, lv) else { continue };
        let scene = load_level(&iso, lv);
        let Some(sim) = scene.sim.as_ref() else { continue };
        let mut writers: std::collections::BTreeMap<u8, Vec<String>> = Default::default();
        let mut note = |ch: u8, w: &str| { if ch != 0 { writers.entry(ch).or_default().push(w.to_string()); } };
        for t in sim.drones.triggers.iter() {
            let kind_name = format!("trigger({})", t.kind);
            note(t.out, &kind_name);
            // The OUT-mirror (kind 5) writes its list from its out input.
            if t.kind == nf_game::triggers::kind::OUT {
                for &c in t.list.iter().filter(|&&c| c != 0) { note(c as u8, "out-mirror(target)"); }
            }
            if t.kind == nf_game::triggers::kind::LOAD_LEVEL {
                note(t.success, "load_level(success)");
                note(t.fallback, "load_level(fallback)");
            }
        }
        for (h, n) in sim.drones.scripts.lib.iter() {
            for s in n.streams.iter() {
                let cmds = &s.commands;
                for i in 0..cmds.len() {
                    if cmds[i] == 0x12 && i + 3 <= cmds.len() {
                        let n_params = cmds[i + 1] as usize;
                        // The port's stream reader (`nis.rs run()`'s rd) is LITTLE-endian over the
                        // raw bytes — the same order the game's script data uses [M 0x1d94c0 via
                        // the port's validated event writes].
                        let le = |o: usize| cmds.get(i + 3 + o..i + 3 + o + 4).map(|b| b.iter().rev().fold(0u32, |a, &x| (a << 8) | x as u32)).unwrap_or(0);
                        match (cmds[i + 2], n_params) {
                            // Event 0xD: the direct write [ch, v] [M Script_EventHandler 0x1d94c0].
                            (0x0d, np) if np >= 2 => note(le(0) as u8, &format!("script {h:08x} event0d v{}", le(4))),
                            // Event 0x11: the conditional write — if ch[p0] on → ch[p1]=p3 else ch[p2]=p3
                            // [M 0x1d94c0; the port's nis.rs event arm]. Both targets are writers.
                            (0x11, np) if np >= 4 => {
                                note(le(4) as u8, &format!("script {h:08x} event11+"));
                                note(le(8) as u8, &format!("script {h:08x} event11-"));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        for f in sim.drones.fuseboxes.iter() { for c in f.ch { if c != 0 { note(c as u8, "fusebox(0x31)"); } } }
        for p in sim.drones.props.subjects.iter() { note(p.channel, "camsubject(0xfd)"); }
        for p in sim.drones.props.destroy.iter() { note(p.channel, "destroy(0xe1)"); }
        for p in sim.drones.pickups.iter() { note(p.switch_channel as u8, "pickup(0xf0)"); }
        for p in data.1.iter().filter(|p| p.flags & 0xff == 0x0f) {
            // The death channel is drone placement param 12 [M set_death_channel 0x13e660 via
            // Drone::new's `death_channel: param(12)`; 07000014's Rook writes 0x54 on death [D]].
            let k12 = p.params.iter().find(|(i, _)| *i == 12).map_or(0, |&(_, v)| v);
            note(k12 as u8, "drone death(p12)");
        }
        per_level.insert(lv.clone(), writers);
    }
    // The report: per record, the writers found anywhere in the same mission.
    for def in mission::LEVELS.iter() {
        let lv = format!("{:08x}.bin", def.level);
        let sibling_writers: Vec<&std::collections::BTreeMap<u8, Vec<String>>> = mission::LEVELS.iter()
            .filter(|d| d.mission == def.mission)
            .filter_map(|d| per_level.get(&format!("{:08x}.bin", d.level)))
            .collect();
        let get = |ch: u8| -> String {
            let hits: Vec<&Vec<String>> = sibling_writers.iter().filter_map(|w| w.get(&ch)).collect();
            if hits.is_empty() { "NONE".into() } else {
                hits.iter().map(|v| v.join(", ")).collect::<Vec<_>>().join(" | ")
            }
        };
        for o in def.objectives.iter() {
            let cond = if o.channel == 0 { "0(raw)".to_string() } else { format!("{:#04x}", o.channel) };
            let init_w = if o.init != 0 { format!("mission_init({})", o.init) } else { get(o.channel) };
            let w = if o.init != 0 { format!("mission_init writes {:#04x}={} [M 0x182270:106-112]", o.channel, o.init) } else { init_w };
            let en = if o.enable != 0 { format!(" enable {:#04x}[{}]", o.enable, get(o.enable)) } else { String::new() };
            let flag = if o.flags & 2 != 0 { " INVERTED" } else { "" };
            eprintln!("[chan] {lv} part{} 0x{:08x} ch {}{}{} → {}", def.part, o.text, cond, en, flag, w);
        }
    }
}
