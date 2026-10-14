//! fix-window-view (101i): the view out of the Mayhew ballroom window on 07000007.
//!
//! The PCSX2 comparison [L docs/live/2026-10-13-portal-ground-truth.md, the ballroom-window row]:
//! the real game shows only the snowy mountain through the window — no lit building — and the
//! port's portal walk matches; with `NF_NOPORTAL=1` the building draws. The user's playtest
//! (with the launcher's `NF_NOPORTAL=1`) saw a lit stone block floating in front of the slope,
//! the snow smeared/stretched and no haze. This file pins, through the real input path:
//!
//! 1. **The block** — with the game's cel walk (`Vision_Portal_Recurse` 0x1e86e8, the renderer's
//!    per-frame mask) the block's room (Gallery_K_2: graph cell 26, room cel 20, world centre
//!    (49.1, 45.6, −51.1)) is NOT in the drawn set at the window spot, and IS in it with the walk
//!    forced off (`NF_NOPORTAL`). The game hides the room the same way — the port matches.
//! 2. **The fog** — `psiPreGameRun` (0x11e2e8) puts 07000007 in its `default` arm: `FogOn = 0`,
//!    `Ps2Far1 = 4194240` [M]; `psiDrawFogPass` (0x10b960) runs only `if (FogOn != 0)` [M]. So the
//!    game applies NO screen fog on 07000007 and the port's `fog_for` returns None for it — the
//!    blue haze over the slope is the skybox dome's own dusk-haze textures (map47 tex 177/180/181
//!    horizon bands), not level fog.
//! 3. **The snow's UVs** — the sky-dome materials' UVs in the port's scene equal the disc's glist
//!    UV stream bit for bit (`add_instance` copies `v.uv` verbatim; the cross-check against
//!    nightfire-re's own VIF reader covers every unique object — docs/formats/ps2gfx.md §Results).
//!    The "tiny" UV patches on the dome (tex 178's u[0.48..0.51] v[0.48..0.52] haze band) are the
//!    disc's own data, not a port parse bug.
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

/// The renderer's World-draw cull (gpu.rs render) replicated headlessly: a `node`-attached draw is
/// hidden when the cell game mask hides its cell; the rest hide on their room cel under the strict
/// mask mapped over `room_to_cell`.
fn drawn_world_mis(sc: &scene::Scene, walk: bool) -> (Vec<usize>, Vec<bool>) {
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
        if m.layer != scene::Layer::World { continue; }
        if nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { continue; }
        let hidden = match (&cell_vis, m.node) {
            (Some(cv), n) if n != u32::MAX => !cv[n as usize],
            _ => m.cel.is_some_and(|c| room_vis.as_ref().map(|r| !r[c as usize]).unwrap_or(false)),
        };
        if !hidden { out.push(mi); }
    }
    (out, room_vis.unwrap_or_default())
}

fn load() -> scene::Scene {
    let iso = iso().expect("NF_ISO");
    nf_view::load(&iso, "07000007.bin", false).expect("07000007 loads")
}

/// The block's instance is not in the drawn set at the window spot with the game's walk, and is
/// drawn with `NF_NOPORTAL` (the sister assertion) — the playtest's floating block is the launcher
/// flag, not a port bug. Gallery_K_2's world centre is (49.1, 45.6, −51.1), graph cell 26, room
/// cel 20 (the diagnostic run on this tree).
#[test]
fn block_hidden_by_the_walk() {
    let sc = load();
    let (on, _) = drawn_world_mis(&sc, true);
    let (off, _) = drawn_world_mis(&sc, false);
    let is_block = |mi: usize| {
        let m = &sc.materials[mi];
        m.centre.iter().zip([49.1f32, 45.6, -51.1]).all(|(a, b)| (a - b).abs() < 0.5)
    };
    let block_on: Vec<usize> = on.iter().copied().filter(|mi| is_block(*mi) as usize & 1 == 1).collect();
    let block_off: Vec<usize> = off.iter().copied().filter(|mi| is_block(*mi)).collect();
    assert!(!block_off.is_empty(), "the block draws with NF_NOPORTAL (the reported symptom)");
    assert!(block_on.is_empty(), "the block's {} draws must be hidden by the walk (the game hides the room)", block_on.len());
    // The walk also hides the rest of the cross-building rooms: the whole drawn set shrinks hard.
    assert!(on.len() < off.len() / 2, "the walk-on drawn set ({}) is much smaller than the walk-off one ({})", on.len(), off.len());
}

