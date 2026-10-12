//! fix-objects2 probe: the 07000008 invisible wall after the opening cut-scene.
//! Dumps every main-map instance with collision near the spawn/stairs, then walks Bond down
//! through the real input path and, when he stops making progress, bisects which instance's
//! collision triangles block him (per-instance mini CollisionWorlds, the NF_COLL idea at
//! contact time). Opt-in via NF_ISO.

use nf_formats::level::Level;
use nf_formats::levelbin::LevelBin;
use nf_formats::iso9660::IsoImage;
use nf_game::player::Input;
use nf_formats::placement::transform;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
fn probe_dumps_collision_instances_and_placements() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000008.bin");
    let level = Level::parse(&data).expect("parse");
    let mm = level.main_map().unwrap();
    let (insts, missing) = level.instances(true);
    eprintln!("--- main map {mm}: {} objects, {} placements, {} cel instances, {missing} unresolved ---",
        level.maps[mm].objects.len(), level.maps[mm].placements.len(), insts.len());

    let spawn = [139.5f32, 50.5, -76.4];
    eprintln!("--- cel instances with collision, nearest the spawn first ---");
    let mut with_coll = Vec::new();
    for (i, inst) in insts.iter().enumerate() {
        let obj = &level.maps[inst.map].objects[inst.object];
        if obj.collision.is_none() { continue }
        let p = inst.placement.translation;
        let d = ((p[0] - spawn[0]).powi(2) + (p[1] - spawn[1]).powi(2) + (p[2] - spawn[2]).powi(2)).sqrt();
        with_coll.push((d, i));
    }
    with_coll.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for (d, i) in with_coll.iter().take(40) {
        let inst = &insts[*i];
        let obj = &level.maps[inst.map].objects[inst.object];
        let n_tris = obj.collision.as_ref().unwrap().triangles().len();
        let has_mesh = obj.gfx.leaves.iter().any(|l| !l.batches.is_empty());
        eprintln!("[i{i}] \"{}\" d={d:.1} at ({:.1},{:.1},{:.1}) tris {n_tris} objflags {:#x} has_mesh {has_mesh} skipped_cel {} pflags {:#x}",
            obj.name.trim(), inst.placement.translation[0], inst.placement.translation[1], inst.placement.translation[2],
            obj.flags, obj.skipped_by_cel_pass(), inst.placement.flags);
    }

    eprintln!("--- ALL placements (any flags) within 45 of the spawn ---");
    for (pi, p) in level.maps[mm].placements.iter().enumerate() {
        let q = p.translation;
        let d = ((q[0] - spawn[0]).powi(2) + (q[1] - spawn[1]).powi(2) + (q[2] - spawn[2]).powi(2)).sqrt();
        if d > 45.0 { continue }
        let name = if p.by_hash() {
            level.by_hash.get(&p.hashcode).map(|(m, o)| level.maps[*m].objects[*o].name.trim().to_string()).unwrap_or(format!("unresolved {:08x}", p.hashcode))
        } else if (p.index as usize) < level.maps[mm].objects.len() {
            level.maps[mm].objects[p.index as usize].name.trim().to_string()
        } else { "bad index".into() };
        let obj = if p.by_hash() {
            level.by_hash.get(&p.hashcode).and_then(|(m, o)| level.maps[*m].objects.get(*o))
        } else { level.maps[mm].objects.get(p.index as usize) };
        let (tris, mesh) = obj.map(|o| (o.collision.as_ref().map(|c| c.triangles().len()).unwrap_or(0), o.gfx.leaves.iter().any(|l| !l.batches.is_empty()))).unwrap_or((0, false));
        eprintln!("[p{pi}] flags {:#010x} \"{}\" d={d:.1} at ({:.1},{:.1},{:.1}) rot ({:.2},{:.2},{:.2}) params {:?} tris {tris} mesh {mesh}",
            p.flags, name, q[0], q[1], q[2], p.rotation[0], p.rotation[1], p.rotation[2], p.params);
    }
    let _ = LevelBin::parse(&data);
}

