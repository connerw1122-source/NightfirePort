//! fix-render-artifacts probes: the user's 2026-10-12 screenshot spots. Opt-in via NF_ISO.
//! Spots (from the task):
//! - 07000012 cross   --bond -3.36,1.13,35.13,4.424,-0.124
//! - 07000012 sun     --bond 26.81,1.13,33.17,6.287,-0.097
//! - 0700000c roof    --bond 14.22,16.73,-154.43,9.919,-0.349
//! - 0700000c crane   --bond 14.22,16.29,-147.02,0.973,-0.300
//! - 07000008 doorway --bond 152.35,43.13,-120.76,1.261,-0.126
//! - 07000007 guards  --bond 34.25,44.13,-70.71,1.549,-0.070 (frame 1790)

use nf_formats::{disc, iso9660::IsoImage, level::Level};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn parse_level(iso_path: &str, archive: &str) -> Level {
    let img = IsoImage::open(iso_path).expect("open ISO");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    Level::parse(&data).expect("parse level")
}

fn tex_stats(level: &Level, key: (usize, u32)) -> String {
    level.maps.get(key.0).and_then(|m| m.textures.iter().find(|t| t.index as u32 == key.1)).map_or_else(
        || "TEX MISSING".to_string(),
        |t| {
            let rgba = {
                let pal = t.palette_rgba8();
                t.frame(0).iter().flat_map(|&i| pal[i as usize]).collect::<Vec<u8>>()
            };
            let n = (rgba.len() / 4).max(1);
            let mut s = [0u64; 4];
            let mut amin = 255u64; let mut amax = 0u64;
            for px in rgba.chunks(4) { for k in 0..4 { s[k] += px[k] as u64; } amin = amin.min(px[3] as u64); amax = amax.max(px[3] as u64); }
            let mean: Vec<f64> = s.iter().map(|&v| v as f64 / n as f64).collect();
            format!("{:04x}:{:04x} {}x{} frames {} mean rgb ({:.0},{:.0},{:.0}) a {:.0} [{amin}..{amax}]",
                key.0, key.1, t.width, t.height, t.frames, mean[0], mean[1], mean[2], mean[3])
        },
    )
}

/// Every placement whose object name contains NF_PROBE_NAME (default "shadow"), all types: type, flags,
/// object flags, glist flags, leaves (gs, texture, vertex rgba range) and its translation.
#[test]
fn probe_shadow_objects() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "0700000c.bin".into());
    let needle = std::env::var("NF_PROBE_NAME").unwrap_or_else(|_| "shadow".into());
    let level = parse_level(&iso_path, &archive);
    let m = level.main_map().expect("main map");
    let mut hits = 0;
    for (pi, p) in level.maps[m].placements.iter().enumerate() {
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
        let Some((map, object)) = target else { continue };
        let o = &level.maps[map].objects[object];
        if !o.name.to_lowercase().contains(&needle.to_lowercase()) { continue }
        hits += 1;
        eprintln!("placement {pi} type {:#04x} pflags {:#x} creates_cel {} at {:?} rot {:?} params {:?} -> \"{}\" oflags {:#x} glist centre {:?} r {:.1}",
            p.flags & 0xff, p.flags, p.creates_cel(), p.translation, p.rotation, p.params, o.name.trim(), o.flags, o.centre, o.radius);
        for (li, l) in o.gfx.leaves.iter().enumerate() {
            for b in &l.batches {
                let (alo, ahi) = b.verts.iter().fold((255u8, 0u8), |(lo, hi), v| (lo.min(v.rgba[3]), hi.max(v.rgba[3])));
                let rgb0 = b.verts.first().map(|v| v.rgba).unwrap_or([0; 4]);
                let nverts = b.verts.len();
                let lo = b.verts.iter().fold([f32::MAX; 3], |a, v| [a[0].min(v.pos[0]), a[1].min(v.pos[1]), a[2].min(v.pos[2])]);
                let hi = b.verts.iter().fold([-f32::MAX; 3], |a, v| [a[0].max(v.pos[0]), a[1].max(v.pos[1]), a[2].max(v.pos[2])]);
                eprintln!("  leaf {li} gs {:x?} tex {} verts {nverts} rgba0 {rgb0:?} a {alo}..{ahi} box ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) uv0 {:?}",
                    l.gs, tex_stats(&level, (map, b.texture.unwrap_or(u32::MAX))), lo[0], lo[1], lo[2], hi[0], hi[1], hi[2],
                    b.verts.first().map(|v| v.uv).unwrap_or([0.0; 2]));
            }
        }
    }
    eprintln!("probe_shadow_objects {archive} \"{needle}\": {hits} placements");
}

