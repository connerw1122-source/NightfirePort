//! fix-weapon-effects — what each weapon DOES when fired, driven through the REAL path
//! (`nf_view::load` + `Sim::step_bond` with the trigger input), the counts asserted as numbers.
//!
//! The impact explode (`Bullet_CollisionHandler` 0x12d068-0x12d0bc, disassembled): after the impact
//! effects, a weapon whose `weapon_data+0x70` carries bit 2 (0x4) destroys its bullet with
//! `Explode_Create(radius = weapon_data+0x08, damage = weapon_data+0x0C, script 0x6000052,
//! attacker = BU+0x30)` at the hit point — unless this contact is a ricochet re-entry
//! (`bnel $s4` at 0x12d07c). Disc records [L ee_start_05.bin @ 0x2bf150]: AT-420 Sentinel 0x2c/0x2d
//! = radius 8 / damage 100 with the bit; Militek MGL 'Grenade' 0x2a = 6/75 with the bit, the 'Delay'
//! twin 0x2b = 6/75 without it (0x280 — bounce, fuse); the plain guns carry 0x12 (no bit).
//! The PCSX2 session measured the pair (docs/live/2026-10-13-weapon-modes-pcsx2.md): 0x2A exploded
//! on contact, 0x2B bounced and exploded ~3-4 s later.
//!
//! The casings rule: `Bullet_init_casing` 0x12fde0 runs from `Player_WeaponInitBullet` 0x1a4bf0:306
//! — only on the shot's release tick, never from movement.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Give a weapon through the dev panel (the user's `--give` path) and wait for the switch.
fn give_and_switch(scene: &mut nf_view::scene::Scene, id: u8) {
    let (msg, switch_to) = nf_view::panel::apply(scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[give] {id:#04x}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { return; }
    }
    panic!("the switch to {id:#04x} never completed");
}

fn alive_casings(scene: &nf_view::scene::Scene) -> usize {
    scene.sim.as_ref().unwrap().drones.casings.list.iter().filter(|c| c.state != 2).count()
}

/// ── Nothing spawns while walking: the casings fly only on the shot's release tick ───────────────
/// `Bullet_init_casing` 0x12fde0 is called from `Player_WeaponInitBullet` 0x1a4bf0:306 (the shot's
/// release), never from a movement path. The user saw casings fly while walking backwards on
/// 07000016; through the real path walking must spawn no casing, no bullet, no blast, no light.
#[test]
fn walking_backwards_spawns_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
    }
    let (c0, e0, l0) = (alive_casings(&scene), scene.sim.as_ref().unwrap().drones.explosions.len(), scene.sim.as_ref().unwrap().drones.lights.lights.len());
    for _ in 0..240 {
        let sim = scene.sim.as_mut().unwrap();
        // Backwards: forward < 0 (the walk input the user held), no trigger.
        sim.step_bond(Input { forward: -1.0, ..Default::default() }, false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.player_bullets.is_empty(), "a bullet spawned while walking");
        assert_eq!(sim.drones.explosions.len(), e0, "an explosion while walking");
        assert!(alive_casings(&scene) <= c0 + 0, "a casing ejected while walking");
        assert_eq!(sim.drones.lights.lights.len(), l0, "a muzzle light while walking");
    }
}

