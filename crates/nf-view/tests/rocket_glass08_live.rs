//! rocket-glass-08: the 07000008 corridor rocket flew ~5 units short — it exploded on the far
//! window's pane (material 0x10) instead of the wall behind it. The game's
//! `Bullet_CollisionHandler` 0x12ce80 lets every bullet pass material 0x10 (glass) unless the
//! shooter's `weapon_data+0x6c` carries 0x800 [M disasm 0x12cf44..0x12cf64], so the PS2 rocket
//! crosses the pane and explodes on the wall past it (~24 units, ~72 frames; side-by-side vs
//! PCSX2 2026-10-16). The port's walk lacked the rule and stopped the round on the pane at
//! 19.25 units (56 ticks). fix-rocket-glass-08.
//! Opt-in: set `NF_ISO` to the disc image.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The glass rule, unit-level: material 0x10 pierces without a draw for a weapon whose +0x6c
/// lacks 0x800; the Laser (0x4E, +0x6c = 0x900 [L ee_start_05.bin]) keeps the gate and stops.
#[test]
fn glass_passes_unless_the_weapon_burns_it() {
    let mut rng = nf_game::drone::Rng::new(0x1234);
    // the unguided launcher 0x2d's record +0x6c = 0x1a200 [L ee_start_05.bin @ 0x2bf150] — no
    // 0x800: the pane passes. This asserted true only after fix-rocket-glass-08.
    let draws_before = rng.draws;
    assert!(nf_game::effect::penetration_roll(&mut rng, 0x10, 0x2d, 0x1_a200),
        "the corridor window (material 0x10) lets the rocket through");
    assert_eq!(rng.draws, draws_before, "the glass rule draws no RNG");
    // the Laser 0x4E's +0x6c = 0x900 [L] — bit 0x800 set: the beam stops on the pane.
    assert!(!nf_game::effect::penetration_roll(&mut rng, 0x10, 0x4e, 0x900),
        "the laser's 0x800 keeps it from passing glass");
    // the plain wall byte from the same corridor (0x0a) never pierces, whatever the weapon.
    assert!(!nf_game::effect::penetration_roll(&mut rng, 0x0a, 0x2d, 0x1_a200),
        "a plain wall still stops the round");
}

/// The task's shot: the unguided launcher fired from the 07000008 level start straight down the
/// corridor. The port's rocket must cross the window plane (z −96.75, material 0x10) and explode
/// on the wall face behind it (z −100.0, material 0x0a) — the game's explosion point.
#[test]
fn the_08_corridor_rocket_explodes_past_the_window() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(iso) = iso() else { return };
    let mut scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x2d]).expect("load 08");
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x2d).max(1);
    assert!(sim.drones.inventory.equip_weapon(&wd, 0x2d, rounds, true), "equip the unguided launcher");
    sim.drones.inventory.equip_ammo(&wd, 0x2d, rounds * 3);
    sim.gun.switch_request = 0x2d;
    for _ in 0..150 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 0x2d, "the launcher is drawn");
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }

    let mut spawn = None;
    for _ in 0..30 {
        let before = sim.drones.player_bullets.len();
        sim.step_bond(Default::default(), true, false, false);
        if sim.drones.player_bullets.len() > before {
            let b = sim.drones.player_bullets.last().unwrap().clone();
            spawn = Some([b.pos[0] - b.dir[0] * b.travelled, b.pos[1] - b.dir[1] * b.travelled, b.pos[2] - b.dir[2] * b.travelled]);
            break;
        }
    }
    let spawn = spawn.expect("a rocket was fired");

    // Fly the round to its death, remembering the last live position.
    let mut last = spawn;
    for _ in 0..140 {
        if sim.drones.player_bullets.iter().all(|b| b.weapon != 0x2d) { break; }
        sim.step_bond(Default::default(), false, false, false);
        if let Some(b) = sim.drones.player_bullets.iter().find(|b| b.weapon == 0x2d) {
            last = b.pos;
        }
    }
    assert!(sim.drones.player_bullets.iter().all(|b| b.weapon != 0x2d), "the rocket exploded within 140 ticks");

    // The world impact the flush recorded is the explosion point.
    let blast = sim.drones.player_impacts.iter().find_map(|i| match i {
        nf_game::drone::PlayerImpact::World(p) => Some(*p),
        _ => None,
    }).expect("the rocket's world impact was recorded");

    let dist = ((blast[0] - spawn[0]).powi(2) + (blast[1] - spawn[1]).powi(2) + (blast[2] - spawn[2]).powi(2)).sqrt();
    // The wall face behind the window sits at z −100.0 [D 07000008 collision, the aim ray's first
    // non-pass surface past the pane]; the pane it must cross is at z −96.75 (material 0x10).
    assert!(blast[2] < -98.5, "the rocket exploded past the window plane (−96.75), at z {:.3}", blast[2]);
    assert!((blast[2] + 100.0).abs() < 0.5, "the rocket exploded on the wall face at z −100.0, got z {:.3}", blast[2]);
    assert!((21.9..=24.4).contains(&dist),
        "the flight covers the game's ~24 units (PS2 ~72 frames × 0.3367 u/frame [L]), got {dist:.3}");
    // Sister check inside the same shot: the surface at the blast is the plain wall, not glass.
    let end = [last[0], last[1], last[2] - 3.0];
    if let Some((_, _, m, _)) = sim.world.segment_surface_flags(last, end, false) {
        assert_eq!(m & 0x3f, 0x0a, "the blast sits on the plain wall (material 0x0a), got {m:#04x}");
    }
}

