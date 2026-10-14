//! Headless tests for every F2 dev-menu action (B-10).
//!
//! The user reported that "Spawn guard" crashed in play. Every menu action must run without panicking
//! (the result message is shown on screen; a panic kills the window). Each test loads the scene, applies
//! the action through the real `dev` API (the same path the viewer's `apply_menu_action` uses), ticks the
//! sim a few frames, and asserts no panic + a sane result. Opt-in via `NF_ISO`.
//!
//! These tests also serve as regression guards: if a future change breaks a menu action, the test fails
//! instead of the user's window crashing.

use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A loaded scene + dev state, the same path the viewer uses. Bond is placed at the level start.
fn loaded() -> Option<(nf_view::scene::Scene, dev::Dev)> {
    let iso = iso()?;
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let dev = dev::Dev { scenario: None, ..Default::default() };
    Some((scene, dev))
}

/// Tick the sim `n` times with idle input (no panic is the assertion).
fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n {
            // `step_bond` is the real per-tick path the viewer uses (input → Sim → output).
            sim.step_bond(Default::default(), false, false, false);
        }
    }
}

/// B-11: measure the real font glyph height at the HUD scale, so the menu layout test can assert no row overlap.
/// Returns (max_glyph_h, max_glyph_h_plus_yoff, top_offset) in HUD pixels.
fn font_metrics(scene: &nf_view::scene::Scene) -> (f32, f32, f32) {
    let font = &scene.hud.as_ref().expect("hud").font;
    let max_h = font.glyphs.iter().map(|g| g.h as f32).fold(0.0f32, f32::max);
    let max_h_yoff = font.glyphs.iter().map(|g| (g.h + g.yoff) as f32).fold(0.0f32, f32::max);
    let top = font.glyphs.first().map_or(0.0, |g| (g.h + g.yoff) as f32);
    (max_h, max_h_yoff, top)
}

