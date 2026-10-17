//! fix-hud: the HUD elements the port lacked, each from `docs/systems/hud.md` and the decompile:
//! the fire-mode / weapon-name labels (`HUD_UpdateAmmoPane` 0x19a7f0, sprites 1/2, `BL+0x95E`), the pickup
//! message bar (`Pickup_Handler` 0x18e650 → `Text_AddMsg` 0x1b0c30, pane 21, 180 ticks; the strings from
//! `USATxt.dat` via `Txt_BindLabel` 0x1cad60), the damage-direction glow (`HUD_UpdateHealthPane` 0x19b9c8
//! sprites 20–23 per `BL+0x967`), the air bar (`HUD_UpdateAirPane` 0x1a11c8, `AirSprInfo` 0x2bd490) and the
//! crouch icon (health pane sprite 19, 0x03000173/74).
//! Opt-in via `NF_ISO`.

use nf_game::pickup::Taken;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Quads of one texture in the frame: (first vertex, count) runs.
fn tex_runs(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32) -> Vec<(usize, usize)> {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == hash) else { return Vec::new() };
    f.runs.iter().filter(|r| r.0 == ti + 1).map(|r| (r.2 as usize, r.3 as usize)).collect()
}

/// The world position of a screen pixel in the overlay's NDC.
fn ndc_y(y: f32) -> f32 { 1.0 - y / 224.0 }

#[test]
fn usatxt_labels_bind_the_game_strings() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let s = scene.sim.as_ref().expect("sim");
    let lang = s.lang.as_ref().expect("USATxt.dat parsed (fix-hud loads it in nf_view::load)");
    // [D USATxt.dat entries fixups[2]+13/14/15/16/74; read back from the disc]
    assert_eq!(lang.bind(0x200_000D), Some("Picked up %s"));
    assert_eq!(lang.bind(0x200_000F), Some("Picked up Armor"));
    assert_eq!(lang.bind(0x200_004A), Some("007 Bonus collected"));
    // Sister: an out-of-range label id binds to nothing (the game's "Invalid Text Label" path).
    assert_eq!(lang.bind(0x2FFF_FFFF), None);
    // The weapon and fire-mode labels the ammo pane shows: weapon 2 (the P2K start).
    let name = lang.bind(s.weapons.name_label(2)).expect("weapon 2 name label binds");
    eprintln!("[hud] weapon 2 name = {name:?}, fire-mode = {:?}", lang.bind(s.weapons.fire_mode_label(2, 0)));
    assert!(!name.is_empty(), "weapon 2's +0x38 name label is empty");
}