/// Dump the nav paths (the drones' routes trace the level's real walkable route), the world
/// AABBs of key geometry, and the trigger chain.
#[test]
fn probe_route_and_geometry() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    {
        let sim = scene.sim.as_ref().unwrap();
        let data = sim.data();
        eprintln!("--- nav paths ---");
        for p in data.nav.paths.iter() {
            eprintln!("[path \"{}\"] kind {:#x} nodes {} links {}", p.name.trim(), p.kind, p.nodes.len(), p.links.len());
            for (ni, n) in p.nodes.iter().enumerate() {
                eprintln!("   n{ni:02} ({:.1},{:.1},{:.1}) flags {:#x}", n.pos[0], n.pos[1], n.pos[2], n.flags);
            }
        }
    }
    let data = read_archive(&iso, "07000008.bin");
    let level = Level::parse(&data).expect("parse");
    let (insts, _) = level.instances(true);
    eprintln!("--- instances whose name matches geometry keywords (world centre from the matrix) ---");
    for (i, inst) in insts.iter().enumerate() {
        let obj = &level.maps[inst.map].objects[inst.object];
        let n = obj.name.trim();
        if !["step", "stair", "balcon", "walk", "zip", "gondola", "cable", "blocking", "platform", "rail"]
            .iter().any(|k| n.to_lowercase().contains(k)) { continue }
        let c = transform(&inst.matrix, obj.centre);
        eprintln!("[i{i}] \"{n}\" world c ({:.1},{:.1},{:.1}) r {:.1} tris {} objflags {:#x}",
            c[0], c[1], c[2], obj.radius, obj.collision.as_ref().map(|x| x.triangles().len()).unwrap_or(0), obj.flags);
    }
    eprintln!("--- ALL trigger-family placements (0xD9..0xFF) with resolved out-channels ---");
    let mm = level.main_map().unwrap();
    for (pi, p) in level.maps[mm].placements.iter().enumerate() {
        let f = p.flags & 0xff;
        if !(0xd9..=0xff).contains(&f) { continue }
        let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|(_, v)| *v).unwrap_or(0);
        let name = if p.by_hash() {
            level.by_hash.get(&p.hashcode).map(|(m, o)| level.maps[*m].objects[*o].name.trim().to_string()).unwrap_or(format!("unresolved {:08x}", p.hashcode))
        } else if (p.index as usize) < level.maps[mm].objects.len() {
            level.maps[mm].objects[p.index as usize].name.trim().to_string()
        } else { "bad index".into() };
        eprintln!("[p{pi}] kind {f:#x} \"{}\" at ({:.1},{:.1},{:.1}) p0 {} p1 {} p2 {} p3 {}",
            name, p.translation[0], p.translation[1], p.translation[2], param(0), param(1), param(2), param(3));
    }
}

/// The invisible blockers in world space: every instance whose object is skipped by the cel pass
/// (glist flag 0x10) with collision, triangles transformed to world space, plus the AABB. These are
/// the level's collision-only "BlockingPoly" objects.
#[test]
fn probe_invisible_blocker_geometry() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000008.bin");
    let level = Level::parse(&data).expect("parse");
    let (insts, _) = level.instances(true);
    for (i, inst) in insts.iter().enumerate() {
        let obj = &level.maps[inst.map].objects[inst.object];
        let Some(c) = obj.collision.as_ref() else { continue };
        if !obj.skipped_by_cel_pass() { continue }
        let tris: Vec<[[f32; 3]; 3]> = c.triangles().iter().map(|t| t.map(|p| transform(&inst.matrix, p))).collect();
        let mut lo = [f32::MAX; 3]; let mut hi = [f32::MIN; 3];
        for t in &tris { for p in t { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } } }
        eprintln!("[i{i}] \"{}\" objflags {:#x} AABB lo ({:.1},{:.1},{:.1}) hi ({:.1},{:.1},{:.1})",
            obj.name.trim(), obj.flags, lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
        for (ti, t) in tris.iter().enumerate().take(24) {
            eprintln!("    t{ti}: ({:.1},{:.1},{:.1}) ({:.1},{:.1},{:.1}) ({:.1},{:.1},{:.1})",
                t[0][0], t[0][1], t[0][2], t[1][0], t[1][1], t[1][2], t[2][0], t[2][1], t[2][2]);
        }
    }
}

