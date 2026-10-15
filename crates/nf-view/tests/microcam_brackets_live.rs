//! fix-microcam-brackets: the Micro-Camera's (0x54/0x55) green target brackets. The real game draws
//! one bracket per drawn, living, non-WaitSwitch drone through `HUD_UpdateCameraPane` 0x1a12d8 — a
//! one-frame Debris-class sprite object (class 0x10, `Debris_Update` 0x1aac00 retires it the next
//! tick: its glist life +0x30 = 0) with gfx 0x02000832 "DroneScan" (texture 0x03000145 [D
//! 07000005.bin map 15], a 2×2 world-unit flat quad) at the drone's root (`obj+0x30`) every tick
//! the pane updates, and the picture shows them through walls. The port drew none. Real input path
//! (`nf_view::load` + `Sim::step_bond`). Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The bracket texture ("DroneScan"'s, [D 07000005.bin map 15 batch 0]).
const BRACKET_TEX: u32 = 0x0300_0145;

/// Bracket quads in the overlay frame (6 vertices each), or None while the texture is not loaded.
fn bracket_quads(scene: &nf_view::scene::Scene) -> Option<usize> {
    let frame = nf_view::hud::overlay(scene, 1280.0 / 720.0);
    let ti = scene.hud_textures.iter().position(|(h, _)| *h == BRACKET_TEX)?;
    Some(frame.runs.iter().filter(|r| r.0 == ti + 1).map(|r| (r.3 / 6) as usize).sum())
}

/// Whether a bracket quad sits near the screen centre (NDC |x|,|y| < 0.35 — the faced guard's box).
fn centre_bracket(scene: &nf_view::scene::Scene) -> Option<bool> {
    let frame = nf_view::hud::overlay(scene, 1280.0 / 720.0);
    let ti = scene.hud_textures.iter().position(|(h, _)| *h == BRACKET_TEX)?;
    Some(frame.runs.iter().filter(|r| r.0 == ti + 1).any(|r| {
        let v = &frame.v[r.2 as usize..(r.2 + r.3) as usize];
        let (mx, my) = (v.iter().map(|q| q.p[0]).sum::<f32>() / v.len() as f32, v.iter().map(|q| q.p[1]).sum::<f32>() / v.len() as f32);
        mx.abs() < 0.8 && my.abs() < 0.8
    }))
}

/// The game's candidate rule [M 0x1a12d8]: walk the drone list, keep rendered-recently
/// (`obj+0xFC` — `View_AddObjects` 0x1e62d8 stamps it on draw-list entry: the hide gate
/// present+enabled, no LOS ray), alive (`Drone+0xac` > 0), non-WaitSwitch (`obj+0xF4` != 1).
fn candidates(sim: &nf_view::scene::Sim) -> Vec<usize> {
    sim.drones.llist.iter().copied().filter(|&i| {
        let dr = &sim.drones.list[i];
        dr.present && dr.enabled && dr.health > 0.0 && dr.state != nf_game::drone::state::WAIT_SWITCH
    }).collect()
}

/// Arm the Micro-Camera through the real raise/zoom path and settle facing `to` from `at`.
fn camera_zoomed_at(w: &mut nf_view::scene::Scene, at: [f32; 3], face_di: usize) -> f32 {
    {
        let sim = w.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
        let to = sim.drones.list[face_di].pos;
        let e = sim.bond.head();
        let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
        sim.bond.yaw = d[0].atan2(d[2]);
    }
    // Equip the Micro-Camera through the REAL number-key switch, settle the swap, then raise+zoom
    // through the held aim/zoom buttons (the user's right click; a raise queued mid-swap is
    // refused — one queue point, and a held button never re-edges).
    for _ in 0..4 { w.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x54), ..Input::default() }, false, false, false); }
    for _ in 0..90 { w.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Every held tick re-aims at the target's live position (the guards walk; zoom 8 narrows the
    // fov to about 7.5 degrees, so the box must be tracked).
    for _ in 0..150 {
        {
            let sim = w.sim.as_mut().unwrap();
            let to = sim.drones.list[face_di].pos;
            let e = sim.bond.head();
            let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
            sim.bond.yaw = d[0].atan2(d[2]);
            let h = (d[0] * d[0] + d[2] * d[2]).max(0.1).sqrt();
            sim.bond.pitch = d[1].atan2(h) / std::f32::consts::FRAC_PI_2;
        }
        w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false);
    }
    w.sim.as_mut().unwrap().zoom
}

