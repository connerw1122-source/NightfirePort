//! fix-window-view (101i): the view out of the Mayhew ballroom window on 07000007.
//!
//! The PCSX2 comparison [L docs/live/2026-10-13-portal-ground-truth.md, the ballroom-window row]:
//! the real game shows only the snowy mountain through the window — no lit building — and the port's
//! walk matches; with `NF_NOPORTAL=1` the building draws. The port also shows the snow smeared /
//! stretched with no haze, while the game shows a blue haze over the slope.
//!
//! Evidence base:
//! - `psiPreGameRun` (0x11e2e8): 07000007 falls into `default` — `FogOn = 0`, `Ps2Far1 = 4194240`
//!   [M]. `psiDrawFogPass` (0x10b960) gates the screen fog on `FogOn != 0` [M], so the game's blue
//!   haze on 07000007 cannot be the screen pass; whatever paints it must ride the drawn content.
//! - The walk: `eye_cell` (Camera_CheckLocation 0x1bae90 state) → `Vision_Portal_Recurse` 0x1e86e8
//!   [M]; the renderer's World cull (gpu.rs render) gates `node`-attached draws on the cell game
//!   mask and the rest on their room cel under the strict mask.
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near_p, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near_p - far), -1.0],
        [0.0, 0.0, near_p * far / (near_p - far), 0.0],
    ]);
    let cp = pitch.cos();
    let fw = Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let view = Mat4::from_cols_array_2d(&[
        [s.x, u.x, -fw.x, 0.0],
        [s.y, u.y, -fw.y, 0.0],
        [s.z, u.z, -fw.z, 0.0],
        [-s.dot(Vec3::from(pos)), -u.dot(Vec3::from(pos)), fw.dot(Vec3::from(pos)), 1.0],
    ]);
    (proj * view).to_cols_array_2d()
}

fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

/// The ballroom window: `--bond 61.32,44.11,-52.58,3.749,-0.056` (the ground-truth doc's spot is
/// the same window at 61.11,44.12,-52.58,-2.541,-0.010; −2.541 + 2π ≈ 3.742). The bond y + 0.7 is
/// the camera head; `--bond`'s pitch is the game's half-radian word — the render path multiplies by
/// π/2.
const WINDOW: ([f32; 3], f32, f32) = ([61.32, 44.11 + 0.7, -52.58], 3.749, -0.056 * std::f32::consts::FRAC_PI_2);

#[derive(Clone, Copy)]
struct Drawn {
    mi: usize,
    centre: [f32; 3],
    radius: f32,
    dist2: f32,
    node: u32,
    cel: Option<u16>,
    key: Option<(usize, u32)>,
    pass: u8,
    uv: ([f32; 2], [f32; 2]),
    nverts: usize,
}

fn uv_range(sc: &scene::Scene, mi: usize) -> (([f32; 2], [f32; 2]), usize) {
    let m = &sc.materials[mi];
    let mut lo = [f32::MAX; 2];
    let mut hi = [f32::MIN; 2];
    let mut n = 0usize;
    for t in sc.indices[m.first_index as usize..(m.first_index + m.index_count) as usize].chunks(3) {
        for &k in t {
            let uv = sc.vertices[k as usize].uv;
            for a in 0..2 { lo[a] = lo[a].min(uv[a]); hi[a] = hi[a].max(uv[a]); }
            n += 1;
        }
    }
    ((lo, hi), n)
}