/// The pack scripts' content: 0x06000077 (the station intro) and 0x06000078 (the ride) — entities,
/// events (channel writes), sounds and the per-stream key extents (where the cut-scene moves things).
#[test]
fn probe_pack_scripts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_ref().unwrap();
    let data = read_archive(&iso, "07000008.bin");
    let level = Level::parse(&data).expect("parse");
    let name_of = |h: u32| level.by_hash.get(&h).map(|(m, o)| level.maps[*m].objects[*o].name.trim().to_string());
    for hash in [0x0600_0077u32, 0x0600_0078, 0x0600_004c] {
        let Some(nis) = sim.drones.scripts.lib.get(&hash) else { eprintln!("[{hash:08x}] NOT in the script lib"); continue };
        eprintln!("=== script {hash:08x}: camera {} loop {} length {} streams {} keys {} ===",
            nis.camera, nis.loop_start, nis.length, nis.streams.len(), nis.keys.len());
        for (si, st) in nis.streams.iter().enumerate() {
            eprintln!("[stream {si}] keys {}..{} ({} keys)", st.first_key, st.last_key, st.last_key.saturating_sub(st.first_key) + 1);
            for cmd in st.decode() {
                match cmd {
                    nf_formats::nis::Cmd::Entity { hash, parent, spline, obj_type, params } =>
                        eprintln!("    Entity {hash:08x} \"{}\" parent {parent:?} spline {spline} type {obj_type} params {params:x?}", name_of(hash).unwrap_or_default()),
                    nf_formats::nis::Cmd::Anim { script, skin, start } =>
                        eprintln!("    Anim script {script:08x} skin {skin:08x} start {start}"),
                    nf_formats::nis::Cmd::Event { kind, params } =>
                        eprintln!("    Event kind {kind} params {params:x?}"),
                    nf_formats::nis::Cmd::Sound { id, flat, has_obj, stream } =>
                        eprintln!("    Sound {id:#x} flat {flat} obj {has_obj} stream {stream}"),
                    nf_formats::nis::Cmd::Camera { spline, first, last } =>
                        eprintln!("    Camera spline {spline} keys {first}..{last}"),
                    nf_formats::nis::Cmd::Wait(t) => eprintln!("    Wait {t}"),
                    nf_formats::nis::Cmd::End(t) => eprintln!("    End {t}"),
                    nf_formats::nis::Cmd::Marker(t) => eprintln!("    Marker {t}"),
                    nf_formats::nis::Cmd::SubScript { hash, stream, mode } =>
                        eprintln!("    SubScript {hash:08x} stream {stream} mode {mode}"),
                    other => eprintln!("    {other:?}"),
                }
            }
            // The key extents: first and last key positions per stream.
            if st.last_key >= st.first_key && (st.last_key as usize) < nis.keys.len() {
                let a = &nis.keys[st.first_key as usize];
                let b = &nis.keys[st.last_key as usize];
                eprintln!("    key[{}] t{:.1} ({:.2},{:.2},{:.2})  → key[{}] t{:.1} ({:.2},{:.2},{:.2})",
                    st.first_key, a.time, a.pos[0], a.pos[1], a.pos[2], st.last_key, b.time, b.pos[0], b.pos[1], b.pos[2]);
            }
        }
    }
}