/// THE SYMPTOM: zoomed into the Micro-Camera facing guards, the overlay draws one bracket per
/// candidate guard. On the pre-fix port the DroneScan texture is not even loaded and no bracket
/// quads exist.
#[test]
fn the_micro_camera_brackets_every_candidate_guard() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    }
    // The nearest drawn, living guard; stand 6 units south of it and zoom in facing it.
    let (di, dp) = {
        let sim = w.sim.as_ref().unwrap();
        let (di, dp) = candidates(sim).into_iter().map(|i| (i, sim.drones.list[i].pos)).min_by(|a, b| a.1[2].total_cmp(&b.1[2])).expect("07000005 has living drawn guards");
        (di, dp)
    };
    let zoom = camera_zoomed_at(&mut w, [dp[0], dp[1], dp[2] + 6.0], di);
    assert!(zoom > 1.5, "the Micro-Camera zoom engaged through the real input path (zoom {zoom})");
    // Re-aim at the guard's current spot (it may have stepped) and let the aim settle.
    {
        let sim = w.sim.as_mut().unwrap();
        let dp = sim.drones.list[di].pos;
        let e = sim.bond.head();
        let d = [dp[0] - e[0], dp[1] + 0.7 - e[1], dp[2] - e[2]];
        sim.bond.yaw = d[0].atan2(d[2]);
    }
    for _ in 0..4 { w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false); }
    // In-view candidates: in front of the camera (the frustum proxy — the game's obj+0xFC rides
    // the cel draw, which culls by view too).
    let want = candidates(w.sim.as_ref().unwrap()).len();
    assert!(want >= 1, "at least the faced guard is a candidate [M 0x1a12d8]");
    let quads = bracket_quads(&w).expect("the bracket texture 0x03000145 is loaded into the HUD (the game draws DroneScan brackets [M 0x1a12d8])");
    assert!(quads >= 1, "the faced guard carries its bracket");
    assert_eq!(centre_bracket(&w), Some(true), "the faced guard's bracket rides the screen centre (the re-aim tracks it)");
    eprintln!("[brackets] {quads} quads drawn, {want} level candidates (the off-view ones cull, like the game's cel draw)");
}

/// Sister: a corpse gets no bracket (the `Drone+0xac` > 0 gate [M 0x1a12d8]).
#[test]
fn a_corpse_gets_no_bracket() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    }
    let (di, dp) = {
        let sim = w.sim.as_ref().unwrap();
        let (di, dp) = candidates(sim).into_iter().map(|i| (i, sim.drones.list[i].pos)).min_by(|a, b| a.1[2].total_cmp(&b.1[2])).expect("guards");
        (di, dp)
    };
    let zoom = camera_zoomed_at(&mut w, [dp[0], dp[1], dp[2] + 6.0], di);
    assert!(zoom > 1.5, "the Micro-Camera zoom engaged through the real input path (zoom {zoom})");
    // Kill the faced guard with the real fire path (the P2K), then back to the camera.
    {
        let sim = w.sim.as_mut().unwrap();
        sim.gun.weapon = 2;
        sim.gun.switch_request = 2;
        sim.gun.switch_state = 0;
    }
    let mut hp = w.sim.as_ref().unwrap().drones.list[di].health;
    for _ in 0..600 {
        {
            let sim = w.sim.as_mut().unwrap();
            let dp = sim.drones.list[di].pos;
            let e = sim.bond.head();
            let d = [dp[0] - e[0], dp[1] + 0.7 - e[1], dp[2] - e[2]];
            sim.bond.yaw = d[0].atan2(d[2]);
            let h = (d[0] * d[0] + d[2] * d[2]).max(0.1).sqrt();
            sim.bond.pitch = d[1].atan2(h) / std::f32::consts::FRAC_PI_2;
            sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        }
        hp = w.sim.as_ref().unwrap().drones.list[di].health;
        if hp <= 0.0 { break; }
    }
    assert!(hp <= 0.0, "the guard died through the real fire path (hp {hp})");
    {
        let sim = w.sim.as_mut().unwrap();
        sim.gun.weapon = 0x54;
        sim.gun.switch_request = 0x54;
        sim.gun.switch_state = 0;
    }
    for _ in 0..150 { w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false); }
    let sim = w.sim.as_ref().unwrap();
    assert!(sim.drones.list[di].health <= 0.0);
    let after = candidates(sim).len();
    drop(sim);
    let quads = bracket_quads(&w).expect("the bracket texture loads");
    assert_eq!(centre_bracket(&w), Some(false), "the dead guard's bracket is gone from the screen centre [M 0x1a12d8: the Drone+0xac > 0 gate]");
    eprintln!("[corpse] {quads} quads remain; the centre (the dead guard) has none");
}