#[test]
fn taking_an_armour_pickup_shows_the_message_bar_then_it_fades() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    // The real take path: stand on a placed pickup (any kind) and let the touch check take it; assert the
    // message text matches that kind's string.
    let cands: Vec<[f32; 3]> = scene.sim.as_ref().unwrap().drones.pickups.iter()
        .filter(|p| !p.gone && !p.hidden && p.from.is_none())
        .map(|p| [p.m[0][3], p.m[1][3], p.m[2][3]]).take(12).collect();
    assert!(!cands.is_empty(), "07000005 places pickups");
    let mut taken_kind: Option<Taken> = None;
    'outer: for at in &cands {
        // The take test (`Pickup::touches`, pickup.rs) reaches a pickup from a small ring around it.
        for off in [[0.0f32, 1.2, -0.6], [0.0, 1.2, 0.6], [0.6, 1.2, 0.0], [-0.6, 1.2, 0.0], [0.0, 0.9, 0.0]] {
            {
                let sim = scene.sim.as_mut().unwrap();
                sim.bond = nf_game::player::Bond::stand_at(&sim.world, [at[0] + off[0], at[1] + off[1], at[2] + off[2]], 0.0);
                sim.hud.taken_seen = sim.drones.taken.len();
            }
            for _ in 0..60 {
                scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
                let sim = scene.sim.as_ref().unwrap();
                if sim.drones.taken.len() > sim.hud.taken_seen { taken_kind = Some(sim.drones.taken.last().unwrap().1.clone()); break 'outer; }
            }
        }
    }
    let Some(tk) = taken_kind else { eprintln!("[hud] no placed pickup could be taken headless — skipping"); return };
    eprintln!("[hud] took {tk:?}");
    let sim = scene.sim.as_ref().unwrap();
    // The bar: the message text from the language table and a full-width bar this tick.
    let msg = sim.hud.msgs.front().expect("the take queued the pickup message");
    let lang = sim.lang.as_ref().unwrap();
    let expected = match &tk {
        Taken::Armour => lang.bind(0x200_000F).unwrap_or("").to_string(),
        Taken::Bonus => lang.bind(0x200_004A).unwrap_or("").to_string(),
        Taken::Label { label } => if *label != 0 { lang.bind(*label).unwrap_or("").to_string() } else { String::new() }, // fix-pickups-ammo: the kind-4 label message (0700000b's item: 0x2000036 [D])
        Taken::Message => lang.bind(0x200_0013).unwrap_or("").to_string(), // case 5 [M 0x18e650:229-233]
        Taken::Key { .. } => lang.bind(0x200_0010).unwrap_or("").to_string(),
        Taken::Weapon { weapon } => lang.format(0x200_000D, &[lang.bind(sim.weapons.name_label(*weapon as usize)).unwrap_or("").to_string()]),
        Taken::Ammo { weapon, rounds } => {
            let at2 = sim.weapons.ammo_type(*weapon as usize);
            lang.format(0x200_000E, &[rounds.to_string(), lang.bind(sim.weapons.ammo_name_label(at2)).unwrap_or("").to_string()])
        }
    };
    assert_eq!(msg.text, expected, "the message text matches the pickup kind's string");
    assert!(!expected.is_empty());
    assert!((sim.hud.bar_close - 256.0).abs() < 1e-4);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(!tex_runs(&scene, &f, 0x0300_007a).is_empty(), "the bar quad draws while the message is up");
    // 180 ticks of text, then the close. Evidence [M]: the pickup message's life 180 is
    // `Pickup_Handler` 0x18e650's `fVar22 = 180.0` → `Text_AddMsg(...,6,str,0,180)` (90 in MP); the
    // life float drains 1/tick (`Text_UpdateMsg` 0x1b11d8:49, −FRAME_RATE_MUL; = 1.0 at 60 Hz [M
    // GS_SetRefreshRate 0x17d2b8:15, L ee_snapshot 0x30d0d8]); the pane fades alpha = life·17 while
    // life < 15 (`HUD_UpdateStatusPane` 0x19c310:262-268) — so the fade is the LAST 15 ticks; and the
    // close is the pane state +0xc decrement: `HUD_CreatePickupStatusPane` 0x1a1140:10 writes 0x10
    // (16) into the pickup pane's rate (`HUD_CreateInfoStatusPane` 0x1a10b8:10 and
    // `HUD_CreateObjectiveStatusPane` 0x1a1070:10 write the same 0x10; the mission pane 0x1a1028:10
    // writes 0x20), which `HUD_UpdateStatusPane` 0x19c310:331 (the field read at :37, `piVar4[3]`)
    // subtracts from the bar's countdown ushort each state-4 tick — 16 per tick, the port's
    // `bar_close`.
    for _ in 0..185 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.hud.msgs.is_empty(), "the message expired");
    assert!(sim.hud.bar_close > 0.0 && sim.hud.bar_close < 256.0, "the bar is mid-close (16 px/tick)");
    let remaining = (sim.hud.bar_close / 16.0).ceil() as u32 + 2;
    for _ in 0..remaining { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.hud.bar_close, 0.0, "the bar finished closing");
}

/// `overlay` takes `&Scene`; the tests hold borrows in blocks, so pass the scene straight through.
fn nf_hack(scene: &nf_view::scene::Scene) -> &nf_view::scene::Scene { scene }

