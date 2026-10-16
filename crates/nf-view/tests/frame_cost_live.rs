//! frame-cost-guard: headless per-frame CPU cost budgets, so the playtest lag cannot come back.
//!
//! The user's 2026-10-15 playtest ran 07000005 at ~52 ms/frame, 34 ms of which was
//! `Scene::colour_mults` walking every character vertex for the effect pools' attachments (8561
//! attachments on 73 characters); master's "fix the in-game lag" (c49f8b69) computes the
//! multipliers once per pickup and per attachment instead, and `skin_palette` is now ~1.4 ms.
//! This file pins the per-frame CPU phases the `RedrawRequested` handler runs (lib.rs:2460) with
//! median-of-N timings after warm-up on EVERY single-player level archive, and asserts generous
//! budgets so a regression fails a test instead of a playtest. Measured numbers are printed for
//! the report table.
//!
//! What is timed (the per-frame CPU chain, gpu.rs `cel_visibility` → the lib.rs `skin_palette` gate →
//! `Renderer::update_skin`'s CPU half → `hud::overlay`/`menu_overlay`; line numbers as of this branch):
//! - `Scene::frame_state` (scene.rs:5994) — the shared pose/attachment core,
//! - `Scene::colour_mults` (scene.rs:6802) — the fixed per-key ambient multipliers,
//! - `Scene::skin_palette` (scene.rs:6678) — the palette the GPU path uploads,
//! - the `Renderer::cel_visibility` composition (gpu.rs:567) replicated headless with the pub
//!   scene pieces, exactly as `cutscene_models_live.rs::render_census` does — the method itself
//!   needs a wgpu device (its body is pure CPU; only `Renderer::new` demands one),
//! - `hud::overlay` (hud.rs:293) and `menu::menu_overlay` (menu.rs:3980),
//! - `Scene::posed_vertices` (scene.rs:6881) — timed for the record but NOT on the window's
//!   per-frame path: lib.rs reaches it only from `--shot` and `NF_BENCH` (lib.rs:1462).
//!
//! Release-only (`#[cfg(not(debug_assertions))]`): the budgets are meaningless in a debug build.
//! Every test is NF_ISO-gated like the other `_live` tests. The palette/colour hashes printed per
//! level are the bit-identity vehicle for cost fixes: run before and after a change and compare.
//!
//! Sister assertion: `a_deliberately_slow_closure_fails_the_budget_helper` (no ISO, no release
//! gate) proves the budget helper actually fails — a 25 ms closure against a 5 ms budget panics.

use nf_view::scene::{self, Scene, ViewOpts};

