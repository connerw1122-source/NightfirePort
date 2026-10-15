//! fix-wallhug-prompt: the wall-shimmy prompt's volume and the Q-Claw's silent latch, through the
//! REAL input path (`nf_view::load` + `Sim::step_bond`, the Input the window builds).
//!
//! The user (playtest 1014-1725, 05 trench ledge): "That wall walking ledge area, its interaction
//! prompt area is a bit too large. Like I'm walking away from it and I still see it." and "hook
//! latched, pulled me in, but I also heard a bullet impact".
//!
//! The game's prompt volume is NOT a flat window: `Player_CollisionHandler` 0x1949a8:22 clears
//! BLData+0x95F = 0xFF each tick and `ThirdIcon_Update` 0x196e80 writes the context only into the
//! player object named by the icon's own obj+0xD0 straddle chain — whose candidates `Collide_Pick`
//! 0x1eb320 admits ONLY when the picker's sphere meets the candidate's world sphere
//! (`Vec_SqDist3D(picker+0x60, obj+0x80) < (picker+0x6C + obj+0x8C)²`, LAB_001eb4e4; the picker's
//! +0x6C = the player's world-sphere radius 1.0576 [L M_CONTROL+0x8C, ee_start_05 player 0
//! standing], the icon's = its `Control_BuildWorldSph` 0x133070 radius, 2.2757 on 05's ctx-6 icons
//! [D probe]). `Collide_Intersect` 0x1ebaa8's LAB_001ebf04 re-tests sphere-sphere. The port's old
//! [S] test — `dist(bond.pos, icon.centre) < 6.0` — measured the feet and reached ~1.8× past the
//! game's shell.
//!
//! The Q-Claw bolt: `Bullet_CollisionHandler` 0x12ce80:187-190 raises the effect flags to 0x61
//! only with `weapon_data+0x70 & 2`, and `Effect_Bullet` 0x1ce8a8:86-90 plays the
//! `EffectInfo+0x14` impact sound only on flags 0x40. The claw records 0x50/0x51 carry
//! +0x70 = 0x40 — the grapple bit alone [L ee_start_05.bin @ 0x2bf150 + 0x50/0x51·0x10C] — so the
//! bolt's world hit (the hook latch) is silent in the game; the port played the surface impact
//! unconditionally.

use nf_game::player::Input;
use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    scene.sim.take().expect("sim")
}

fn wait_for_control(sim: &mut Sim) {
    let idle = Input::default();
    for _ in 0..12000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
    for _ in 0..60 { sim.step_bond(idle, false, false, false); }
    assert!(!sim.disabled, "the intro NIS never released Bond");
}

/// The ctx-6 icon nearest a point (the parsed level data, not hardcoded positions).
fn nearest_ctx6(sim: &Sim, p: [f32; 3]) -> (usize, f32) {
    sim.icons.iter().enumerate().filter(|(_, i)| i.ctx == 6)
        .map(|(i, ic)| (i, ((ic.centre[0] - p[0]).powi(2) + (ic.centre[1] - p[1]).powi(2) + (ic.centre[2] - p[2]).powi(2)).sqrt()))
        .min_by(|a, b| a.1.total_cmp(&b.1)).expect("07000005 carries ctx-6 icons")
}