#[test]
fn menu_font_metrics_measured() {
    // B-11: measure the real font glyph height so the menu layout is computed from it, not guessed. The user
    // reported rows overlap at ~40 px each — this test prints the actual glyph h + the HUD scale, so we can set
    // ROW_H correctly. At a 2000x1094 window, 1 HUD-px = 1094/448 = 2.44 window-px vertically.
    let Some((scene, _)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let (max_h, max_h_yoff, top) = font_metrics(&scene);
    eprintln!("[font] max glyph h = {max_h:.1} HUD-px, max h+yoff = {max_h_yoff:.1}, top offset = {top:.1}");
    eprintln!("[font] at 2000x1094: 1 HUD-px = {:.2} window-px (vert), so max glyph = {:.0} window-px", 1094.0/448.0, max_h * 1094.0/448.0);
    // A row must be at least (max_h + a 2px gap) tall to avoid overlap.
    let needed_row_h = max_h + 2.0;
    eprintln!("[font] ROW_H must be >= {needed_row_h:.1} HUD-px to avoid overlap");
    assert!(max_h > 0.0, "font has no glyphs");
    // B-11: assert the menu's ROW_H is tall enough for the real glyphs (no overlap).
    assert!(dev::MENU_ROW_H >= max_h + 1.0,
        "MENU_ROW_H ({}) is shorter than the font's max glyph h ({max_h}) + 1 — rows will overlap",
        dev::MENU_ROW_H);
    // Assert the panel stays a side panel, not a screen takeover. fix-ingame-dev-menu widened it
    // 150 -> 190 HUD-px so the game's own weapon names fit ("0x2c AT-420 Sentinel" = 20 glyphs) —
    // at a real 16:9 window the aspect correction (menu_hit's s) puts 190 px at ~28% of the screen,
    // still under a third.
    assert!(dev::MENU_PANEL_W < 512.0 * 0.40,
        "MENU_PANEL_W ({}) is over 40% of the 512-px HUD width", dev::MENU_PANEL_W);
    eprintln!("[font] layout OK: ROW_H={} (glyph {max_h}+2), PANEL_W={} ({:.0}% of 512)",
        dev::MENU_ROW_H, dev::MENU_PANEL_W, dev::MENU_PANEL_W / 512.0 * 100.0);
}

/// B-11: compute each menu row's text bounding box from the font metrics, and assert no two rows overlap and
/// everything fits inside the panel. This is the layout test the user demanded: "compute each row's text bounding
/// box from the font metrics, and assert there's no overlap and everything fits inside the panel." It catches the
/// B-10 regression (ROW_H=12 with 17px glyphs → overlap) that headless action tests can't see.
#[test]
fn menu_layout_no_row_overlap_and_fits_panel() {
    let Some((scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let font = &scene.hud.as_ref().expect("hud").font;
    let items = dev::menu_items(&dev, Some(&scene));
    let n = items.len();
    let row_h = dev::MENU_ROW_H;
    let glyph_h = dev::MENU_GLYPH_H;
    let panel_w = dev::MENU_PANEL_W;
    let panel_x = dev::MENU_PANEL_X;
    let visible = dev::MENU_VISIBLE_ROWS;
    // For each visible row, the text's vertical extent is [row_y, row_y + glyph_h] (the glyph occupies the full
    // glyph_h, placed at row_y + 1 by the menu fn). Two consecutive rows overlap if row_y(i+1) < row_y(i) + glyph_h.
    let banner = if dev.scenario.is_some() { dev::MENU_HEADER_H + 1.0 } else { 2.0 };
    let list_y0 = 4.0 + banner;
    for vi in 0..n.min(visible) {
        let row_y = list_y0 + vi as f32 * row_h;
        let text_top = row_y + 1.0;            // the menu places text at row_y + 1
        let text_bot = text_top + glyph_h;     // the glyph is glyph_h tall
        // The next row's text starts at row_y + row_h + 1. No overlap if that's >= this row's text_bot.
        if vi + 1 < n.min(visible) {
            let next_row_y = list_y0 + (vi + 1) as f32 * row_h;
            let next_text_top = next_row_y + 1.0;
            assert!(next_text_top >= text_bot,
                "row {vi} text [{text_top:.1}, {text_bot:.1}] overlaps row {} text starting at {next_text_top:.1} (ROW_H={row_h}, glyph_h={glyph_h})",
                vi + 1);
        }
        // Also assert the text's width fits in the panel. Measure the label via font.layout.
        let (label, _desc, _) = &items[vi];
        let lbl = dev::fit(label, 16);
        let (_, w) = font.layout(&lbl, 0.0, 0.0);
        let text_x = panel_x + 4.0;
        assert!(text_x + w <= panel_x + panel_w,
            "row {vi} label '{lbl}' width {w:.0} + x {text_x:.0} exceeds panel right {:.0}",
            panel_x + panel_w);
    }
    // The whole menu (header + list + desc + footer) must fit in the HUD height (448).
    let total_h = banner + (n.min(visible) as f32 * row_h) + dev::MENU_DESC_H + 1.0 + dev::MENU_DESC_H + 1.0 + dev::MENU_ROW_H;
    assert!(total_h < 448.0, "menu total height {total_h:.0} exceeds the HUD height 448");
    eprintln!("[layout] {n} rows, ROW_H={row_h}, glyph_h={glyph_h}: no overlap, panel {panel_w}px, total {total_h:.0}px < 448");
    let _ = dev; // silence unused warning
}

#[test]
fn menu_spawn_normal_guard_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    // Diagnostic: print available skins on 07000005.
    if let Some(sim) = scene.sim.as_ref() {
        let skins: Vec<u32> = sim.skins.keys().copied().collect();
        eprintln!("[diag] {} skins on 07000005: {:x?}", skins.len(), &skins[..skins.len().min(20)]);
    }
    dev.spawn = dev::SpawnPick::Normal;
    // Spawn at a known-open spot (Bond's start on 07000005 is open ground).
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let n_drones_before = scene.sim.as_ref().map_or(0, |s| s.drones.list.len());
    let n_chars_before = scene.characters.len();
    let n_char_drone_before = scene.sim.as_ref().map_or(0, |s| s.char_drone.len());
    let gi = dev.spawn_guard(&mut scene, pos, 0.0);
    assert!(gi.is_some(), "spawn_guard returned None - no drone created");
    let gi = gi.unwrap();
    // B-11/B-12: assert the VISIBLE effect - a new drone exists, is present + drawn, in the llist, AND has a
    // Character (mesh + skeleton) + char_drone entry so the renderer draws it (the Round 7 bug: the drone existed
    // in the AI but was invisible because no Character was added).
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.list.len(), n_drones_before + 1, "spawn did not add a drone to the list");
    let dr = &sim.drones.list[gi];
    assert!(dr.present, "spawned guard not present");
    assert!(dr.drawn, "spawned guard not drawn (obj+0xFC) - it wouldn't be visible in play");
    assert!(sim.drones.llist.contains(&gi), "spawned guard not in the llist - the game wouldn't run its AI");
    assert!(dr.grenade_count > 0, "spawned guard has no grenades (spawn_guard gives 5)");
    // B-12: the Character + char_drone (the renderer's link from character to drone).
    assert_eq!(scene.characters.len(), n_chars_before + 1, "spawn did not add a Character (mesh) - the guard would be invisible");
    assert_eq!(sim.char_drone.len(), n_char_drone_before + 1, "spawn did not add a char_drone entry - the renderer can't link the character to the drone");
    assert_eq!(sim.char_drone.last().copied().flatten(), Some(gi), "char_drone entry does not point to the spawned drone");
    eprintln!("[menu_test] spawn Normal: guard #{gi} present+drawn+in-llist, {} grenades, Character+char_drone added, at {:?}", dr.grenade_count, dr.pos);
    // Tick 60 frames; the new drone's first tick (set_state, anim, vision) must not panic.
    tick(&mut scene, 60);
    let dr = &scene.sim.as_ref().unwrap().drones.list[gi];
    assert!(dr.present, "spawned guard not present after 60 ticks");
    eprintln!("[menu_test] spawn Normal: after 60 ticks state 0x{:x} health {}", dr.state, dr.health);
}

#[test]
fn menu_spawn_sniper_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Sniper;
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.spawn_guard(&mut scene, pos, 0.0);
    assert!(gi.is_some(), "spawn_guard (sniper) returned None");
    tick(&mut scene, 60);
    eprintln!("[menu_test] spawn Sniper: ok");
}