/// The median of `runs` timed calls after `warmup` un-timed calls. The budget primitive.
fn median_ms(warmup: u32, runs: u32, mut f: impl FnMut()) -> f64 {
    for _ in 0..warmup { f(); }
    let mut v: Vec<f64> = (0..runs)
        .map(|_| { let t0 = std::time::Instant::now(); f(); t0.elapsed().as_secs_f64() * 1000.0 })
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Asserts one measured phase against its budget and prints the number for the report table.
fn assert_budget(name: &str, budget_ms: f64, measured: f64) {
    eprintln!("[budget] {name}: {measured:.3} ms (budget {budget_ms:.1} ms)");
    assert!(measured <= budget_ms, "{name} took {measured:.3} ms, budget {budget_ms:.1} ms — the per-frame cost regressed");
}

/// FNV-1a over the f32 bits — the palette/colour bit-identity fingerprint (deterministic per build).
fn fnv_f32s<'a>(it: impl Iterator<Item = &'a f32>) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for f in it {
        for b in f.to_bits().to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x1000_0000_01b3); }
    }
    h
}

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// gpu.rs:543 `Renderer::depth0_game` — copied (it is a private method): the game's depth-0 planes.
fn depth0_game(planes: &[[f32; 4]; 6], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

/// render3_live.rs:22 — the start view's projection (the same numbers the window builds).
fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
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

/// The on-foot single-player archives on the disc — levels_sweep_live.rs:26's enumeration.
fn on_foot_levels(iso_path: &str) -> Vec<String> {
    let img = nf_formats::iso9660::IsoImage::open(iso_path).expect("iso");
    let (entries, _) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let mut names: Vec<String> = entries.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .filter(|n| {
            let code = u32::from_str_radix(&n[..8], 16).unwrap_or(0) & 0xff;
            !(0x23..=0x29).contains(&code) && code != 0x00 && n != "07000500.bin" && n != "07000999.bin"
        })
        .collect();
    names.sort(); names.dedup(); names
}

/// One level's per-frame medians, printed and budget-asserted. Returns nothing; panics on breach.
fn time_a_level(iso_path: &str, name: &str) {
    let load_scene = || -> Option<Scene> {
        match nf_view::load(iso_path, name, false) { Ok(s) => Some(s), Err(e) => { eprintln!("[cost] {name} not loadable: {e}"); None } }
    };
    let Some(mut scene) = load_scene() else { return };
    // 120 sim ticks (2 game-seconds) put the level in its live state — guards enabled, the AI
    // running — the same `step_bond` drive the other headless tests use (gpu_skin_live.rs:20).
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    }
    let opts = ViewOpts::default();
    let t = 120.0f32;
    let (layout, _attrs) = scene.skin_layout();
    let (pos, yaw) = scene.start.clone().unwrap_or(([0.0; 3], 0.0));
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);

    // The cel_visibility composition (gpu.rs:567) headless — the room mask the window feeds
    // skin_palette with (the lib.rs gate at 2960), plus the near-LOS rescue the game-mask pass runs.
    let cel_once = || -> Option<Vec<bool>> {
        let planes = nf_view::gpu::extract_frustum(vp);
        if scene.cel_graph.portals.is_empty() { return None; }
        let depth0 = depth0_game(&planes, eye);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let world = scene.sim.as_ref().map(|s| &s.world);
        let mut state = scene::ViewerCel::default();
        let cell = scene::eye_cell(&scene.cel_graph, &mut state, eye, world)?;
        let mut game = scene::portal_visible_cels_game_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, &sides, None);
        if let (Some(m), Some(w)) = (game.as_mut(), world) {
            scene::near_los_rescue(&scene.cel_graph, m, eye, &sides, w, 30.0);
        }
        let strict = scene::portal_visible_cels_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, None)?;
        Some(scene.cel_graph.room_to_cell.iter().map(|m| m.map(|c| strict[c]).unwrap_or(true)).collect())
    };
    let vis = cel_once();

    let chars = scene.characters.len();
    let attach = scene.attachments.len();
    let fs = median_ms(2, 7, || { let _ = scene.frame_state(None, t, opts); });
    let cm = median_ms(2, 7, || { let _ = scene.colour_mults(); });
    let sp = median_ms(2, 7, || { let _ = scene.skin_palette(&layout, None, t, opts, vis.as_deref()); });
    let hud = median_ms(2, 7, || { let _ = nf_view::hud::overlay(&scene, 16.0 / 9.0); });
    let cel = median_ms(2, 7, || { let _ = cel_once(); });
    // Off the window's per-frame path (lib.rs reaches it from --shot / NF_BENCH only) — timed for
    // the record, budget generous: it is the by-design walk-every-vertex debug path.
    let pv = median_ms(1, 5, || { let _ = scene.posed_vertices(None, t, opts); });
    eprintln!("[cost] {name} chars={chars} attach={attach} | frame_state {fs:.3} ms | colour_mults {cm:.3} ms | skin_palette {sp:.3} ms | hud_overlay {hud:.3} ms | cel_vis {cel:.3} ms | posed_vertices {pv:.3} ms (off the per-frame path)");

    // The bit-identity fingerprints: one hash pair at t and one at the next tick, printed so a
    // cost fix can be proven output-identical by diffing the before/after runs.
    for tt in [t, t + 1.0] {
        let (pal, cols) = scene.skin_palette(&layout, None, tt, opts, vis.as_deref());
        let (mults, pickup_mult, attach_mult) = scene.colour_mults();
        eprintln!(
            "[hash] {name} t={tt} palette={:016x}:{} cols={:016x}:{} mults={:016x}:{} picks={} atts={}",
            fnv_f32s(pal.iter().flat_map(|m| m.iter().flat_map(|r| r.iter()))), pal.len(),
            fnv_f32s(cols.iter().flat_map(|c| c.iter())), cols.len(),
            fnv_f32s(mults.iter().flat_map(|m| m.iter())), mults.len(),
            pickup_mult.len(), attach_mult.len(),
        );
    }

    assert_budget(&format!("{name} frame_state"), 8.0, fs);
    assert_budget(&format!("{name} colour_mults"), 8.0, cm);
    assert_budget(&format!("{name} skin_palette"), 8.0, sp);
    assert_budget(&format!("{name} hud_overlay"), 3.0, hud);
    assert_budget(&format!("{name} cel_vis"), 3.0, cel);
    // posed_vertices gets NO budget: it is off the window's per-frame path by construction (its
    // only caller is the NF_BENCH block, lib.rs:1462 — the render path is skin_palette +
    // update_skin), and its cost scales with the level's vertex count (measured 0.4 ms on
    // 07000001's 76-character scene, 420 ms there — level-dependent by design). The render3_live
    // counter test guards its per-key ambient lookups instead.
    eprintln!("[budget] {name} posed_vertices: {pv:.3} ms — no budget: off the per-frame path (NF_BENCH/--shot only)");
}