/// The prompt arms only inside the game's sphere (1.0576 + the icon's 2.2757 = 3.333 around the
/// SPHERE CENTRE, not the feet): the vent mouth (3.0 from icon[1]) still reads ctx 6 and posts the
/// "Press Action" hint; 1.6 further out (~4.9, inside the old flat 6.0) reads NONE — the user's
/// "I'm walking away from it and I still see it". Sisters: the grapple spot (8.17 [L PCSX2
/// 2026-10-14]) and the gate door (24.1 [L PCSX2 2026-10-13]) stay silent.
#[test]
fn the_crawl_prompt_arms_only_inside_the_games_sphere() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The vent mouth: the crawl out-point of creepwalls[0] (the fix_swim_vent5 construction — one
    // half-length-plus past the wall's end, on the crawl axis).
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    let w = sim.creepwalls[0].clone();
    let (a, b) = w.endpoints();
    let mouth = [b[0] - (a[0] - b[0]) / w.half_len.max(1e-3), w.pos[1], b[2] - (a[2] - b[2]) / w.half_len.max(1e-3)];
    let (icon_i, d_mouth) = nearest_ctx6(&sim, mouth);
    assert!(d_mouth < 3.333, "the test's mouth spot must sit inside the game's sphere (got {d_mouth:.2})");
    // Walking away: continue along the mouth→out direction, +2.2 past it — outside the game's
    // shell (2.5 + 2.2757 = 4.78 around the sphere centre), inside the old flat 6.0 window (the
    // fail-first proof).
    let icon_c = sim.icons[icon_i].centre;
    let away = [mouth[0] - icon_c[0], 0.0, mouth[2] - icon_c[2]];
    let al = (away[0] * away[0] + away[2] * away[2]).sqrt();
    let mid = [mouth[0] + away[0] / al * 2.2, mouth[1], mouth[2] + away[2] / al * 2.2];
    let d_mid = nearest_ctx6(&sim, mid).1;
    assert!(d_mid > 5.0 && d_mid < 6.0, "the mid spot must sit between the game's shell and the old window (got {d_mid:.2})");

    // At the mouth: ctx 6 and the "Press Action" hint posts [M ThirdIcon_Update 0x196e80:38-44;
    // the port binds the queue into hud.info_msgs on the same tick, Msg::label keeps the id].
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, mouth, 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 6, "the vent mouth must read the crawl icon [L PCSX2 2026-10-13] (got {})", sim.bond.icon_ctx);
    assert!(sim.hud.info_msgs.iter().any(|m| m.label == 0x100_0291),
        "the mouth must post the 'Press Action' hint (0x1000291)");

    // 1.6 further out: no context, no hint — the user's walking-away symptom.
    sim.hud.info_msgs.clear();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, mid, 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 0xff,
        "walking away must drop the prompt before the old 6.0 window ends [M Collide_Pick 0x1eb320 LAB_001eb4e4] (dist {d_mid:.2}, got {})", sim.bond.icon_ctx);
    assert!(!sim.hud.info_msgs.iter().any(|m| m.label == 0x100_0291), "no 'Press Action' hint out here");

    // Sisters (the live pins): the grapple spot and the gate door never arm.
    for (spot, why) in [([16.5f32, 29.6, 58.6], "the grapple spot [L PCSX2 2026-10-14]"),
                        ([30.0, 38.84, 37.70], "the gate door [L PCSX2 2026-10-13]")] {
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, spot, 0.0);
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
        assert_eq!(sim.bond.icon_ctx, 0xff, "{why} must read no icon (got {})", sim.bond.icon_ctx);
    }
}