/// Draw-list materials near NF_PROBE_AT with the FULL scene (`--all` build: dynamics included), with nearest
/// placement name. For the untextured-quad spots.
#[test]
fn probe_scene_near_a_point_all() {
    let Some(iso_path) = iso() else { return };
    let Ok(at) = std::env::var("NF_PROBE_AT") else { return };
    let p: Vec<f32> = at.split(',').map(|v| v.parse().unwrap()).collect();
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "0700000c.bin".into());
    let scene = nf_view::load(&iso_path, &archive, true).expect("load");
    let level = parse_level(&iso_path, &archive);
    for (mi, m) in scene.materials.iter().enumerate() {
        let tris = &scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
        for &i in tris { let v = &scene.vertices[i as usize]; for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } }
        let d2: f32 = (0..3).map(|k| { let c = p[k].clamp(lo[k], hi[k]); (c - p[k]).powi(2) }).sum();
        if d2 > 9.0 || hi[0] - lo[0] > 300.0 { continue }
        let tex = m.key.and_then(|k| scene.textures.iter().find(|t| t.key == k));
        let (amin, amax) = tex.map_or((0, 0), |t| t.rgba.chunks(4).fold((255u8, 0u8), |(a, b), px| (a.min(px[3]), b.max(px[3]))));
        let vc = scene.vertices[tris[0] as usize].rgba;
        let (mut best, mut name) = (f32::MAX, String::new());
        for i in level.scene_instances(true).0.iter().chain(level.instances(false).0.iter()) {
            let o = &level.maps[i.map].objects[i.object];
            let d = (o.centre[0] - m.centre[0]).powi(2) + (o.centre[1] - m.centre[1]).powi(2) + (o.centre[2] - m.centre[2]).powi(2);
            if d < best { best = d; name = format!("\"{}\" flags {:#x}", o.name.trim(), o.flags); }
        }
        let fb = if tex.is_some() { "" } else { "WHITE-FALLBACK" };
        eprintln!("m{mi:5} {lm:?} {dm:?} tex {tk:?}[{fb} a {amin}..{amax}] vc {vc:?} box ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) chain {ch} {best:.1}u {name}",
            lo[0], lo[1], lo[2], hi[0], hi[1], hi[2], lm = m.layer, dm = m.draw, tk = m.key, ch = m.chain);
    }
}

/// Identify the characters that own the materials near NF_PROBE_AT (char-tail vertices): character index,
/// skin hash, script stream and matrix position. Opt-in via NF_ISO.
#[test]
fn probe_characters_near_a_point() {
    let Some(iso_path) = iso() else { return };
    let Ok(at) = std::env::var("NF_PROBE_AT") else { return };
    let p: Vec<f32> = at.split(',').map(|v| v.parse().unwrap()).collect();
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "0700000c.bin".into());
    let scene = nf_view::load(&iso_path, &archive, true).expect("load");
    eprintln!("=== {} characters ===", scene.characters.len());
    for (ci, c) in scene.characters.iter().enumerate() {
        let m = &c.matrix;
        let d = ((m[3][0] - p[0]).powi(2) + (m[3][1] - p[1]).powi(2) + (m[3][2] - p[2]).powi(2)).sqrt();
        if d > 4000.0 { continue }
        eprintln!("char {ci}: skin {:#09x} script {:?} role {:?} at ({:.1},{:.1},{:.1}) d {d:.1} floor {:?}",
            c.skin.hashcode, c.script, c.role, m[3][0], m[3][1], m[3][2], c.floor);
    }
    // Materials near the point owned by characters.
    for (mi, m) in scene.materials.iter().enumerate() {
        let tris = &scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
        let mut chars = std::collections::BTreeSet::new();
        for &i in tris {
            let v = &scene.vertices[i as usize];
            for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); }
            if i as usize >= scene.char_vertex_start { chars.insert(scene.char_verts[i as usize - scene.char_vertex_start].character); }
        }
        let d2: f32 = (0..3).map(|k| { let c = p[k].clamp(lo[k], hi[k]); (c - p[k]).powi(2) }).sum();
        if d2 > 9.0 || hi[0] - lo[0] > 300.0 { continue }
        let vc = scene.vertices[tris[0] as usize].rgba;
        eprintln!("m{mi:5} {lm:?} {dm:?} tex {tk:?} vc {vc:?} chars {chars:?} box ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1})",
            lo[0], lo[1], lo[2], hi[0], hi[1], hi[2], lm = m.layer, dm = m.draw, tk = m.key);
    }
}