/// The budgets on every single-player archive: the on-foot set (levels_sweep's enumeration) plus
/// the driving mission archives 07000023-29 (docs/systems/levels.md:91 [D] — the front-end
/// campaign's driving rows 0900000x resolve to these through ResetMap's LevelCode2Img).
#[test]
#[cfg(not(debug_assertions))]
fn frame_budgets_hold_on_every_single_player_archive() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut levels = on_foot_levels(&iso_path);
    assert!(levels.len() >= 27, "expected the 27 on-foot archives, got {}", levels.len());
    for code in 0x23u32..=0x29 { levels.push(format!("{:08x}.bin", 0x0700_0000 | code)); }
    for lvl in &levels { time_a_level(&iso_path, lvl); }
}

/// The in-repo proof that `skin_palette`'s per-attachment fallback loop (scene.rs:6790, removed in
/// this task) was dead: `colour_mults` inserts EVERY attachment index `0..len` whenever it runs,
/// so `attach_mult.contains_key(&a)` was always true inside the loop's `if has` gate (`has` implies
/// sim is Some, and the insert loop runs exactly when sim is Some).
#[test]
fn colour_attach_map_covers_every_slot_on_the_lag_levels() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for lvl in ["07000005.bin", "07000006.bin", "07000007.bin"] {
        let mut scene = match nf_view::load(&iso_path, lvl, false) { Ok(s) => s, Err(e) => { eprintln!("{lvl} not loadable: {e}"); continue } };
        if let Some(sim) = scene.sim.as_mut() { for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); } }
        let (_, _, attach_mult) = scene.colour_mults();
        let n = scene.attachments.len();
        let missing: Vec<u32> = (0..n as u32).filter(|a| !attach_mult.contains_key(a)).collect();
        assert!(missing.is_empty(), "{lvl}: {missing:?} of {n} attachment slots missing from attach_mult — the removed fallback loop is reachable again");
    }
}

/// The budget helper must actually fail: a deliberately slow closure against a 5 ms budget panics
/// (caught here and asserted), and a fast closure passes. Without the slow side a bug in the
/// helper (or a future unit change) could silently stop guarding.
#[test]
fn a_deliberately_slow_closure_fails_the_budget_helper() {
    let slow = std::panic::catch_unwind(|| {
        assert_budget("deliberately slow", 5.0, median_ms(0, 3, || std::thread::sleep(std::time::Duration::from_millis(25))));
    });
    assert!(slow.is_err(), "the budget helper accepted a 25 ms closure against a 5 ms budget");
    let fast = std::panic::catch_unwind(|| {
        assert_budget("deliberately fast", 5.0, median_ms(0, 3, || { std::hint::black_box(1 + 1); }));
    });
    assert!(fast.is_ok(), "the budget helper rejected a sub-microsecond closure against a 5 ms budget");
}

/// The menu overlay builder: every front-end page, the game's own font layout (menus_flow.rs's
/// synthetic font + textures — the builder's cost is the quads it emits, not the texture bytes).
#[test]
#[cfg(not(debug_assertions))]
fn menu_overlay_budgets_hold_on_every_page() {
    use nf_view::menu::{self, FrontEnd, Page};
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    fe.unlock_all = true;
    for page in [Page::Start, Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect, Page::Dossier] {
        fe.page = page;
        let ms = median_ms(2, 7, || { let _ = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0); });
        assert_budget(&format!("menu_overlay {page:?}"), 3.0, ms);
    }
}

fn test_font() -> nf_formats::hud::Font {
    let glyph = |code: u16| nf_formats::hud::Glyph { u: 0, v: 0, w: 8, h: 12, yoff: 0, pre: 0, post: 8, code, kerns: vec![] };
    nf_formats::hud::Font { first: 33, last: 126, gap: 0.0, space: 3.0, glyphs: (33..=126).map(glyph).collect() }
}

fn crate_texture(hash: u32) -> nf_view::scene::TextureImage {
    let (w, h) = if hash == 0x0300_0075 { (256, 128) } else if hash == 0x0300_0005 { (512, 128) } else { (8, 8) };
    nf_view::scene::TextureImage { key: (0, hash), width: w as u32, height: h as u32, rgba: vec![255; (w * h * 4) as usize], more: vec![], rate: 1 }
}