/// The window view's drawn set with the walk ON contains the two sky domes (the snowy mountain
/// backdrop) — the game shows "only the snowy mountain" [L ground truth] — and their GS state is
/// the plain opaque sky pass (no fog on 07000007).
#[test]
fn window_view_is_the_sky_domes() {
    let sc = load();
    let (eye, yaw, pitch) = WINDOW;
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sky: Vec<(usize, f32)> = sc.materials.iter().enumerate()
        .filter(|(_, m)| matches!(m.layer, scene::Layer::SkyCamera | scene::Layer::SkyFixed))
        .filter(|(_, m)| !nf_view::gpu::sphere_outside(&planes, m.centre, m.radius))
        .map(|(mi, m)| (mi, ((m.centre[0] - eye[0]).powi(2) + (m.centre[1] - eye[1]).powi(2) + (m.centre[2] - eye[2]).powi(2)).sqrt()))
        .collect();
    assert!(sky.len() >= 10, "both domes sit in the window frustum (got {} sky draws)", sky.len());
    // The Mountains_skybox dome (the snowy mountain): r ≈ 244.6, near the eye inside the dome.
    let mountains = sky.iter().filter(|(mi, _)| sc.materials[*mi].radius > 200.0 && sc.materials[*mi].radius < 300.0).count();
    let skybox = sky.iter().filter(|(mi, _)| sc.materials[*mi].radius > 1000.0).count();
    assert!(mountains >= 4, "Mountains_skybox materials in the frustum (got {mountains})");
    assert!(skybox >= 6, "skybox dome materials in the frustum (got {skybox})");
    for (mi, _) in &sky {
        let d = &sc.materials[*mi].draw;
        assert_eq!(d.pass, 4, "the sky draws after the fog pass (param 4 ≠ 1 on 07000007)");
        assert_eq!(d.fog, 0, "no fog flag on the 07000007 sky (FogOn = 0 [M 0x11e2e8])");
    }
}

/// `psiPreGameRun` (0x11e2e8): 07000007 is in the `default` arm — `FogOn = 0`, `Ps2Far1 = 4194240`
/// — so `fog_for` returns None; the sister level 07000008 keeps its fog ([0x3b 0x50 0x69],
/// 13191350.0). `psiDrawFogPass` (0x10b960) gates on `FogOn != 0` [M].
#[test]
fn fog_params_for_07000007() {
    assert_eq!(scene::fog_for(0x0700_0007), None, "FogOn = 0 on 07000007 [M 0x11e2e8 default arm]");
    assert_eq!(scene::fog_for(0x0700_0008), Some(([0x3b, 0x50, 0x69], 13_191_350.0)), "the sister: 07000008's fog [M 0x11e2e8 case 0x7000008]");
    assert_eq!(scene::fog_for(0x0700_0005), Some(([0x3a, 0x4d, 0x68], 19_118_528.0)), "the sister: 07000005's fog [M 0x11e2e8 case 0x7000005]");
    // The dome batches agree: the sky materials carry fog = 0 (checked in window_view_is_the_sky_domes).
}

/// The frame order [M `Game_Draw` 0x1e6f50:114-139]: `psiPostSolid` (the fog pass) →
/// `View_DrawSky(.., 0)` (the after-fog sky, the port's pass 4, line 123) → `View_DrawObjList(.., 1)`
/// (the alpha list, the port's pass 3, line 139). The port drew pass 3 before pass 4 (`dc.pass * 2`
/// ascending), so the far-pinned sky overpainted the window glass — the lost blue haze over the
/// slope on 07000007.
#[test]
fn sky_draws_before_the_alpha_list() {
    assert!(nf_view::gpu::pass_rank(4) < nf_view::gpu::pass_rank(3), "the after-fog sky (pass 4) draws before the alpha list (pass 3) [M Game_Draw 0x1e6f50:123,139]");
    assert!(nf_view::gpu::pass_rank(0) < nf_view::gpu::pass_rank(1), "the pre-world sky (pass 0) draws before the world [M Game_Draw 0x1e6f50:93,108]");
    assert!(nf_view::gpu::pass_rank(1) < nf_view::gpu::pass_rank(4), "the world (pass 1) draws before the after-fog sky [M Game_Draw 0x1e6f50:108,123]");
    assert!(nf_view::gpu::pass_rank(3) < nf_view::gpu::pass_rank(5), "the effect pools draw after the alpha list");
    // Under the pre-fix ranking (pass * 2) the sky ranked 8 vs the alpha list's 6 — the assertion
    // above fails against it, which is the reported symptom's order.
}