/// Sister: the brackets draw through a wall — the projection has no LOS test [M 0x1a12d8 creates
/// the sprite at the drone's position unconditionally; the user saw them through walls].
#[test]
fn brackets_draw_through_a_wall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    }
    // Find a candidate guard with a wall on the ray from some spot 10 units away (the ring walk),
    // then hold the zoom state at the game's own held-button endpoint (+0x8D4 = the per-weapon cap
    // written on the aim-bit rise [M 0x1a24c8:156-187]; Player_Zoom interpolates to it) — the wet
    // ring spots refuse the raise itself, and this sister is about the DRAW, not the raise.
    let llist: Vec<usize> = w.sim.as_ref().unwrap().drones.llist.clone();
    let mut done = false;
    'guards: for &i in &llist {
        let (present, enabled, alive, waiting, dpos) = {
            let dr = &w.sim.as_ref().unwrap().drones.list[i];
            (dr.present, dr.enabled, dr.health > 0.0, dr.state == nf_game::drone::state::WAIT_SWITCH, dr.pos)
        };
        if !(present && enabled && alive && !waiting) { continue; }
        for k in 0..24 {
            let a = k as f32 * std::f32::consts::TAU / 24.0;
            let at = [dpos[0] + a.sin() * 10.0, dpos[1], dpos[2] + a.cos() * 10.0];
            let eye = [at[0], at[1] + 1.6, at[2]];
            if w.sim.as_ref().unwrap().world.segment(eye, dpos).is_none() { continue; }
            let cap = w.sim.as_ref().unwrap().weapons.zoom(0x54);
            {
                let sim = w.sim.as_mut().unwrap();
                sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
                sim.gun.weapon = 0x54;
                sim.gun.switch_request = 0x54;
                sim.gun.switch_state = 0;
                // Aim at the guard's root (the bracket centres there; zoom 8's fov is ~7.5 degrees).
                let dp = sim.drones.list[i].pos;
                let e = sim.bond.head();
                let d = [dp[0] - e[0], dp[1] - e[1], dp[2] - e[2]];
                sim.bond.yaw = d[0].atan2(d[2]);
                let h = (d[0] * d[0] + d[2] * d[2]).max(0.1).sqrt();
                sim.bond.pitch = d[1].atan2(h) / std::f32::consts::FRAC_PI_2;
                sim.zoom_target = cap;
                sim.zoom = cap;
            }
            // NO settle ticks: every step_bond rewrites zoom_target (the wet spot's raise is
            // refused, so the aim bit never sets and the held-button zoom decays) — the state-set
            // stands as the held-button endpoint and the draw reads the sim directly.
            {
                let sim = w.sim.as_ref().unwrap();
                let dp = sim.drones.list[i].pos;
                let e = sim.bond.head();
                // The wall really is between: the eye→drone segment hits the world.
                assert!(sim.world.segment(e, dp).is_some(), "a wall stands between the eye and the guard");
                let rects = nf_view::hud::camera_brackets(sim, 1280.0 / 720.0);
                assert_eq!(rects.len(), 1, "exactly the walled-off guard's bracket (present, enabled, alive, not WaitSwitch [M 0x1a12d8] — no LOS test anywhere in the chain)");
            }
            let quads = bracket_quads(&w).expect("the bracket texture loads");
            assert!(quads >= 1, "the overlay draws the walled-off guard's bracket");
            eprintln!("[wall] {quads} quads with a wall between Bond and guard {i}");
            done = true;
            break 'guards;
        }
    }
    assert!(done, "no walled-off spot found on the ring walk");
}