/// ── The Sentinel's missile explodes ON IMPACT with the record's blast ───────────────────────────
/// Pre-fix the port's world-hit path never exploded a bullet: the missile played the impact effects
/// and ricocheted like a rifle round. [M 0x12d068-0x12d0bc] + [L 0x2c = radius 8 / damage 100].
#[test]
fn the_sentinel_missile_explodes_on_impact() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    give_and_switch(&mut scene, 0x2c);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = -1.0; // straight down: the floor is the wall
    }
    let mut exploded = None;
    let mut armed = false;
    for t in 0..400u32 {
        let had = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), !armed, false, false);
        let now = scene.sim.as_ref().unwrap().drones.player_bullets.len();
        if now > had { armed = true; } // the round is out: release (the game's own pull shape)
        if !scene.sim.as_ref().unwrap().drones.explosions.is_empty() { exploded = Some(scene.sim.as_ref().unwrap().drones.explosions.clone()); break; }
    }
    let ex = exploded.expect("the Sentinel's missile must explode on the floor contact [M 0x12d0bc]");
    assert_eq!(ex.len(), 1, "exactly one blast");
    assert!((ex[0].1 - 8.0).abs() < 1e-3, "the blast radius is weapon_data+0x08 = 8.0 [L 0x2c], got {}", ex[0].1);
    assert!((ex[0].2 - 100.0).abs() < 1e-3, "the blast damage is weapon_data+0x0C = 100.0 [L 0x2c], got {}", ex[0].2);
    // Sister: the blast's effect script 0x6000052 plays at the blast, scale radius·0.0667 [M 0x17cbd0].
    assert!(scene.sim.as_ref().unwrap().drones.shake >= 7.0, "Camera_Shake(radius) from radius 0.5 [M 0x17cc28] (the tick-end decay 0.9·x−0.05 runs after the blast)");
}

/// Sister: the P2K's bullet does NOT explode — its record's +0x70 = 0x12 has no bit 2 [L].
#[test]
fn a_plain_gun_bullet_never_explodes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = -1.0;
    }
    for t in 0..120u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), t < 3, false, false);
        assert!(scene.sim.as_ref().unwrap().drones.explosions.is_empty(),
            "the P2K's bullet exploded (its +0x70 = 0x12 has no bit 2)");
    }
}

/// ── The MGL's 'Grenade' twin detonates on contact; the 'Delay' twin bounces ────────────────────
/// [L docs/live/2026-10-13-weapon-modes-pcsx2.md: 0x2A exploded on contact within ~0.3 s, 0x2B
/// bounced and exploded 3-4 s later; the user's "the MGL is always delayed"].
#[test]
fn the_mgl_grenade_mode_detonates_on_contact() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    give_and_switch(&mut scene, 0x2a);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.pitch = -1.0;
    }
    let mut exploded = None;
    for t in 0..200u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), t < 3, false, false);
        if !sim.drones.explosions.is_empty() { exploded = Some(sim.drones.explosions.clone()); break; }
    }
    let ex = exploded.expect("the MGL 'Grenade' round must explode on contact");
    assert!((ex[0].1 - 6.0).abs() < 1e-3, "the blast radius is weapon_data+0x08 = 6.0 [L 0x2a], got {}", ex[0].1);
    assert!((ex[0].2 - 75.0).abs() < 1e-3, "the blast damage is weapon_data+0x0C = 75.0 [L 0x2a], got {}", ex[0].2);
}

/// Sister: the 'Delay' twin (0x2b, +0x70 = 0x280 — no bit 2) bounces and does not explode early.
#[test]
fn the_mgl_delay_mode_bounces_and_waits_for_the_fuse() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    give_and_switch(&mut scene, 0x2a);
    // The 'Delay' record is the MODE twin (the ○ button, action 12 [M 0x1a24c8:246-260] — the
    // direct number-key select lands on the base; the live session switched with ○ too).
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x2b, "○ on the MGL requests the 'Delay' twin 0x2b");
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
        assert_eq!(sim.gun.weapon, 0x2b, "the switch completes onto the 'Delay' record");
        sim.bond.pitch = -1.0;
    }
    let mut contact_seen = false;
    for t in 0..120u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), t < 3, false, false);
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.explosions.is_empty(), "the 'Delay' round exploded early (fuse 210, not on contact)");
        if !sim.drones.grenades.is_empty() { contact_seen = true; }
    }
    assert!(contact_seen, "the 'Delay' round stayed alive past the contact (it bounces)");
}