/// The snow's UVs match the disc's: for both sky-dome objects (map47's `skybox` 279 and
/// `Mountains_skybox` 280), the multiset of (u, v) bit patterns in the port's scene equals the
/// multiset in the level's parsed glist vertex stream (`add_instance` copies `v.uv` verbatim; the
/// disc stream was cross-checked against nightfire-re's own VIF reader — docs/formats/ps2gfx.md).
/// The disc's own tiny UV patches (the haze band on tex 178) are asserted too: the "smeared"
/// patches are the game's data, not a port parse bug.
#[test]
fn snow_uvs_match_the_disc() {
    use std::collections::BTreeMap;
    let sc = load();
    // The port's side: the sky materials grouped by dome (the two centres from the diagnostic run).
    let domes: [([f32; 3], usize); 2] = [([-179.3, 26.8, 234.4], 0), ([91.8, 30.8, -51.4], 1)];
    let mut port: Vec<BTreeMap<(u32, u32), usize>> = vec![BTreeMap::new(), BTreeMap::new()];
    for (mi, m) in sc.materials.iter().enumerate() {
        if !matches!(m.layer, scene::Layer::SkyCamera | scene::Layer::SkyFixed) { continue; }
        let Some(di) = domes.iter().position(|(c, _)| m.centre.iter().zip(c).all(|(a, b)| (a - b).abs() < 2.0)) else { continue };
        for t in sc.indices[m.first_index as usize..(m.first_index + m.index_count) as usize].chunks(3) {
            for &k in t {
                let uv = sc.vertices[k as usize].uv;
                *port[di].entry((uv[0].to_bits(), uv[1].to_bits())).or_default() += 1;
            }
        }
    }
    // The disc's side: parse the archive and collect the two objects' glist UV streams.
    use nf_formats::disc;
    use nf_formats::iso9660::IsoImage;
    use nf_formats::level::Level;
    let img = IsoImage::open(&iso().expect("NF_ISO")).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    let lv = Level::parse(&data).expect("level parse");
    let map = &lv.maps[47];
    for (oi, di) in [(279usize, 0usize), (280, 1)] {
        let obj = &map.objects[oi];
        let mut disc_uv: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for leaf in &obj.gfx.leaves {
            for b in &leaf.batches {
                // Count the way the port's index buffer counts: per triangle corner (the same
                // strip triangulation the port's exporter uses).
                for t in b.triangles() {
                    for k in t {
                        let uv = b.verts[k as usize].uv;
                        *disc_uv.entry((uv[0].to_bits(), uv[1].to_bits())).or_default() += 1;
                    }
                }
            }
        }
        assert_eq!(port[di], disc_uv, "obj {oi} ({:?}): the port's dome UVs differ from the disc's glist stream", obj.name);
    }
    // The disc's own tiny haze patches: some skybox batch samples a small centre patch of its
    // texture (the diagnostic's tex-178 batch: u[0.48..0.51] v[0.48..0.52]).
    let obj = &map.objects[279];
    let tiny = obj.gfx.leaves.iter().flat_map(|l| &l.batches)
        .filter(|b| b.verts.len() > 20)
        .any(|b| {
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for v in &b.verts { for a in 0..2 { lo[a] = lo[a].min(v.uv[a]); hi[a] = hi[a].max(v.uv[a]); } }
            lo[0] > 0.35 && hi[0] < 0.65 && lo[1] > 0.35 && hi[1] < 0.65 && hi[0] - lo[0] < 0.06 && hi[1] - lo[1] < 0.06
        });
    assert!(tiny, "the disc itself samples the dome haze from a small centre patch of its texture");
}