/// The RUNTIME draw list near NF_PROBE_BOND=x,y,z,yaw,pitch after NF_PROBE_TICKS idle steps (default 1790,
/// the user's screenshot frame): every material whose posed box reaches within 4 units of the eye, World statics
/// included, with draw state, texture, posed vertex colour and the owning character's drone state. This is the
/// user's screenshot reproduced headlessly. Opt-in via NF_ISO.
#[test]
fn probe_play_draws_near() {
    let Some(iso_path) = iso() else { return };
    let Ok(bond) = std::env::var("NF_PROBE_BOND") else { return };
    let b: Vec<f32> = bond.split(',').map(|v| v.parse().unwrap()).collect();
    let ticks: u32 = std::env::var("NF_PROBE_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(1790);
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "0700000c.bin".into());
    let mut scene = nf_view::load(&iso_path, &archive, false).expect("load");
    let sim = scene.sim.as_mut().expect("sim");
    sim.bond.pos = [b[0], b[1], b[2]];
    sim.bond.yaw = *b.get(3).unwrap_or(&0.0);
    sim.bond.pitch = *b.get(4).unwrap_or(&0.0);
    for _ in 0..ticks { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
    let eye = sim.bond.head();
    eprintln!("=== after {ticks} ticks: bond {:?} yaw {:.3} eye {eye:?} ===", sim.bond.pos, sim.bond.yaw);
    let opts = nf_view::scene::ViewOpts::default();
    let posed = scene.posed_vertices(None, ticks as f32, opts);
    let start = scene.char_vertex_start;
    // Posed tail boxes per material.
    for (mi, m) in scene.materials.iter().enumerate() {
        let is_char = m.layer == nf_view::scene::Layer::Character || m.layer == nf_view::scene::Layer::Effect;
        let tris = &scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
        let mut chars = std::collections::BTreeSet::new();
        let mut first_rgba = [0u8; 4];
        let mut seen = 0;
        for &i in tris {
            let v = if is_char && i as usize >= start { &posed[i as usize - start] } else { &scene.vertices[i as usize] };
            if seen == 0 { first_rgba = v.rgba; }
            seen += 1;
            for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); }
            if is_char && i as usize >= start {
                let cv = &scene.char_verts[i as usize - start];
                if let Some(a) = cv.attachment { chars.insert(10_000 + a as u32); } else { chars.insert(cv.character); }
            }
        }
        let d2: f32 = (0..3).map(|k| { let c = eye[k].clamp(lo[k], hi[k]); (c - eye[k]).powi(2) }).sum();
        if d2 > 16.0 { continue }
        let tex = m.key.and_then(|k| scene.textures.iter().find(|t| t.key == k));
        let tstat = tex.map_or("NONE".to_string(), |t| {
            let n = (t.rgba.len() / 4).max(1) as f32;
            let mean: [f32; 4] = [0, 1, 2, 3].map(|c| t.rgba.chunks(4).map(|p| p[c] as f32).sum::<f32>() / n);
            format!("{:04x}:{:04x} rgb({:.0},{:.0},{:.0}) a({:.0})", m.key.unwrap().0, m.key.unwrap().1, mean[0], mean[1], mean[2], mean[3])
        });
        let dr_state = chars.iter().filter_map(|&c| {
            if c >= 10_000 { return None }
            let s = scene.sim.as_ref()?;
            s.char_drone.get(c as usize).copied().flatten().map(|di| {
                let d = &s.drones.list[di];
                format!("c{c}->d{di} present {} enabled {} started {} at {:.1},{:.1},{:.1}", d.present, d.enabled, d.started, d.pos[0], d.pos[1], d.pos[2])
            }).or_else(|| scene.characters.get(c as usize).map(|ch| format!("c{} no-drone role {:?}", c, ch.role)))
        }).collect::<Vec<_>>();
        eprintln!("m{mi:5} {lm:?} {dm:?} tex {tstat} vc {first_rgba:?} chars {chars:?} box ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) {}",
            lo[0], lo[1], lo[2], hi[0], hi[1], hi[2], dr_state.join("; "), lm = m.layer, dm = m.draw);
    }
}

