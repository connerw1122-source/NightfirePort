//! The player's shell ejects when the weapon's +0x78 delay runs out, from the view model's eject datum
//! as posed THEN (the user, 1015-0046: "I don't see any shotgun shells flying out").
//!
//! Evidence base:
//! - `Bullet_init_casing` 0x12fde0: at the shot only the pool slot, state 3, the mesh and the delay
//!   (`weapon_data+0x78` as f32) are set [M :9-30].
//! - `Bullet_casing_update` 0x12faa0:31-37: state 3 counts the delay down and then calls
//!   `Bullet_init_casing_ex` 0x12f678, which reads the shooter's datum (`AnimGetBoneWorldTrans`, :88-99)
//!   and draws the 7 RNG values (:127-180).
//! - PCSX2 07000001 (Frinesi 0x1c, +0x78 = 53): the shell appears mid-pump, large and next to the camera,
//!   flying up and to the left (2026-10-15 burst capture).
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn draw(sim: &mut nf_view::scene::Sim, id: usize) {
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(id).max(1);
    if sim.drones.inventory.equip_weapon(&wd, id, rounds, true) { sim.drones.inventory.equip_ammo(&wd, id, rounds * 3); }
    sim.gun.switch_request = id as u8;
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon as usize, id, "the weapon is drawn");
}

fn live(sim: &nf_view::scene::Sim) -> Vec<nf_game::casing::Casing> {
    sim.drones.casings.list.iter().filter(|c| c.state == 0 || c.state == 3).cloned().collect()
}

#[test]
fn the_frinesi_shell_ejects_mid_pump_next_to_the_camera() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    draw(sim, 0x1c);
    let delay = sim.weapons.records[0x1c][0x78] as u32;
    assert_eq!(delay, 53, "the Frinesi's eject delay [L weapon_data]");
    sim.step_bond(Input::default(), true, false, false);
    let after_shot = live(sim);
    assert_eq!(after_shot.len(), 1, "one shell reserved by the shot");
    assert!(after_shot[0].pending && after_shot[0].state == 3, "reserved, not yet ejected [M 0x12fde0]");
    let mut armed_at = None;
    for t in 1..=delay + 2 {
        sim.step_bond(Input::default(), false, false, false);
        if armed_at.is_none() && live(sim).iter().any(|c| c.state == 0) { armed_at = Some(t); }
    }
    let t = armed_at.expect("the shell ejected");
    assert!(t >= delay - 1 && t <= delay + 1, "the shell ejects when the delay runs out (tick {t}, delay {delay})");
    // Where: next to the camera, since the pump has pulled the gun up to the face (PCSX2: the shell crosses the view large).
    let c = live(sim).into_iter().find(|c| c.state == 0).unwrap();
    let eye = sim.bond.head();
    let d = ((c.pos[0] - eye[0]).powi(2) + (c.pos[1] - eye[1]).powi(2) + (c.pos[2] - eye[2]).powi(2)).sqrt();
    eprintln!("shell {d:.3} from the eye, vel {:?}", c.vel);
    assert!(d < 0.8, "the shell is next to the camera mid-pump ({d})");
}

/// Sister: a no-delay gun (the P2K, +0x78 = 0) ejects in the same tick it fires.
#[test]
fn a_no_delay_shell_ejects_on_the_shot_tick() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    draw(sim, 0x02);
    assert_eq!(sim.weapons.records[0x02][0x78], 0);
    sim.step_bond(Input::default(), true, false, false);
    let l = live(sim);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].state, 0, "ejected the same tick");
    assert!(!l[0].pending);
}
