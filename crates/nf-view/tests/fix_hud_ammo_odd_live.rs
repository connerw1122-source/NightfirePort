//! fix-hud-ammo-odd: the ammo pane's numbers for the Samurai, the detonator and the thrown weapons
//! (the main agent's dry run of the playtest checks on master, `nf-view <iso> 07000005 --dev --give …`):
//!
//! - the detonator (0x38, base 0x37, +0x05 = 0xFF): the game's pane reads the clip through
//!   `Player_AmmoIndex` 0x1a9148 — the twin-resolved slot — so holding the detonator shows the MINE's
//!   shared counts ("1-4" = clip[0x37], carried[type 0x16]). The port's dev infinite-ammo pin maxed
//!   the live clip with the HELD record's own +0x92 (500) and kept in step with the twin's junk own
//!   slot (the give's twin fill parks clip[0x38] = 500) → "500-4".
//! - the Samurai (0x32, kind 0x1f, cap 100): the pane draws the DEPLETED percent
//!   `(min(100−clip, 88)·100)/88` [M 0x19a7f0:129-136, disasm 0x19a914-0x19a934]. The charge drain
//!   (`Player_WeaponFiring` 0x1a5460:197-201, the scene.rs port) is not a `Player_RoundToFire` spend —
//!   the game's ammo flag (DAT_002a3740) never blocks it — so the pane climbs while the overcharge
//!   holds ("the HUD number climbs 0 → 100", the fix-fire-modes3 PCSX2 pass). The dev pin re-maxed the
//!   clip to 100 every tick and the pane froze at the full-charge "0".
//! - the thrown weapons (mine 0x37, frag 0x35, satchel 0x36): with the dev infinite ammo OFF the
//!   counts drop per throw; the ON top-up is the trace-cheat shape and expected (the task's own note).
//!
//! Every test drives the real input path (`nf_view::load` + `Sim::step_bond` with the window-built
//! `Input`, the give mirroring `--give`'s `Player_EquipWeapon`/`EquipAmmo` calls, the dev window's
//! `Dev::after_tick` in the App's per-tick order) and reads the drawn count text back out of
//! `nf_view::hud::overlay`'s font quads. Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The pane's count text, decoded from the overlay's font-2 glyph quads in the ammo-text box
/// (right-aligned at (496+sp.x, 394+sp.y), sprite 0 of `AmmoSprInfo` — the same box
/// hud_playtest_live reads, 448-line space). Each glyph draws twice (shadow, then text) with the
/// same atlas cell [M hud.rs `text_scaled_tex`]; the text copy sits at the smaller x. The quads
/// carry NORMALIZED atlas UVs — scaled back to the font's texel grid for the glyph match.
fn pane_text(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame) -> String {
    let Some(t) = scene.hud.as_ref() else { return String::new() };
    let font = &t.font;
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == 0x0300_0003) else { return String::new() };
    let (tw, th) = (scene.hud_textures[ti].1.width as f32, scene.hud_textures[ti].1.height as f32);
    let mut cells: std::collections::HashMap<(i32, i32, i32, i32), (f32, char)> = Default::default();
    for r in f.runs.iter().filter(|r| r.0 == ti + 1) {
        for q in f.v[r.2 as usize..(r.2 + r.3) as usize].chunks(6) {
            let (a, b) = (q[0], q[2]);
            let (x0, x1) = ((a.p[0] + 1.0) * 256.0, (b.p[0] + 1.0) * 256.0);
            let (y0, y1) = ((1.0 - a.p[1]) * 224.0, (1.0 - b.p[1]) * 224.0);
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            if !(380.0..512.0).contains(&cx) || !(396.0..418.0).contains(&cy) { continue; }
            let uv = (a.uv[0] * tw, a.uv[1] * th, (b.uv[0] - a.uv[0]) * tw, (b.uv[1] - a.uv[1]) * th);
            // Nearest atlas cell (the thin glyphs quantize; L1 on the texel box).
            let Some(g) = font.glyphs.iter().enumerate().min_by_key(|(_, g)| {
                ((g.u as f32 - uv.0).abs() + (g.v as f32 - uv.1).abs()
                 + (g.w as f32 - uv.2).abs() + (g.h as f32 - uv.3).abs()) as i32
            }).map(|(_, g)| g) else { continue };
            let ch = char::from_u32(u32::from(g.code)).unwrap_or('?');
            let key = (g.u as i32, g.v as i32, g.w as i32, g.h as i32);
            let e = cells.entry(key).or_insert((x0, ch));
            if x0 < e.0 { *e = (x0, ch); }
        }
    }
    let mut s: Vec<(f32, char)> = cells.into_values().collect();
    s.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    s.into_iter().map(|(_, c)| c).collect()
}