/// The Q-Claw's bolt is SILENT on world hits: the records carry +0x70 = 0x40 — the grapple bit,
/// no effects bit — so `Bullet_CollisionHandler` 0x12ce80:187-190 never raises the effect flags
/// to 0x61 and `Effect_Bullet` 0x1ce8a8:86-90's 0x40 sound arm never runs. The user's "hook
/// latched, pulled me in, but I also heard a bullet impact" is the bolt that flies when the aim
/// misses the pick on the press tick (the port latches only when the pick accepts; a missed pick
/// lets the fire machine spawn the real claw bolt, which dies on the wall). The port played the
/// surface impact unconditionally. Sister: the same harness with the P2K (+0x70 = 0x12 [L]) DOES
/// hear the impact — the silence is the record bit, not a dead sound path.
#[test]
fn the_q_claw_latch_plays_no_bullet_impact() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    // The user's playtest spot; the wall-shot phase aims at the wall BEHIND the spot — outside
    // the pick's acceptance sphere (the ray points away from the point), so the press tick FIRES
    // the real claw bolt, which dies on that wall. (The forward castle wall at the pulley has no
    // collision at the bracket gap — the bolt dies at its range cap unsounded there; the mechanism
    // under test is the bolt's WORLD HIT, whichever wall.)
    let spot = [-18.28f32, 38.19, -71.83];
    let target = [-28.28f32, 39.70, -71.83];
    sim.bond.pos = spot;
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x50).max(1);
    let _ = sim.drones.inventory.equip_weapon(&wd, 0x50, rounds, true);
    sim.gun.switch_request = 0x50;
    let mut equipped = false;
    for _ in 0..120 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.gun.weapon == 0x50 { equipped = true; break; }
    }
    assert!(equipped, "the claw (0x50) must draw; weapon {:#x}", sim.gun.weapon);
    sim.bond.pos = spot;
    // Aim through the real look input at the wall spot.
    for _ in 0..200 {
        let (turn, look) = aim_inputs(&sim, target);
        sim.step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    // Every surface-impact id the bank knows — none may play while the claw bolt flies.
    let impact_ids: Vec<u32> = sim.surfaces.as_ref().expect("05 carries the surface table")
        .impact.iter().copied().filter(|&v| v != 0xffff).map(u32::from).collect();
    assert!(!impact_ids.is_empty(), "the impact column must parse");
    for _ in 0..120 {
        // The aim is converged; hold the trigger on a still input (the fire machine's shot ticks
        // spawn the claw bolt, which dies on the wall — SILENT in the game).
        sim.step_bond(Input::default(), true, false, false);
        assert!(sim.bond.grapple.is_none(),
            "the away-facing wall spot must not latch (the pick's ray points away from the point)");
        let active = sim.drones.audio.active_ids();
        let hit = active.iter().find(|id| impact_ids.contains(id));
        assert!(hit.is_none(),
            "the claw bolt's world hit must be silent [M Bullet_CollisionHandler 0x12ce80:187-190 + Effect_Bullet 0x1ce8a8:86-90; L 0x50/0x51 +0x70 = 0x40] — impact sound {} played (active {active:?})", hit.unwrap());
    }
    // The latch itself stays: refill the claw's clip directly (equip_weapon no-ops on the armed
    // weapon; a dry claw cannot re-latch through the fire machine's path), aim AT the point and
    // hold fire — the latch runs through the real path, and the reel is silent too.
    if let Some(c) = sim.drones.inventory.clip.get_mut(0x50usize) { *c = rounds; }
    sim.gun.clip = u32::from(rounds.max(0) as u16);
    sim.gun.switch_request = 0x50;
    for _ in 0..120 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.gun.weapon == 0x50 { break; }
    }
    for _ in 0..200 {
        let (turn, look) = aim_inputs(&sim, [-3.3962271, 42.940285, -71.76922]);
        sim.step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    let mut latched = false;
    for _ in 0..240 {
        sim.step_bond(Input::default(), true, false, false);
        if sim.bond.grapple.is_some() { latched = true; }
        let active = sim.drones.audio.active_ids();
        let hit = active.iter().find(|id| impact_ids.contains(id));
        assert!(hit.is_none(),
            "the latch and the reel must be silent too — impact sound {} played (active {active:?})", hit.unwrap());
    }
    assert!(latched, "the held fire at the point must latch (fix-gadgets-live's own pin)");

    // Sister: the P2K's bolt (+0x70 = 0x12, the effects bit set [L]) DOES play its impact — the
    // same harness hears the same sound class from a record with the bit. The aim is the wall
    // BEHIND the spot (the forward ray's castle wall carries the pulley's bracket gap; the
    // backward wall is the probe-proven hit).
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    sim.bond.pos = spot;
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x02).max(1);
    let _ = sim.drones.inventory.equip_weapon(&wd, 0x02, rounds, true);
    sim.gun.switch_request = 0x02;
    let mut equipped = false;
    for _ in 0..120 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.gun.weapon == 0x02 { equipped = true; break; }
    }
    assert!(equipped, "the P2K (0x02) must draw");
    sim.bond.pos = spot;
    let p2k_target = [-28.28f32, 39.70, -71.83]; // the wall behind the spot (the probe-proven hit)
    for _ in 0..100 {
        let (turn, look) = aim_inputs(&sim, p2k_target);
        sim.step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    let impact_ids: Vec<u32> = sim.surfaces.as_ref().expect("05 carries the surface table")
        .impact.iter().copied().filter(|&v| v != 0xffff).map(u32::from).collect();
    let mut heard = false;
    for _ in 0..120 {
        // The aim is converged; the fire phase holds the trigger on a still input (the shot spawns
        // on the fire script's frame; the bolt reaches the wall in a few ticks).
        sim.step_bond(Input::default(), true, false, false);
        if sim.drones.audio.active_ids().iter().any(|id| impact_ids.contains(id)) { heard = true; break; }
    }
    assert!(heard, "the P2K's wall hit must play its impact sound (the control for the gate)");
}

/// The aim helper the grapple tests use: nudge (turn, look) toward the target through the real
/// input lane.
fn aim_inputs(sim: &Sim, target: [f32; 3]) -> (f32, f32) {
    let head = sim.bond.head();
    let d = [target[0] - head[0], target[1] - head[1], target[2] - head[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-4);
    let want_yaw = d[0].atan2(d[2]);
    let want_pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / std::f32::consts::FRAC_PI_2;
    let dy = want_yaw - sim.bond.yaw;
    // Wrap to (−π, π]: the π·round() fold has a FIXED POINT exactly π away (dy = π maps to 0),
    // which stalled the aim at the antipode on the first run.
    let dy = dy - std::f32::consts::TAU * (dy / std::f32::consts::TAU).round();
    let dp = want_pitch - sim.bond.pitch;
    (dy.clamp(-0.05, 0.05), dp.clamp(-0.05, 0.05))
}