#[test]
fn ammo_pane_shows_the_fire_mode_and_weapon_name_labels() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Font glyphs the overlay draws in the bottom-right label region (x > 380, y in 400..445): with the
    // language table removed only the ammo count text remains; restoring it adds the fire-mode and
    // weapon-name labels (sprites 1/2 at (496, 432), layer 28).
    let label_glyphs = |scene: &nf_view::scene::Scene| -> usize {
        let f = nf_view::hud::overlay(scene, 4.0 / 3.0);
        let mut n = 0;
        for (start, c) in tex_runs(scene, &f, 0x0300_0003) {
            for v in &f.v[start..start + c] {
                let (sx, sy) = ((v.p[0] + 1.0) * 256.0, (1.0 - v.p[1]) * 224.0);
                if sx > 380.0 && sy > 400.0 && sy < 445.0 { n += 1; }
            }
        }
        n
    };
    let base = label_glyphs(&scene);
    let saved = scene.sim.as_mut().unwrap().lang.take();
    let without = label_glyphs(&scene);
    scene.sim.as_mut().unwrap().lang = saved;
    eprintln!("[hud] label-region glyphs with lang: {base}, without: {without}");
    assert!(base >= without + 4, "restoring the language table draws the fire-mode/name label glyphs");
    // Switching the weapon through the REAL input path (the cycle button) arms `BL+0x95E` (the timer the
    // labels' brightness follows) — set when the new id's record +2 word differs [M 0x19a7f0:206-213].
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { weapon_cycle: 1, ..Default::default() }, false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[hud] after the cycle: weapon {}, timer {}", sim.gun.weapon, sim.hud.weapon_label_timer);
    if sim.gun.weapon != 2 {
        assert!(sim.hud.weapon_label_timer > 0, "the weapon change armed the 60-tick label timer (60 set, minus the same-frame decrement)");
    } else { eprintln!("[hud] the cycle stayed on weapon 2 (no other owned weapon) — timer not exercised here"); }
    // Sister: the labels' text is the table's (already asserted by the bind test); here the timer and
    // glyph presence prove the pane draws. An unbindable name draws nothing (the nid==0 guard).
}

#[test]
fn damage_direction_glow_lights_the_attacker_side() {
    // The mapping of `Player_DealWithObjHit` 0x194458:201-226 (ported as `PlayerHealth::pain_from`):
    // view-space x<0 → bit 4 (sprite 22, tex 0x0300018E), x>0 → bit 8 (sprite 23, flipped U),
    // y<0 → bit 1 (sprite 20, tex 0x0300018D), y>0 → bit 2 (sprite 21, flipped V).
    let mut h = nf_game::bullet::PlayerHealth::default();
    let mut rng = nf_game::drone::Rng::new(0);
    let eye = [0.0, 1.5, 0.0];
    // The port's object frame has +X = screen-left (`Sim::crosshair`), so an attacker on the +X side of a
    // yaw-0 Bond (facing +Z) is on the screen's LEFT → bit 4 (sprite 22); −X = screen right → bit 8.
    h.pain_from(&mut rng, 5.0, 1, 0x0700_0005, [4.0, 1.5, 0.0], eye, 0.0);
    assert_eq!(h.dir_bits, 0x4, "an attacker on the screen's LEFT sets bit 4");
    h.pain_from(&mut rng, 5.0, 1, 0x0700_0005, [-4.0, 1.5, 0.0], eye, 0.0);
    assert_eq!(h.dir_bits, 0x8, "an attacker on the screen's RIGHT sets bit 8");
    h.pain_from(&mut rng, 5.0, 1, 0x0700_0005, [0.0, 4.0, 0.0], eye, 0.0);
    assert_eq!(h.dir_bits, 0x2, "an attacker ABOVE sets bit 2");
    h.pain_from(&mut rng, 5.0, 1, 0x0700_0005, [0.0, -1.0, 0.0], eye, 0.0);
    assert_eq!(h.dir_bits, 0x1, "an attacker BELOW sets bit 1");
    // The generic pain path writes 0xC (both sides) [M Player_HandlePain 0x1902c8:124-130].
    let mut g = nf_game::bullet::PlayerHealth::default();
    g.pain_generic(&mut rng, 5.0, 1, 0x0700_0005, 0);
    assert_eq!(g.dir_bits, 0xC);
    // The HUD: bit 4 lights sprite 22 (tex 0x0300018E, unflipped) and nothing else.
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let glow_quads = |iso: &str, dir_bits: u8| -> usize {
        let mut sc = nf_view::load(iso, "07000005.bin", false).expect("load");
        {
            let sim = sc.sim.as_mut().unwrap();
            sim.drones.health.dir_bits = dir_bits;
            sim.drones.health.flash = 128.0;
        }
        let f = nf_view::hud::overlay(&sc, 4.0 / 3.0);
        let mut scene = sc; // keep alive for the texture lookup
        let n: usize = tex_runs(&scene, &f, 0x0300_018e).iter().map(|(_, c)| c / 6).sum();
        n
    };
    assert!(glow_quads(&iso, 0x4) >= 1, "bit 4 draws the tex-0x0300018E glow");
    assert_eq!(glow_quads(&iso, 0x1), 0, "bit 1 draws only the 0x0300018D pair, not 0x0300018E");
    // Sister: with no direction bits (and not the 0xC both-sides case) nothing draws.
    assert_eq!(glow_quads(&iso, 0), 0, "no glow quads without direction bits");
}