/// Cull-hole finder: replicate the app's camera at NF_PROBE_BOND, extract the frustum, and print every World
/// static whose real geometry is IN the frustum but which the renderer would skip (sphere cull, or the cel-box
/// cull) — each such draw is a hole in the level that the sky pass fills (the pale shapes). Opt-in via NF_ISO.
#[test]
fn probe_cull_holes() {
    use glam::{Mat4, Vec3};
    use nf_view::gpu::{box_outside, extract_frustum, sphere_outside};
    let Some(iso_path) = iso() else { return };
    let Ok(bond) = std::env::var("NF_PROBE_BOND") else { return };
    let b: Vec<f32> = bond.split(',').map(|v| v.parse().unwrap()).collect();
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "0700000c.bin".into());
    let scene = nf_view::load(&iso_path, &archive, false).expect("load");
    let (near, far) = (0.05f32, 4000.0f32);
    let fov = std::f32::consts::FRAC_PI_3;
    let pitch = b.get(4).copied().unwrap_or(0.0) * std::f32::consts::FRAC_PI_2;
    let yaw = b.get(3).copied().unwrap_or(0.0);
    let eye = [b[0], b[1] + 0.7, b[2]];
    let fw = Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let f = 1.0 / (fov * 0.5).tan();
    let aspect = 16.0f32 / 9.0;
    let e = Vec3::from(eye);
    let proj = Mat4::from_cols_array_2d(&[[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]]);
    let view = Mat4::from_cols_array_2d(&[[s.x, u.x, -fw.x, 0.0], [s.y, u.y, -fw.y, 0.0], [s.z, u.z, -fw.z, 0.0], [-s.dot(e), -u.dot(e), fw.dot(e), 1.0]]);
    let vp = (proj * view).to_cols_array_2d();
    let planes = extract_frustum(vp);
    eprintln!("=== cull holes at eye {eye:?} yaw {yaw:.3} pitch {pitch:.3} ({} World draws) ===", scene.materials.iter().filter(|m| m.layer == nf_view::scene::Layer::World).count());
    let level = parse_level(&iso_path, &archive);
    for (mi, m) in scene.materials.iter().enumerate() {
        if m.layer != nf_view::scene::Layer::World { continue }
        let tris = &scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
        for &i in tris { let v = &scene.vertices[i as usize]; for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } }
        let vis_geom = !box_outside(&planes, lo, hi);
        if !vis_geom { continue }
        let hole_sphere = sphere_outside(&planes, m.centre, m.radius);
        let hole_cel = m.cel.and_then(|c| scene.cels.get(c as usize)).map(|c| box_outside(&planes, c.min, c.max)).unwrap_or(false);
        if !hole_sphere && !hole_cel { continue }
        let (mut best, mut name) = (f32::MAX, String::new());
        for inst in level.instances(true).0 {
            let o = &level.maps[inst.map].objects[inst.object];
            let d = (o.centre[0] - m.centre[0]).powi(2) + (o.centre[1] - m.centre[1]).powi(2) + (o.centre[2] - m.centre[2]).powi(2);
            if d < best { best = d; name = format!("\"{}\"", o.name.trim()); }
        }
        eprintln!("HOLE m{mi:5} {} {} tex {:?} box ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) centre {:?} r {:.1} cel {:?} sphere-culled {hole_sphere} cel-culled {hole_cel} -> {best:.1}u {name}",
            format!("{:?}", m.layer), format!("{:?}", m.draw), m.key, lo[0], lo[1], lo[2], hi[0], hi[1], hi[2], m.centre, m.radius, m.cel);
    }
}