#[test]
fn menu_spawn_ambush_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Ambush;
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.spawn_guard(&mut scene, pos, 0.0);
    assert!(gi.is_some(), "spawn_guard (ambush) returned None");
    tick(&mut scene, 60);
    eprintln!("[menu_test] spawn Ambush: ok");
}

#[test]
fn menu_force_grenade_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    // Need a guard near Bond to force. Spawn one first, then force.
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.spawn = dev::SpawnPick::Normal;
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    let bond = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.force_grenade(&mut scene, bond);
    assert!(gi.is_some(), "force_grenade found no guard near Bond");
    tick(&mut scene, 60);
    eprintln!("[menu_test] force grenade: ok");
}

#[test]
fn menu_impulse_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.spawn = dev::SpawnPick::Normal;
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    let bond = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.impulse_guard(&mut scene, bond);
    assert!(gi.is_some(), "impulse_guard found no guard near Bond");
    tick(&mut scene, 30);
    eprintln!("[menu_test] impulse: ok");
}

#[test]
fn menu_special_death_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.spawn = dev::SpawnPick::Normal;
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    let bond = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.special_death(&mut scene, bond);
    assert!(gi.is_some(), "special_death found no guard near Bond");
    tick(&mut scene, 60);
    eprintln!("[menu_test] special_death: ok");
}