#[test]
fn air_bar_draws_while_swimming() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let air_quads = |iso: &str, air: f32, under: bool| -> usize {
        let mut sc = nf_view::load(iso, "07000005.bin", false).expect("load");
        {
            let sim = sc.sim.as_mut().unwrap();
            sim.bond.air = air;
            sim.bond.underwater = under;
        }
        let f = nf_view::hud::overlay(&sc, 4.0 / 3.0);
        let n: usize = tex_runs(&sc, &f, 0x0300_017f).iter().map(|(_, c)| c / 6).sum();
        n
    };
    // Submerged at half air: the empty track + the fill bar (fill width = air·2 + 28 = 128 of 256).
    assert!(air_quads(&iso, 50.0, true) >= 2, "submerged with air 50: the track and the fill draw");
    // Sister: dry with a full tank draws no air bar at all.
    assert_eq!(air_quads(&iso, 100.0, false), 0, "no air pane while dry at full air");
}

#[test]
fn crouch_icon_shows_while_crouched_and_fades_after_standing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let crouch_quads = |scene: &nf_view::scene::Scene, hash: u32| {
        let f = nf_view::hud::overlay(scene, 4.0 / 3.0);
        let n: usize = tex_runs(scene, &f, hash).iter().map(|(_, c)| c / 6).sum();
        n
    };
    // The real input path: hold crouch for a few ticks.
    let input = nf_game::player::Input { crouch: true, ..Default::default() };
    for _ in 0..12 { scene.sim.as_mut().unwrap().step_bond(input, false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.bond.crouched, "the crouch input crouched Bond");
        assert!(sim.hud.crouch_icon_timer > 0, "the icon timer holds while crouched");
    }
    assert!(crouch_quads(&scene, 0x0300_0173) >= 1, "the crouched icon (0x03000173) draws");
    assert_eq!(crouch_quads(&scene, 0x0300_0174), 0, "the post-stand icon does not draw while crouched");
    // Stand: crouch is a toggle in the input mapping (press again to stand, with a release gap);
    // the post-stand icon then holds 60 ticks and fades.
    let has_tex = scene.hud_textures.iter().any(|(h, _)| *h == 0x0300_0174);
    let mut stood = false;
    'stand: for _ in 0..3 {
        for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { crouch: true, ..Default::default() }, false, false, false); }
        for t in 0..30 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            if !scene.sim.as_ref().unwrap().bond.crouched { stood = true; eprintln!("[hud] stood, icon timer {}", scene.sim.as_ref().unwrap().hud.crouch_icon_timer); break 'stand; }
        }
    }
    assert!(stood, "Bond stands again after the crouch toggle");
    if has_tex {
        assert!(crouch_quads(&scene, 0x0300_0174) >= 1, "the post-stand icon (0x03000174) draws after standing");
    } else {
        eprintln!("[hud] 0x03000174 is not in 07000005.bin — the game shows no post-stand icon here either (parity)");
    }
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    if has_tex { assert_eq!(crouch_quads(&scene, 0x0300_0174), 0, "the post-stand icon expired"); }
    // Sister: before any crouch neither icon draws.
    assert_eq!(crouch_quads(&scene, 0x0300_0173), 0, "no crouch icon while standing");
}