fn collect(sc: &scene::Scene, walk: bool) -> Vec<Drawn> {
    let (eye, yaw, pitch) = WINDOW;
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let (cell_vis, room_vis) = if walk {
        let depth0 = depth0_game(vp, eye);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let world = sc.sim.as_ref().map(|s| &s.world);
        let mut state = scene::ViewerCel::default();
        let cell = scene::eye_cell(&sc.cel_graph, &mut state, eye, world).expect("the eye cel resolves");
        let game = scene::portal_visible_cels_game_from_cell(&sc.cel_graph, cell, eye, &depth0, &sides).expect("game mask");
        let strict = scene::portal_visible_cels_from_cell(&sc.cel_graph, cell, eye, &depth0).expect("strict mask");
        let rooms: Vec<bool> = sc.cel_graph.room_to_cell.iter().map(|m| m.map(|c| strict[c]).unwrap_or(true)).collect();
        (Some(game), Some(rooms))
    } else { (None, None) };
    let mut out = Vec::new();
    for (mi, m) in sc.materials.iter().enumerate() {
        if m.layer != scene::Layer::World && m.draw.pass != 0 && m.draw.pass != 4 { continue; }
        let d2 = (m.centre[0] - eye[0]).powi(2) + (m.centre[1] - eye[1]).powi(2) + (m.centre[2] - eye[2]).powi(2);
        if nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { continue; }
        if m.layer != scene::Layer::World { // sky passes are never cel-culled
            let (uv, n) = uv_range(sc, mi);
            out.push(Drawn { mi, centre: m.centre, radius: m.radius, dist2: d2, node: m.node, cel: m.cel, key: m.key, pass: m.draw.pass, uv, nverts: n });
            continue;
        }
        let hidden = match (&cell_vis, m.node) {
            (Some(cv), n) if n != u32::MAX => !cv[n as usize],
            _ => m.cel.is_some_and(|c| room_vis.as_ref().map(|r| !r[c as usize]).unwrap_or(false)),
        };
        if hidden { continue; }
        let (uv, n) = uv_range(sc, mi);
        out.push(Drawn { mi, centre: m.centre, radius: m.radius, dist2: d2, node: m.node, cel: m.cel, key: m.key, pass: m.draw.pass, uv, nverts: n });
    }
    out.sort_by(|a, b| a.dist2.total_cmp(&b.dist2));
    out
}

fn name_at(sc: &scene::Scene, c: [f32; 3]) -> String {
    let mut best: Option<(&String, f32)> = None;
    for (name, lo, hi) in &sc.colliders {
        if (0..3).all(|k| c[k] >= lo[k] - 0.5 && c[k] <= hi[k] + 0.5) {
            let mid = [0.0; 3];
            let d = (0..3).map(|k| (c[k] - mid[k]).powi(2)).sum::<f32>();
            let d = (0..3).map(|k| (c[k] - (lo[k] + hi[k]) / 2.0).powi(2)).sum::<f32>().sqrt();
            let _ = d;
            if best.map(|(_, bd)| d < bd).unwrap_or(true) { best = Some((name, d)); }
        }
    }
    best.map(|(n, _)| n.clone()).unwrap_or_else(|| "?".into())
}

fn dump(sc: &scene::Scene, tag: &str, list: &[Drawn], limit: usize) {
    println!("== {tag}: {} draws (first {limit} by distance)", list.len());
    for d in list.iter().take(limit) {
        let tex = d.key.map(|(mapi, t)| {
            sc.textures.iter().find(|x| x.key == (mapi, t)).map(|t2| format!("tex {t} {}x{} frames {}", t2.width, t2.height, 1 + t2.more.len()))
                .unwrap_or_else(|| format!("tex {t} (no image)"))
        }).unwrap_or_else(|| "untextured".into());
        println!("  mi={mi} pass={p} c=({x:.1},{y:.1},{z:.1}) r={r:.1} d={d:.1} node={n} cel={cel:?} {tex} uv=u[{u0:.2}..{u1:.2}] v[{v0:.2}..{v1:.2}] nv={nv} name={name}",
            mi = d.mi, p = d.pass, x = d.centre[0], y = d.centre[1], z = d.centre[2], r = d.radius, d = d.dist2.sqrt(),
            n = d.node, cel = d.cel, u0 = d.uv.0[0], u1 = d.uv.1[0], v0 = d.uv.0[1], v1 = d.uv.1[1], nv = d.nverts,
            name = name_at(sc, d.centre));
    }
}