/// Find every object whose batches use texture NF_PROBE_TEX=map,index; print name, placement, leaf gs, verts.
#[test]
fn probe_objects_with_texture() {
    let Some(iso_path) = iso() else { return };
    let Ok(tx) = std::env::var("NF_PROBE_TEX") else { return };
    let t: Vec<u32> = tx.split(',').map(|v| v.parse().unwrap()).collect();
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "07000012.bin".into());
    let level = parse_level(&iso_path, &archive);
    for (mi, map) in level.maps.iter().enumerate() {
        for (oi, o) in map.objects.iter().enumerate() {
            let mut used = false;
            for l in &o.gfx.leaves { for b in &l.batches { if b.texture == Some(t[1]) && mi == t[0] as usize { used = true; } } }
            if !used { continue }
            eprintln!("map {mi} obj {oi} \"{}\" oflags {:#x} centre {:?} r {:.1}", o.name.trim(), o.flags, o.centre, o.radius);
            let placed: Vec<[f32; 3]> = level.main_map().map(|mm| level.maps[mm].placements.iter()
                .filter(|p| if p.by_hash() { level.by_hash.get(&p.hashcode) == Some(&(mi, oi)) } else { p.index as usize == oi })
                .map(|p| p.translation).collect()).unwrap_or_default();
            eprintln!("  placements: {:?}", placed.iter().take(4).collect::<Vec<_>>());
            for (li, l) in o.gfx.leaves.iter().enumerate() {
                for b in &l.batches {
                    if b.texture != Some(t[1]) || mi != t[0] as usize { continue }
                    let (alo, ahi) = b.verts.iter().fold((255u8, 0u8), |(lo, hi), v| (lo.min(v.rgba[3]), hi.max(v.rgba[3])));
                    eprintln!("  leaf {li} gs {:x?} verts {} rgba0 {:?} a {alo}..{ahi} uv0 {:?}", l.gs, b.verts.len(), b.verts.first().map(|v| v.rgba), b.verts.first().map(|v| v.uv));
                }
            }
        }
    }
}

/// Count materials per texture key near NF_PROBE_AT: a decal texture bound by two materials with different
/// draw states = a duplicated draw (the pale overlay candidate). Opt-in via NF_ISO.
#[test]
fn probe_materials_per_texture() {
    let Some(iso_path) = iso() else { return };
    let Ok(at) = std::env::var("NF_PROBE_AT") else { return };
    let p: Vec<f32> = at.split(',').map(|v| v.parse().unwrap()).collect();
    let archive = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "0700000c.bin".into());
    let scene = nf_view::load(&iso_path, &archive, false).expect("load");
    use std::collections::BTreeMap;
    let per_tex: std::cell::RefCell<std::collections::BTreeMap<(usize, u32), Vec<(usize, nf_view::scene::Draw)>>> = std::cell::RefCell::new(Default::default());
    for (mi, m) in scene.materials.iter().enumerate() {
        if m.layer != nf_view::scene::Layer::World { continue }
        let tris = &scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
        for &i in tris { let v = &scene.vertices[i as usize]; for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } }
        let d2: f32 = (0..3).map(|k| { let c = p[k].clamp(lo[k], hi[k]); (c - p[k]).powi(2) }).sum();
        if d2 > 2500.0 { continue }
        if let Some(k) = m.key { per_tex.borrow_mut().entry(k).or_default().push((mi, m.draw)); }
    }
    for (k, ms) in per_tex.into_inner() { if ms.len() > 1 { eprintln!("tex {k:?}: {} materials {:?}", ms.len(), ms.iter().map(|(mi, d)| (*mi, *d)).collect::<Vec<_>>()); } }
}