/// The game's count text for a state — what 0x19a7f0 sprintfs at rest (0x19aad4 percent /
/// 0x19ab18 "%d-%d") [M], the same table ammo_pane_live asserts.
fn expected_text(kind: u8, clip: u32, spare: u32, cap: u32) -> String {
    match kind {
        0x14 | 0x1a => format!("{}", clip * 100 / cap.max(1)),
        0x1f => { let mx = cap.saturating_sub(12); format!("{}", mx.min(cap.saturating_sub(clip)) * 100 / mx.max(1)) }
        _ => format!("{}-{}", clip, spare),
    }
}

/// `--give`'s equip (lib.rs: `Player_EquipWeapon` 0x1a8a58 + three clips, then the twin fill of
/// fix-aims20-modes-lights), without the draw request.
fn give_one(sim: &mut nf_view::scene::Sim, wd: &nf_formats::weapon::WeaponData, id: u8) {
    let rounds = wd.clip_size(id as usize).max(1);
    assert!(sim.drones.inventory.equip_weapon(wd, id as usize, rounds as i16, true), "equip {id:#04x} refused");
    sim.drones.inventory.equip_ammo(wd, id as usize, (rounds * 3).max(rounds) as i16);
    let twin = wd.records[id as usize][5] as i8;
    if twin != 0 {
        let t = (id as i32 + twin as i32) as usize;
        if let Some(c) = sim.drones.inventory.clip.get_mut(t) {
            if *c == 0 { *c = wd.clip_size(t).max(1); }
        }
    }
}

/// Ticks the real input path until `pred` holds (the dev window's after_tick runs every tick in the
/// App's order [M lib.rs:2347]).
fn run_until(scene: &mut nf_view::scene::Scene, mut dev: Option<&mut nf_view::dev::Dev>, max: usize,
             mut pred: impl FnMut(&nf_view::scene::Sim) -> bool) -> bool {
    for _ in 0..max {
        { let sim = scene.sim.as_mut().unwrap(); sim.step_bond(Input::default(), false, false, false); }
        if let Some(d) = dev.as_deref_mut() { d.after_tick(scene); }
        if pred(scene.sim.as_ref().unwrap()) { return true; }
    }
    false
}

/// Gives `ids`, draws `draw` through the give's own draw request (`--give` sets
/// `gun.switch_request` — drone+0x63, the game's pending-request mechanism [M lib.rs:2782]) and
/// settles the draw and the switch timer.
fn give_and_draw(scene: &mut nf_view::scene::Scene, mut dev: Option<&mut nf_view::dev::Dev>, ids: &[u8], draw: u8) {
    { let sim = scene.sim.as_mut().unwrap(); let wd = sim.weapons.clone();
      for &id in ids { give_one(sim, &wd, id); }
      sim.gun.switch_request = draw; }
    let mut drew = false;
    for i in 0..160 {
        { let sim = scene.sim.as_mut().unwrap(); sim.step_bond(Input::default(), false, false, false); }
        if let Some(d) = dev.as_deref_mut() { d.after_tick(scene); }
        let s = scene.sim.as_ref().unwrap();
        if i % 40 == 0 { eprintln!("ud] draw tick {i}: weapon {:#04x} req {:#04x} state {}", s.gun.weapon, s.gun.switch_request, s.gun.switch_state); }
        if s.gun.weapon == draw { drew = true; break; }
    }
    assert!(drew, "{draw:#04x} did not draw");
    assert!(run_until(scene, dev, 220, |s| s.gun.switch_state == 0 && s.gun.reload_state == 0
                                          && s.hud.weapon_label_timer == 0), "{draw:#04x} never settled");
}

/// The dry run's detonator check (`--give 0x37,0x38`, cycle to the detonator, read the pane) with
/// the dev window's infinite ammo ON — its `--dev` default [M lib.rs App::new]. The game's pane
/// reads the twin-resolved slot [M Player_AmmoIndex 0x1a9148]: the mine's clip[0x37] = 1 and the
/// type-0x16 carried = 4 (the give: 3 rounds + the detonator equip's 500 clamped at the type cap
/// 4 [L ee_snapshot ammo_data 0x2c69b8 + 0x16·0xC + 2 = 4]) → "1-4".
#[test]
fn the_detonators_pane_shows_the_mines_shared_counts_under_the_dev_pin() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut dev = nf_view::dev::Dev::default();
    dev.infinite_ammo = true;
    give_and_draw(&mut scene, Some(&mut dev), &[0x37, 0x38], 0x38);
    {
        let sim = scene.sim.as_ref().unwrap();
        // The shared slot holds the mine's round; the detonator's own slot is the give's twin-fill
        // junk (500) that the game's pane never reads [M 0x1a9148].
        assert_eq!(sim.drones.inventory.clip.get(0x37).copied(), Some(1), "the mine's clip");
        assert_eq!(sim.gun.clip, 1, "the live clip is the resolved slot's (pre-fix the pin maxed it to 500)");
    }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let txt = pane_text(&scene, &f);
    assert_eq!(txt, "1-4", "the detonator's pane reads the mine's shared counts (pre-fix: 500-4)");
    // Sister: the detonate press spends nothing (+0x91 = 0 [L ee_start_05.bin record 0x38]) — the
    // pane holds through a B press.
    { let sim = scene.sim.as_mut().unwrap(); sim.step_bond(Input::default(), true, false, false); }
    dev.after_tick(&mut scene);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert_eq!(pane_text(&scene, &f), "1-4", "the detonate press spends 0 rounds — the pane holds");
}