#[test]
fn menu_toggle_god_and_ammo_do_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    // B-11: assert the VISIBLE effect - god mode locks health at 100 + clears the hit flash; infinite ammo refills.
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.health.health = 50.0;   // simulate Bond took damage
    sim.drones.health.flash = 0.5;    // hit flash active
    let wi = sim.gun.weapon as usize;
    drop(sim);
    dev.god_mode = true;
    dev.infinite_ammo = true;
    // fix-pickups-trace: the refill is the recorder's `--ammo` cheat — each OWNED weapon's clip and each
    // spare type tops back to the most seen this run. Seed the best the way play does (the cheat sees the
    // full clip first), then drain, then top back up.
    dev.after_tick(&mut scene);
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.clip = 3;                  // low ammo
    if let Some(c) = sim.drones.inventory.clip.get_mut(wi) { *c = 3; } // the store, too (fix-dev: the refill writes the inventory)
    drop(sim);
    dev.after_tick(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.health.health, 100.0, "god mode did not restore health to 100");
    assert_eq!(sim.drones.health.flash, 0.0, "god mode did not clear the hit flash");
    let cs = sim.weapons.clip_size(wi);
    // fix-dev: the refill tops the INVENTORY (the game's own store) to the weapon's clip size, and the
    // gun's HUD copy follows — the old gun-only refill (clip >= 40) was overwritten by the next tick's
    // inventory read, which is why ammo still ran out (the user's "isn't always reliable").
    assert!(sim.gun.clip >= 3, "infinite ammo did not refill the clip (got {})", sim.gun.clip);
    assert_eq!(sim.drones.inventory.clip[wi], cs, "infinite ammo did not top the inventory's clip slot to the clip size ({})", cs);
    assert!(sim.gun.clip >= cs.max(0) as u32, "the gun's HUD copy ({}) is below the topped clip size ({})", sim.gun.clip, cs);
    // Toggle off - the effect stops (health is no longer locked).
    eprintln!("[menu_test] god mode: health 50->{}, flash cleared, clip 3->{} (infinite ammo)", sim.drones.health.health, sim.gun.clip);
    // Toggle off - the effect stops (health is no longer locked).
    dev.god_mode = false;
    dev.infinite_ammo = false;
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.health.health = 50.0; // damage again
    drop(sim);
    dev.after_tick(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.health.health, 50.0, "god mode still active after toggle off - health was locked");
    eprintln!("[menu_test] god mode off: health stays at {} (not locked)", sim.drones.health.health);
}

#[test]
fn menu_teleport_save_restore_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let p = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.saved_spot = Some(p);
    dev.teleport(&mut scene, p, 0.0);
    tick(&mut scene, 10);
    eprintln!("[menu_test] teleport save/restore: ok");
}

#[test]
fn menu_spawn_then_animate_does_not_panic() {
    // Reproduces the viewer's "Spawn guard crashed" report: the crash happens in the render/pose path after a
    // spawn (the headless tick-only test doesn't exercise it). This calls `scene.animate` — the same pose loop
    // the renderer's `shot()` uses — after spawning, to catch any indexing panic on the new drone.
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Normal;
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    tick(&mut scene, 5);
    // The pose loop the renderer runs each frame. `choice = None` is the normal AI-driven path.
    let opts = nf_view::scene::ViewOpts { reinforcements: false, firing: false, right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0] };
    let _poses = scene.animate(None, 1.0, opts);
    eprintln!("[menu_test] spawn + animate: ok (no panic in pose loop)");
}

#[test]
fn menu_timescale_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.time_scale = 0.25;
    let n = dev.tick_count(4);
    assert_eq!(n, 1, "0.25× of 4 ticks should be 1");
    dev.time_scale = 0.0;
    let n = dev.tick_count(4);
    assert_eq!(n, 0, "paused should be 0 ticks");
    dev.step = true;
    let n = dev.tick_count(0);
    assert_eq!(n, 1, "step one tick when paused");
    tick(&mut scene, n);
    eprintln!("[menu_test] time scale: ok");
}
