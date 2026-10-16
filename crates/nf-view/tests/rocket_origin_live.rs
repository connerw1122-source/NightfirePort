//! rocket-origin: a fired bullet with +0x68 & 0x10000 (the rocket launcher's unguided 0x2d) starts at the
//! MIDPOINT of the head and the view model's datum 0 and is re-aimed at head + aim·256
//! [M Player_WeaponInitBullet 0x1a4bf0:226-245; Player_GetAimingPoint 0x1a4a98]. The port fired from the
//! eye, so the rocket's own mesh filled the screen on the launch frame (07000008, against PCSX2).
//! Opt-in: set `NF_ISO` to the disc image.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_rocket_leaves_the_launcher_not_the_eye() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(iso) = iso() else { return };
    let mut scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x2d]).expect("load 08");
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x2d).max(1);
    assert!(sim.drones.inventory.equip_weapon(&wd, 0x2d, rounds, true), "equip the unguided launcher");
    sim.drones.inventory.equip_ammo(&wd, 0x2d, rounds * 3);
    sim.gun.switch_request = 0x2d; // the `--give` path's switch [lib.rs shot path]
    for _ in 0..150 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 0x2d, "the launcher is drawn");
    let before = sim.drones.player_bullets.len();
    let mut shot = None;
    for _ in 0..30 {
        let head = sim.bond.head();
        let look = sim.bond.look();
        sim.step_bond(Default::default(), true, false, false);
        if sim.drones.player_bullets.len() > before { shot = Some((head, look)); break; }
    }
    let (head, look) = shot.expect("a rocket was fired");
    let b = sim.drones.player_bullets.last().unwrap();
    // Back the bullet up along its own flight to the spawn point (it may already have moved this tick).
    let start = [b.pos[0] - b.dir[0] * b.travelled, b.pos[1] - b.dir[1] * b.travelled, b.pos[2] - b.dir[2] * b.travelled];
    let off = [start[0] - head[0], start[1] - head[1], start[2] - head[2]];
    let along = off[0] * look[0] + off[1] * look[1] + off[2] * look[2];
    let side = [off[0] - look[0] * along, off[1] - look[1] * along, off[2] - look[2] * along];
    let side_d = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
    assert!(side_d > 0.05, "the rocket starts off the eye's line (towards the launcher), off-axis {side_d:.3}");
    // Sister: it still flies at the aiming point — 256 units out it is within 4 units of head + look·256
    // (the record's own spread, `bullet::spread`, accounts for the rest; ~1°).
    let far = [start[0] + b.dir[0] * 256.0, start[1] + b.dir[1] * 256.0, start[2] + b.dir[2] * 256.0];
    let aim = [head[0] + look[0] * 256.0, head[1] + look[1] * 256.0, head[2] + look[2] * 256.0];
    let miss = ((far[0] - aim[0]).powi(2) + (far[1] - aim[1]).powi(2) + (far[2] - aim[2]).powi(2)).sqrt();
    assert!(miss < 4.0, "aimed at the aiming point, missed by {miss:.2}");
}