#[test]
fn probe_walk_down_the_stairs() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let data = read_archive(&iso, "07000008.bin");
    let level = Level::parse(&data).expect("parse");
    let (insts, _) = level.instances(true);
    // Per-instance collision worlds, built lazily.
    let mut mini: std::collections::HashMap<usize, nf_game::world::CollisionWorld> = Default::default();

    let sim = scene.sim.as_mut().unwrap();
    // The opening cut-scene: step until Bond is free.
    for t in 0..3000u32 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { eprintln!("[intro] free at t{t}"); break; } }
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    eprintln!("[start] bond at {:?}", sim.bond.pos);

    // Walk the real route: spawn → intro plate #237 (139.2,49.5,−81.2) → the shutters
    // (139.5,49.5,−96.8) → down the stairs → the walkway triggers (125.0,47.6,−101.1)…
    let waypoints: Vec<[f32; 3]> = vec![
        [139.2, 49.5, -81.2],   // the intro plate
        [139.5, 49.5, -95.0],   // toward the shutters
        [139.5, 48.0, -99.0],   // through the shutters, down the stairs
        [133.4, 48.3, -101.2],  // the walkway (Hand_Over_Hand #105)
        [125.0, 47.6, -101.1],  // the walkway (TouchOnce out 91)
        [118.7, 35.0, -113.2],  // the lower station start #8 (the far end)
    ];
    let mut total_stuck = 0usize;
    for (wi, target) in waypoints.iter().enumerate() {
        eprintln!("=== waypoint {wi} ({:.1},{:.1},{:.1}) ===", target[0], target[1], target[2]);
        let mut stuck: Vec<String> = Vec::new();
        let (mut cutscene_ticks, mut was_disabled) = (0u32, false);
        let mut tries = 0u32;
        for t in 1..=9000u32 {
            let b = &sim.bond;
            let dx = target[0] - b.pos[0]; let dz = target[2] - b.pos[2];
            let dist = (dx * dx + dz * dz).sqrt();
            if dist < 1.0 { eprintln!("[wp{wi}] reached at t{t}"); break; }
            let want = dx.atan2(dz);
            let mut input = Input { forward: 1.0, ..Default::default() };
            let mut diff = want - b.yaw;
            while diff > std::f32::consts::PI { diff -= std::f32::consts::TAU; }
            while diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
            input.turn = diff.clamp(-0.05, 0.05);
            let before = b.pos;
            // Stuck against a closed door? Press E (the use key) for a stretch, then walk on.
            let press_e = stuck.len() > 40 && stuck.len() < 100;
            sim.step_bond(input, false, false, press_e);
            let moved = ((sim.bond.pos[0] - before[0]).powi(2) + (sim.bond.pos[2] - before[2]).powi(2)).sqrt();
            if sim.disabled { cutscene_ticks += 1; was_disabled = true; }
            else if was_disabled && cutscene_ticks > 0 { eprintln!("[wp{wi}] cut-scene released Bond after {cutscene_ticks} ticks"); was_disabled = false; }
            if t % 600 == 0 {
                eprintln!("[wp{wi} t{t}] pos ({:.2},{:.2},{:.2}) yaw {:.2} moved {moved:.3} dist {dist:.1}",
                    sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2], sim.bond.yaw);
            }
            if moved < 0.0025 {
                if sim.disabled {
                    // The station intro (SP #294, 0x06000077, 3880 ticks) holds Bond — not a wall.
                    stuck.clear();
                } else {
                    stuck.push(format!("t{t} ({:.2},{:.2},{:.2})", sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]));
                    if stuck.len() == 41 {
                        tries += 1;
                        eprintln!("[wp{wi}] blocked at ({:.2},{:.2},{:.2}) try {tries} — doors near:",
                            sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]);
                        for (k, d) in sim.drones.doors.list.iter().enumerate() {
                            let p = d.matrix[3];
                            if (p[0] - sim.bond.pos[0]).abs() < 6.0 && (p[2] - sim.bond.pos[2]).abs() < 6.0 {
                                eprintln!("  door {k}: glist {:#x} matrix t ({:.2},{:.2},{:.2})", d.glist, p[0], p[1], p[2]);
                            }
                        }
                    }
                    if stuck.len() > 150 { eprintln!("[wp{wi}] STUCK at {:?}", sim.bond.pos); break; }
                }
            } else { stuck.clear(); }
        }
        if stuck.len() > 150 { total_stuck += 1; break; }
    }
    eprintln!("--- final pos {:?} (stuck waypoints: {total_stuck}) ---", sim.bond.pos);

    // Find the blocker at the stuck spot: (a) per-instance capsule push, (b) a forward segment
    // from the chest, (c) teleport past the spot and walk on.
    let here = sim.bond.pos;
    let facing = [sim.bond.yaw.sin(), 0.0, sim.bond.yaw.cos()]; // yaw 0 faces +Z [M Bond doc]
    let mut hits: Vec<(f32, usize, String)> = Vec::new();
    for (i, inst) in insts.iter().enumerate() {
        let obj = &level.maps[inst.map].objects[inst.object];
        if obj.collision.is_none() { continue }
        let c = transform(&inst.matrix, obj.centre);
        let r = obj.radius + 2.0;
        if (c[0] - here[0]).abs() > r || (c[1] - here[1]).abs() > r || (c[2] - here[2]).abs() > r { continue }
        let w = mini.entry(i).or_insert_with(|| nf_game::world::CollisionWorld::build(&level, std::slice::from_ref(&insts[i])));
        let (push, _) = w.capsule_push([here[0], here[1] + 1.0, here[2]], [here[0], here[1] + 0.1, here[2]], 0.55);
        let m = push.iter().map(|v| v * v).sum::<f32>().sqrt();
        // A forward segment at chest and knee height.
        let seg = |y: f32| w.segment([here[0], here[1] + y, here[2]], [here[0] + facing[0] * 2.0, here[1] + y, here[2] + facing[2] * 2.0]);
        let f = seg(1.0).or_else(|| seg(0.5)).or_else(|| seg(0.2));
        if m > 1e-4 || f.is_some() { hits.push((m.max(f.unwrap_or(0.0)), i, obj.name.trim().to_string())); }
    }
    hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    eprintln!("--- pushers/blockers at the stuck spot ({:.2},{:.2},{:.2}) facing ({:.2},{:.2}) ---", here[0], here[1], here[2], facing[0], facing[2]);
    for (m, i, n) in hits.iter().take(12) {
        let inst = &insts[*i];
        eprintln!("  {m:.3} [i{i}] \"{n}\" at ({:.1},{:.1},{:.1}) pflags {:#x} params {:?}",
            inst.placement.translation[0], inst.placement.translation[1], inst.placement.translation[2], inst.placement.flags, inst.placement.params);
    }
    // (c) Teleport past the stuck z and try walking again.
    sim.bond.pos = [here[0], here[1], -83.0];
    let before = sim.bond.pos;
    for _ in 0..120 {
        let b = &sim.bond;
        let dx = -81.2 - b.pos[0]; let dz = -81.2 - b.pos[2];
        let want = dx.atan2(dz);
        let mut input = Input { forward: 1.0, ..Default::default() };
        let mut diff = want - b.yaw;
        while diff > std::f32::consts::PI { diff -= std::f32::consts::TAU; }
        while diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
        input.turn = diff.clamp(-0.05, 0.05);
        sim.step_bond(input, false, false, false);
    }
    eprintln!("[teleport to (x,{here_y},-83)] after 120 ticks pos {:?}", sim.bond.pos, here_y = here[1]);
    eprintln!("  moved {:.3}", ((sim.bond.pos[0] - before[0]).powi(2) + (sim.bond.pos[2] - before[2]).powi(2)).sqrt());
}
