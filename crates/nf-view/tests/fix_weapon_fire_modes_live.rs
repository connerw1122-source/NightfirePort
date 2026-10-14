//! fix-weapon-fire-modes: the MGL's impact/delay pair and the AT-420 Sentinel's guided missile
//! camera, through the real input path (`nf_view::load_with_carry` + `Sim::step_bond`).
//!
//! Ground truth: docs/live/2026-10-13-weapon-modes-pcsx2.md — the Militek MGL is a TWIN pair,
//! 0x2A 'Grenade' = IMPACT detonation, 0x2B 'Delay' = a ~210-tick fuse; the AT-420 Sentinel 0x2C
//! 'Guided' puts the view ON the rocket and the look input steers it, until impact (red static,
//! then Bond's view). The user: "The MGL grenade launcher is always delayed when I believe it
//! should be impact for one of the modes."

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, "07000005.bin", false, None).expect("load 07000005")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// The MGL's two modes through the real fire path: 0x2A 'Grenade' detonates ON the wall
/// (`+0x70 & 0x4` → `Explode_Create(wd+8, wd+8, wd+0xC, …)` [M Bullet_CollisionHandler
/// 0x12ce80:194-206; L ee_start_05.bin 0x2A's +0x70 = 0x00000004]), while the 'Delay' twin 0x2B
/// (+0x70 = 0x00000280 — 0x100 bounce + 0x200 fuse, no 0x4 [L]) bounces and dies on its 210-tick
/// fuse (`Bullet_DoTrails` 0x12d890:277 BU+0xFC −= 1/tick, exploded at < 0 [M]; the PCSX2 pass
/// measured ~3–4 s ≈ 210 ticks). The mode switch is the real B press: 0x2A's twin `+0x05` = 1 →
/// the switch request 0x2B (`Player_Weapon` 0x1a24c8:246-260), gated on `Player_WeaponHasAmmo`
/// (the twin needs rounds — the PCSX2 pass saw the press do nothing while 0x2B was empty).
#[test]
fn mgl_grenade_0x2a_detonates_on_impact_and_delay_0x2b_runs_its_fuse() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    let explosion_tick = |scene: &mut nf_view::scene::Scene, fire_ticks: u32| -> Option<(u32, usize)> {
        // Fire once (the real trigger through step_bond's `trigger`), then watch the blast.
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 6;
        sim.gun.pending = 0;
        let owned = sim.drones.inventory.owned.len();
        for w in [0x2ausize, 0x2b] {
            if w < owned { sim.drones.inventory.owned[w] = 1; }
            if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = 6; }
        }
        let mut input = Input::default();
        input.fire = true;
        let mut saw_grenade = false;
        let mut fired_at = None;
        let mut boom = None;
        for t in 0..fire_ticks {
            let sim = scene.sim.as_mut().unwrap();
            let before = sim.drones.explosions.len();
            sim.step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
            if sim.drones.grenades.iter().any(|g| g.weapon_type == 0x2a || g.weapon_type == 0x2b) { saw_grenade = true; if fired_at.is_none() { fired_at = Some(t); } }
            if sim.drones.explosions.len() > before { boom = Some((t, sim.drones.explosions.len())); break; }
        }
        assert!(saw_grenade, "the MGL fired a grenade projectile");
        boom
    };

    // Mode 1: 0x2A 'Grenade' — the wall detonates it, fast (the PCSX2 pass: "on contact, within
    // about 0.3 s"; the round at +0x20 = 0.44 u/tick reaches the start room's walls in ~20 ticks).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x2a;
        sim.gun.switch_request = 0x2a;
        sim.gun.switch_state = 0;
    }
    tick(&mut scene, 10);
    let boom = explosion_tick(&mut scene, 240).expect("0x2A 'Grenade' explodes on impact");
    assert!(boom.0 < 120, "the impact mode detonated on the wall within 2 s, got tick {}", boom.0);

    // Mode 2: the REAL mode press (the port's B key on the game's mode action) swaps to the twin
    // 0x2B 'Delay' (twin +0x05 = 1 [L]) — the switch request goes through the normal machinery.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x2b, "the mode press on 0x2A requests the 'Delay' twin 0x2B [M 0x1a24c8:246-260]");
    }
    tick(&mut scene, 200);
    {
        let sim = scene.sim.as_mut().unwrap();
        assert_eq!(sim.gun.weapon, 0x2b, "the twin swap completed onto the 'Delay' record");
    }
    // The 'Delay' round must NOT detonate on the wall: it bounces and runs its 210-tick fuse.
    let boom2 = explosion_tick(&mut scene, 400);
    assert!(boom2.is_none(), "0x2B 'Delay' must not explode on contact (the user's symptom is the reverse pairing; the fuse dies at ~210 ticks — a wall impact at tick {} would be wrong)", boom2.map(|b| b.0).unwrap_or(0));
}