/// drake-cutscene-06: the 07000006 dressing cut-scene (0x06000071, SP #810, the playtest
/// 1015-2142 "drake cutscene on o6 has some lag" report) — 649's budgets extended to a running
/// cut-scene. The scene is driven through the real path (the truck-ride carry, the LoadPoint
/// trigger #746 → ch 0x83), and at three scene ticks (the hold, the mid-swap cut, the closing
/// sweep) the window's per-frame phases are timed with the SCENE's script camera. The sim tick's
/// own median over the scene is budgeted too: the profile found `Pickup_Update`'s port casting a
/// take-ray for every pickup every tick there (1.0 of the 1.7 ms tick — the game's
/// `Pickup_Update` 0x18e148 gates the `Collide_LineOfSight` on the distance; fixed in this
/// task), so the tick budget fails if that class of per-tick waste comes back.
#[test]
#[cfg(not(debug_assertions))]
fn frame_budgets_hold_through_the_07000006_dressing_cutscene() {
    use nf_game::player::{Input, RamSave};
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut ch = nf_game::triggers::Channels::default();
    for &c in &[0x1du32, 0x45, 0x70] { ch.set(c, 1, 0); }
    let save = RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch);
    let mut scene = nf_view::load_with_carry(&iso_path, "07000006.bin", false, Some(&save)).expect("07000006 loads");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 0..60 { sim.step_bond(input, false, false, false); }
    }
    let lp = scene.sim.as_ref().unwrap().drones.triggers.iter().find(|t| t.out == 0x83).expect("LoadPoint #746").clone();
    let c = lp.centre();
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [c[0], c[1] + 1.0, c[2]];
        for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x83) { break; } }
    }
    assert!(scene.sim.as_ref().unwrap().drones.scripts.players.iter()
        .any(|p| p.scripts.contains(&0x0600_0071) && p.state == 2), "the dressing scene did not start");

    // The sim tick's median over 300 scene ticks (the frozen-AI scene state — event 5's
    // Drone_EnableAll(0) at tick 0 [D 0x06000071 stream 8]).
    let tick_ms = median_ms(20, 300, || { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); });

    let opts = ViewOpts::default();
    let (layout, _attrs) = scene.skin_layout();
    for label in ["hold_t100", "swap_t560", "sweep_t1100"] {
        let steps: u32 = label.rsplit("_t").next().and_then(|x| x.parse().ok()).unwrap_or(100);
        { let sim = scene.sim.as_mut().unwrap(); for _ in 0..steps { sim.step_bond(Input::default(), false, false, false); } }
        let (pos, yaw, pitch, fov) = scene.sim.as_ref().and_then(|s| s.script_camera())
            .unwrap_or(([0.0; 3], 0.0, 0.0, 1.0));
        let eye = pos;
        let vp = view_proj(eye, yaw, pitch, fov.max(0.2));
        // The cel_visibility composition headless — the room mask the palette gate consumes
        // (the same recipe `time_a_level` uses; during the scene the eye is the script camera's).
        let cel_once = || -> Option<Vec<bool>> {
            let planes = nf_view::gpu::extract_frustum(vp);
            if scene.cel_graph.portals.is_empty() { return None; }
            let depth0 = depth0_game(&planes, eye);
            let sides = [planes[0], planes[1], planes[2], planes[3]];
            let world = scene.sim.as_ref().map(|s| &s.world);
            let mut state = scene::ViewerCel::default();
            let cell = scene::eye_cell(&scene.cel_graph, &mut state, eye, world)?;
            let mut game = scene::portal_visible_cels_game_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, &sides, None);
            if let (Some(m), Some(w)) = (game.as_mut(), world) { scene::near_los_rescue(&scene.cel_graph, m, eye, &sides, w, 30.0); }
            let strict = scene::portal_visible_cels_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, None)?;
            Some(scene.cel_graph.room_to_cell.iter().map(|m| m.map(|c| strict[c]).unwrap_or(true)).collect())
        };
        let vis = cel_once();
        let fs = median_ms(2, 7, || { let _ = scene.frame_state(None, steps as f32, opts); });
        let cm = median_ms(2, 7, || { let _ = scene.colour_mults(); });
        let sp = median_ms(2, 7, || { let _ = scene.skin_palette(&layout, None, steps as f32, opts, vis.as_deref()); });
        let cel = median_ms(2, 7, || { let _ = cel_once(); });
        eprintln!("[cost] dressing scene {label}: frame_state {fs:.3} ms | colour_mults {cm:.3} | skin_palette {sp:.3} | cel_vis {cel:.3}");
        assert_budget(&format!("dressing {label} frame_state"), 8.0, fs);
        assert_budget(&format!("dressing {label} colour_mults"), 8.0, cm);
        assert_budget(&format!("dressing {label} skin_palette"), 8.0, sp);
        assert_budget(&format!("dressing {label} cel_vis"), 3.0, cel);
    }
    eprintln!("[cost] dressing scene sim tick: {tick_ms:.3} ms (median of 300)");
    assert_budget("dressing scene sim tick (step_bond)", 1.5, tick_ms);
}