/// The dry run's Samurai check (`--give 0x32`, hold fire for "Overcharge") with the dev pin running:
/// the charge drain pulls clip[0x33] down (0x1a5460:197-201) and the kind-0x1f pane climbs with it.
#[test]
fn the_samurai_charge_climbs_the_pane_while_the_dev_pin_runs() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut dev = nf_view::dev::Dev::default();
    dev.infinite_ammo = true;
    give_and_draw(&mut scene, Some(&mut dev), &[0x32], 0x32);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.inventory.clip.get(0x33).copied(), Some(100), "the Samurai's full charge [L +0x92 = 100]");
        assert_eq!(pane_text(&scene, &nf_view::hud::overlay(&scene, 4.0 / 3.0)), "0", "at rest the depleted percent is 0 (the game's full-charge display)");
    }
    // Hold the trigger: the charge script 0x060007c2 [L ee_start_05 +0xb0] plays, the drain runs,
    // the pane climbs. The pin must not erase the drain (the game's flag never blocks it).
    let mut climbed = None;
    for i in 0..150 {
        { let sim = scene.sim.as_mut().unwrap(); sim.step_bond(Input::default(), true, false, false); }
        let clip = scene.sim.as_ref().unwrap().drones.inventory.clip.get(0x33).copied().unwrap_or(0);
        dev.after_tick(&mut scene);
        let txt = pane_text(&scene, &nf_view::hud::overlay(&scene, 4.0 / 3.0));
        assert!(!txt.contains('-'), "tick {i}: kind 0x1f draws one percent number, never clip-spare [M 0x19aad4]");
        if txt != "0" && climbed.is_none() { climbed = Some((i, txt.clone(), clip)); }
    }
    let (at, txt, clip) = climbed.expect("the charge climbs the pane above the full-charge 0 (pre-fix the dev pin pinned the clip at cap: a frozen 0)");
    eprintln!("ud] the pane left 0 at tick {at}: text {txt:?} at clip[0x33] {clip}");
    assert!(clip < 100, "the drain pulled the shared slot below the cap");
    assert_eq!(txt, expected_text(0x1f, clip as u32, 0, 100), "the drawn text is the game's formula over the live shared slot [M 0x19a7f0:129-136]");
}

/// The thrown weapons with the dev infinite ammo OFF (the task's own check): the counts drop per
/// throw through the real trigger — the pane falls to "0-N" on the empty clip and back up one spare
/// lower after the auto-reload [M the 0x1a5460 reload path].
#[test]
fn the_thrown_weapons_counts_drop_with_infinite_ammo_off() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // (weapon, the give list, the rest text, the text after one throw + auto-reload). The mine's
    // dry run gave the pair — the detonator's equip tops the type-0x16 carried at its cap 4
    // [L ee_snapshot ammo_data + 0x16·0xC + 2 = 4] — hence the rest "1-4".
    for (id, give, rest, after) in [(0x37u8, &[0x37u8, 0x38u8][..], "1-4", "1-3"),
                                    (0x35, &[0x35], "1-3", "1-2"), (0x36, &[0x36], "1-3", "1-2")] {
        let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
        give_and_draw(&mut scene, None, give, id);
        let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
        assert_eq!(pane_text(&scene, &f), rest, "{id:#04x}: the give's counts at rest");
        // One real trigger tick spends the clip's round (the shot fires on the press [M 0x1a5460]).
        { let sim = scene.sim.as_mut().unwrap(); sim.step_bond(Input::default(), true, false, false); }
        let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
        let spent = pane_text(&scene, &f);
        assert!(spent.starts_with('0'), "{id:#04x}: the clip emptied on the throw (read {spent:?})");
        // The auto-reload refills from the spare — one lower.
        assert!(run_until(&mut scene, None, 500, |s| s.gun.reload_state == 0 && s.gun.clip == 1), "{id:#04x}: the auto-reload never refilled");
        let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
        assert_eq!(pane_text(&scene, &f), after, "{id:#04x}: the count dropped one throw's worth");
    }
}