/// Sister: a shot at a plain wall still stops ON its face — the fix must not let rounds tunnel
/// through ordinary surfaces.
#[test]
fn a_shot_at_a_plain_wall_stops_on_its_face() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(iso) = iso() else { return };
    let mut scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x2d]).expect("load 08");
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x2d).max(1);
    assert!(sim.drones.inventory.equip_weapon(&wd, 0x2d, rounds, true), "equip the unguided launcher");
    sim.drones.inventory.equip_ammo(&wd, 0x2d, rounds * 3);
    sim.gun.switch_request = 0x2d;
    for _ in 0..150 { sim.step_bond(Default::default(), false, false, false); }
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }

    // Aim at the corridor floor a few units ahead and check the aim surface is plain first.
    sim.bond.pitch = -1.2;
    for _ in 0..3 { sim.step_bond(Default::default(), false, false, false); }
    let head = sim.bond.head();
    let look = sim.bond.look();
    let aim_end = [head[0] + look[0] * 30.0, head[1] + look[1] * 30.0, head[2] + look[2] * 30.0];
    let (_, _, mat, node_pass) = sim.world.segment_surface_flags(head, aim_end, false)
        .expect("the aim ray hits the floor");
    assert!(!node_pass && mat & 0xc0 == 0 && mat & 0x3f != 0x10,
        "precondition: the sister aim surface must be plain, got {mat:#04x}");

    let mut spawn = None;
    for _ in 0..30 {
        let before = sim.drones.player_bullets.len();
        sim.step_bond(Default::default(), true, false, false);
        if sim.drones.player_bullets.len() > before {
            let b = sim.drones.player_bullets.last().unwrap().clone();
            spawn = Some([b.pos[0] - b.dir[0] * b.travelled, b.pos[1] - b.dir[1] * b.travelled, b.pos[2] - b.dir[2] * b.travelled]);
            break;
        }
    }
    let spawn = spawn.expect("a rocket was fired");
    for _ in 0..140 {
        if sim.drones.player_bullets.iter().all(|b| b.weapon != 0x2d) { break; }
        sim.step_bond(Default::default(), false, false, false);
    }
    let blast = sim.drones.player_impacts.iter().find_map(|i| match i {
        nf_game::drone::PlayerImpact::World(p) => Some(*p),
        _ => None,
    }).expect("the rocket's world impact was recorded");
    // The blast lies on the aim surface's plane (the head-aimed ray's hit height, ± the sphere
    // the walk's own hit test covers): it did NOT fly past the face.
    let aim_t = ((blast[0] - head[0]) * look[0] + (blast[1] - head[1]) * look[1] + (blast[2] - head[2]) * look[2]);
    let aim_pt = [head[0] + look[0] * aim_t, head[1] + look[1] * aim_t, head[2] + look[2] * aim_t];
    let off = ((blast[0] - aim_pt[0]).powi(2) + (blast[1] - aim_pt[1]).powi(2) + (blast[2] - aim_pt[2]).powi(2)).sqrt();
    assert!(off < 1.0, "the rocket stopped on the wall's face (off the aimed surface by {off:.3})");
    let dist = ((blast[0] - spawn[0]).powi(2) + (blast[1] - spawn[1]).powi(2) + (blast[2] - spawn[2]).powi(2)).sqrt();
    assert!(dist < 30.0, "the sister shot stops close by, at {dist:.3}");
}