/// Sister (unit, no ISO): the two records' flags drive the behaviour — `Grenade::explode_on_impact`
/// explodes at the FIRST contact and never at range-out; a fuse grenade never explodes on contact
/// and explodes at the fuse [M 0x12ce80:194-206 + Bullet_update 0x12e7b0:158-162's silent state 4].
#[test]
fn grenade_impact_flag_matches_the_records() {
    use nf_game::grenade::{Grenade, GrenadeEvent};
    use nf_game::world::{CollisionWorld, V3};
    let wall = CollisionWorld::from_triangles(vec![
        [[10.0, -10.0, -10.0], [10.0, 10.0, 10.0], [10.0, 10.0, -10.0]],
        [[10.0, -10.0, -10.0], [-10.0, -10.0, 10.0], [-10.0, 10.0, 10.0]],
    ]);
    let mk = |impact: bool| Grenade {
        pos: [0.0, 1.0, 0.0], velocity: [0.44, 0.0, 0.0], speed: 0.44, travelled: 0.0,
        range: if impact { 909.0 } else { 210.0 }, damage: 75.0, radius: 6.0,
        weapon_type: if impact { 0x2a } else { 0x2b }, mesh: 0x0200_0860, owner: usize::MAX,
        state: 0, bounces: 0, smoke_popped: false, explode_on_impact: impact,
    };
    // 0x2A: the first contact explodes (within ~23 ticks at 0.44/tick for 10 units)…
    let mut g = mk(true);
    let mut rng = nf_game::drone::Rng::new(0);
    let mut boom_at = None;
    for t in 0..100 {
        if let Some(GrenadeEvent::Explode { .. }) = g.update(&wall, &mut rng) { boom_at = Some(t); break; }
    }
    assert_eq!(boom_at, Some(22), "the impact grenade detonates at the wall [M 0x12ce80:194-206]");
    // …and never explodes at range-out (the game's silent state 4 [M 0x12e7b0:158-162]).
    let mut g = mk(true);
    let mut died_silent = false;
    for _ in 0..950 { if g.update(&CollisionWorld::empty(), &mut rng).is_some() { died_silent = false; break; } died_silent = g.state >= 2; }
    assert!(died_silent, "the impact grenade ranges out WITHOUT an explosion event");
    // 0x2B: a wall touch is a BOUNCE, the explosion only at the 210-tick fuse.
    let mut g = mk(false);
    let mut bounced = false;
    let mut events = Vec::new();
    for _ in 0..209 { if let Some(ev) = g.update(&wall, &mut rng) { if matches!(ev, GrenadeEvent::Bounce(_)) { bounced = true; } events.push(ev); } }
    assert!(bounced, "the delay grenade bounces off the wall");
    assert!(!events.iter().any(|e| matches!(e, GrenadeEvent::Explode { .. })), "no wall detonation before the fuse");
    let mut boomed = false;
    for _ in 0..4 { if let Some(GrenadeEvent::Explode { .. }) = g.update(&CollisionWorld::empty(), &mut rng) { boomed = true; break; } }
    assert!(boomed, "the delay grenade explodes at its 210-tick fuse [M Bullet_DoTrails 0x12d890:277]");
}