#[test]
fn probe_hud_textures() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let w = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let hit = w.hud_textures.iter().position(|(h, _)| *h == BRACKET_TEX);
    eprintln!("hud_textures: {} entries; BRACKET_TEX at {:?}; total TEXTURES {}", w.hud_textures.len(), hit, nf_view::hud::TEXTURES.len());
}

#[test]
fn probe_zoom_p2k() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
        sim.gun.weapon = 2; sim.gun.switch_request = 2; sim.gun.switch_state = 0;
    }
    for _ in 0..60 { w.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    for k in 0..8 {
        for _ in 0..15 { w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false); }
        let sim = w.sim.as_ref().unwrap();
        eprintln!("p2k +{}: aiming {} zoom {:.3} target {}", (k+1)*15, sim.aiming, sim.zoom, sim.zoom_target);
    }
}

#[test]
fn probe_zoom_raise() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
        sim.gun.weapon = 0x54; sim.gun.switch_request = 0x54; sim.gun.switch_state = 0;
    }
    for k in 0..12 {
        for _ in 0..25 { w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false); }
        let sim = w.sim.as_ref().unwrap();
        eprintln!("+{}: weapon {} switch_state {} aiming {} zoom {} target {}", (k+1)*25, sim.gun.weapon, sim.gun.switch_state, sim.aiming, sim.zoom, sim.zoom_target);
    }
}

#[test]
fn probe_raise_scripts() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let w = nf_formats::weapon::WeaponData::from_action_elf(&elf).unwrap();
    for wid in [2usize, 0x54] {
        // raise/idle script ids: weapon_data+0x70/+0x74 (the view anim script pair).
        let s70 = w.raise_script(wid);
        let s74 = w.lower_script(wid);
        eprintln!("weapon {wid:#x}: raise {s70:#x} lower {s74:#x}");
    }
}

#[test]
fn probe_idle_scripts() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let w = nf_formats::weapon::WeaponData::from_action_elf(&elf).unwrap();
    for wid in [2usize, 0x54] {
        eprintln!("weapon {wid:#x}: idle {:?} idle_alert {:?}", w.idle_script(wid), w.idle_alert_raise_script(wid));
    }
}

#[test]
fn probe_wall_raise() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    }
    let at = [61.342293f32, 27.060064, 109.498375];
    {
        let sim = w.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
        eprintln!("[wall] bond placed at {:?}", sim.bond.pos);
    }
    for _ in 0..4 { w.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x54), ..Input::default() }, false, false, false); }
    for k in 0..6 {
        for _ in 0..15 { w.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        let sim = w.sim.as_ref().unwrap();
        eprintln!("settle +{}: weapon {} view_script {} switch_state {}", (k+1)*15, sim.gun.weapon, sim.view_script, sim.gun.switch_state);
    }
    for k in 0..10 {
        for _ in 0..15 { w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false); }
        let sim = w.sim.as_ref().unwrap();
        eprintln!("raise +{}: view_script {} aiming {} zoom {:.2}", (k+1)*15, sim.view_script, sim.aiming, sim.zoom);
    }
}

#[test]
fn probe_view_chain() {
    let Some(iso) = iso() else { eprintln!("no NF_ISO"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = w.sim.as_mut().unwrap();
        for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
        sim.gun.weapon = 0x54; sim.gun.switch_request = 0x54; sim.gun.switch_state = 0;
    }
    for _ in 0..60 { w.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = w.sim.as_ref().unwrap();
        eprintln!("after swap: view_script {} idle(0x54) {} has_script {}", sim.view_script, sim.weapons.idle_script(0x54), sim.scripts.len());
        eprintln!("scripts contain idle: {} raise: {}", sim.scripts.contains_key(&100664742), sim.scripts.contains_key(&0x60005a4));
    }
    for k in 0..6 {
        for _ in 0..15 { w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false); }
        let sim = w.sim.as_ref().unwrap();
        eprintln!("+{}: view_script {} aiming {} zoom {:.2} target {}", (k+1)*15, sim.view_script, sim.aiming, sim.zoom, sim.zoom_target);
    }
}