#[test]
fn level_named_objects() {
    use nf_formats::disc;
    use nf_formats::iso9660::IsoImage;
    use nf_formats::level::Level;
    let Some(iso) = iso() else { eprintln!("NF_ISO not set; skip"); return };
    let img = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    let lv = Level::parse(&data).expect("level parse");
    for (mi, m) in lv.maps.iter().enumerate() {
        for (oi, o) in m.objects.iter().enumerate() {
            let n = o.name.to_lowercase();
            if n.contains("sky") || n.contains("mount") || n.contains("haze") || n.contains("cloud") || n.contains("snow") {
                println!("obj map{mi}[{oi}] {:?} centre={:?} r={} flags={:#x}", o.name, o.centre, o.radius, o.flags);
            }
        }
        for (pi, p) in m.placements.iter().enumerate() {
            let (om, oi) = match lv.by_hash.get(&p.hashcode) { Some(x) => *x, None => continue };
            let o = &lv.maps[om].objects[oi];
            let n = o.name.to_lowercase();
            if n.contains("sky") || n.contains("mount") || n.contains("haze") || n.contains("cloud") {
                println!("plc map{mi}[{pi}] {:?} t=({:.1},{:.1},{:.1}) flags={:#x} params={:?}", o.name, p.translation[0], p.translation[1], p.translation[2], p.flags, p.params);
            }
        }
    }
    // World placements near the window in the view direction (the slope candidates).
    let eye = [61.32f32, 44.81, -52.58];
    let (yaw, pitch) = (3.749f32, -0.056f32 * std::f32::consts::FRAC_PI_2);
    let fw = [yaw.cos() * yaw.sin(), pitch.sin(), yaw.cos() * yaw.cos()];
    for (mi, m) in lv.maps.iter().enumerate() {
        for (pi, p) in m.placements.iter().enumerate() {
            let Some((om, oi)) = lv.by_hash.get(&p.hashcode).copied() else { continue };
            let o = &lv.maps[om].objects[oi];
            if o.gfx.leaves.is_empty() { continue; }
            let t = p.translation;
            let d = [t[0] - eye[0], t[1] - eye[1], t[2] - eye[2]];
            let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if dist < 6.0 || dist > 120.0 { continue; }
            let dot = d[0] * fw[0] + d[1] * fw[1] + d[2] * fw[2];
            if dot < 0.86 * dist { continue; }
            println!("slope? map{mi}[{pi}] plc{:#x} {:?} t=({:.1},{:.1},{:.1}) d={:.1} flags={:#x}", p.hashcode, o.name, t[0], t[1], t[2], dist, p.flags);
        }
    }
}

#[test]
fn window_view_drawn_set() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set; skip"); return };
    let sc = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
    println!("fog_for(07000007) = {:?}", scene::fog_for(0x0700_0007));
    let on = collect(&sc, true);
    let off = collect(&sc, false);
    // Totals by pass, and the far half of the walk-ON set (the view through the window).
    let mut by_pass: std::collections::BTreeMap<u8, usize> = Default::default();
    for d in &on { *by_pass.entry(d.pass).or_default() += 1; }
    println!("== walk ON totals by pass: {by_pass:?}");
    let sky: Vec<_> = on.iter().filter(|d| d.pass != 1 && d.pass != 3).copied().collect();
    dump(&sc, "walk ON sky/glass (non-world)", &sky, 20);
    let far: Vec<_> = on.iter().filter(|d| d.dist2 > 3600.0).copied().collect();
    dump(&sc, "walk ON beyond 60 units", &far, 40);
    let on_mi: std::collections::HashSet<usize> = on.iter().map(|d| d.mi).collect();
    let only_off: Vec<_> = off.iter().filter(|d| !on_mi.contains(&d.mi)).copied().collect();
    dump(&sc, "NF_NOPORTAL-ONLY (the floating block candidates)", &only_off, 15);
}