/// The AT-420 Sentinel's guided missile: firing 'Guided' 0x2C (`+0x6C = 0x12204` — the 0x4 bit [L])
/// hands the view to the rocket (`Bullet_init` 0x12c5a0:234-254: `Player_ChangeSubState(owner, 10)`,
/// `BLData+0x878 = the bullet`, `Player_SetCamMode(BLData, 0xc)`), the look steers it
/// (`Bullet_update` 0x12e7b0:177-244), and the impact freezes the view for the 15 ticks of
/// `BLData+0x944` before the normal view returns (`Bullet_Delete` 0x12e660:26-30 +
/// `Player_Update` 0x193600:242-256). Sister: the 'Unguided' twin 0x2D (`+0x6C = 0x1a200`, no 0x4
/// [L]) never rides the camera — the PCSX2 pass saw the camera only on 0x2C.
#[test]
fn the_sentinel_guided_mode_rides_the_missile_and_the_unguided_twin_does_not() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        for w in [0x2cusize, 0x2d] {
            if w < owned { sim.drones.inventory.owned[w] = 1; }
            if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = 4; }
        }
        sim.gun.weapon = 0x2c;
        sim.gun.switch_request = 0x2c;
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 4;
    }
    tick(&mut scene, 10);
    // Fire once: the camera must hand over to the missile.
    let mut input = Input::default();
    input.fire = true;
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(input, true, false, false);
    tick(&mut scene, 2);
    let sim = scene.sim.as_mut().unwrap();
    assert!(sim.missile_cam.is_some(), "the 'Guided' shot puts the camera on the missile [M 0x12c5a0:234-254]");
    let cam0 = sim.missile_cam.unwrap();
    assert!(cam0.riding, "the missile is alive and the view rides it");
    // The view override tracks the missile as it flies (speed +0x20 = 0.25 u/tick [L]).
    for _ in 0..20 { step_default(scene.sim.as_mut().unwrap()); }
    let sim = scene.sim.as_mut().unwrap();
    let cam1 = sim.missile_cam.expect("still riding after 20 ticks (range 200, speed 0.25)");
    let moved = ((cam1.pos[0] - cam0.pos[0]).powi(2) + (cam1.pos[1] - cam0.pos[1]).powi(2) + (cam1.pos[2] - cam0.pos[2]).powi(2)).sqrt();
    assert!(moved > 1.0, "the camera rides the flying missile (moved {:.2} units in 20 ticks)", moved);
    // The look input steers the missile's direction [M 0x12e7b0:177-244].
    let d0 = cam1.dir;
    for _ in 0..12 {
        let sim = scene.sim.as_mut().unwrap();
        let mut steer = Input::default();
        steer.turn = 1.0;
        if sim.missile_cam.is_some() && sim.missile_cam.unwrap().riding { sim.step_bond(steer, false, false, false); } else { break; }
    }
    let sim = scene.sim.as_mut().unwrap();
    if let Some(cam2) = sim.missile_cam {
        let dot = d0[0] * cam2.dir[0] + d0[1] * cam2.dir[1] + d0[2] * cam2.dir[2];
        assert!(dot < 0.999, "holding the turn input curves the missile (dot {:.5})", dot);
    }
    // The impact: the explosion, then the frozen view counts down and the normal view returns.
    let mut restored_at = None;
    for t in 0..300 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        if sim.missile_cam.is_none() { restored_at = Some(t); break; }
        let cam = sim.missile_cam.unwrap();
        if !cam.riding { assert!(cam.restore <= 15, "the static countdown starts at 15 [M 0x12e660:28]"); }
    }
    assert!(restored_at.is_some(), "the view returned to Bond after the missile died");
}

fn step_default(sim: &mut nf_view::scene::Sim) { sim.step_bond(Default::default(), false, false, false); }

/// Sister: the 'Unguided' twin 0x2D (+0x6C = 0x1a200 — no 0x4 bit [L ee_start_05.bin]) fires a
/// plain rocket: no camera handover ever.
#[test]
fn the_unguided_twin_never_rides_the_camera() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        for w in [0x2cusize, 0x2d] {
            if w < owned { sim.drones.inventory.owned[w] = 1; }
            if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = 4; }
        }
        sim.gun.weapon = 0x2d;
        sim.gun.switch_request = 0x2d;
        sim.gun.switch_state = 0;
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 4;
    }
    tick(&mut scene, 10);
    let mut input = Input::default();
    input.fire = true;
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(input, true, false, false);
    for _ in 0..60 { step_default(scene.sim.as_mut().unwrap()); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.missile_cam.is_none(), "the 'Unguided' record has no `+0x6C & 4` — the camera stays with Bond [L record; M 0x12c5a0:234]");
}
