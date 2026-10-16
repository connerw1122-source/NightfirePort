//! delta-repeater: the two playtest-1015-1738 fixes for the Delta Repeater (the crossbow 0x11),
//! through the real input path (`nf_view::load_weapons` + `Sim::step_bond`).
//!
//! 1. **The bolt's world hit.** The game's bolt is a GRAVITY bullet — `weapon_data+0x6C & 0x8`
//!    is `Bullet_update` 0x12e7b0 case 1's gravity arm (`velocity = dir·speed;
//!    velocity += WldGravity·(1/60)²; Vec_NormaliseLen`), NOT a grenade-family marker. Its wall
//!    hit is `Bullet_CollisionHandler` 0x12ce80's plain arm: state 2 (dead), the 0x61 effect
//!    stack (weapon `+0x70 & 2`), and NONE of the bounce (0x80), stick (0x100) or ricochet
//!    (0x10) arms — record 0x11 carries +0x6C = 0x00000808, +0x70 = 0x00000002
//!    [L ee_start_05.bin @ 0x2bf150 + 0x11·0x10C]. The port's old dispatch routed every
//!    `+0x6C & 8` weapon into the GRENADE path, so the bolt bounced off walls with the grenade
//!    bounce sounds — the user's "it sounds like a grenade is bouncing when the crossbow flies
//!    out and bounces against the wall when it shouldn't be bouncing". The new gate is the
//!    gravity bit AND a grenade-family collision flag (0x4 / 0x80 / 0x100); the record sweep
//!    [L] moves exactly 0x11 off the grenade path.
//! 2. **The loaded bolts.** `Player_WeaponFiring` 0x1a5460:331-381 re-issues the view model's
//!    datum entities EVERY tick for the base-0x11 records: the bolt mesh (the record's +0x5C =
//!    0x0200816, the flying projectile's own mesh) on datum 4 when count > 0 (NOT while the view
//!    state is 9), datum 5 when > 1, datum 6 when >= 3; count = clip[0x11], +1 while the fire
//!    script plays, min(carried[+0x90 = 0x0F], clip_size +0x92 = 3) once the reload script
//!    passes frame 80 [M]. The user's "model is missing the arrows".

use nf_game::player::Input;
use nf_game::drone::PlayerImpact;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// Gives `id` through the real give path (the dev menu's Give button — `Player_EquipWeapon`
/// 0x1a8a58 + three clips), via the panel exactly as fire_modes3_live's `give`.
fn give(scene: &mut nf_view::scene::Scene, id: u8, rounds: i16) -> bool {
    let (_, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("0x{id:02x}")),
        ("rounds".to_string(), rounds.to_string()),
    ]);
    sw == Some(id)
}

/// Draws `id` through the real input path, then settles (fire_modes3_live's `draw`).
fn draw(scene: &mut nf_view::scene::Scene, id: u8) -> bool {
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
    }
    let ok = scene.sim.as_ref().unwrap().gun.weapon == id;
    for _ in 0..200 {
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    tick(scene, 5);
    ok
}

/// Arms the crossbow for a clean shot (the harness's direct setup).
fn arm(scene: &mut nf_view::scene::Scene, clip: i16) {
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.reload_state = 0;
    sim.gun.fire_state = false;
    sim.gun.pending = 0;
    sim.gun.clip = clip.max(0) as u32;
    if let Some(c) = sim.drones.inventory.clip.get_mut(0x11) { *c = clip; }
}

/// Pitches the view down over `ramp` ticks (the real look input).
fn aim_down(scene: &mut nf_view::scene::Scene, ramp: u32, look: f32) {
    for _ in 0..ramp {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input { look, ..Default::default() }, false, false, false);
    }
}

/// A collapsed matrix lost its rotation (the port collapses a hidden mesh to a point by zeroing
/// the 3×3, `Scene::frame_state`'s `collapse`).
fn collapsed(m: [[f32; 4]; 4]) -> bool { m[0][0] == 0.0 && m[1][1] == 0.0 }

fn quiver_rows(scene: &nf_view::scene::Scene) -> Vec<(u8, [[f32; 4]; 4])> {
    let Some(vm) = scene.view_models.get(&0x11).copied() else { return Vec::new() };
    let fs = scene.frame_state(None, 0.0, Default::default());
    scene.attachments.iter().zip(fs.attach.iter())
        .filter(|(a, _)| a.quiver_bolt.is_some() && a.character as usize == vm)
        .map(|(a, m)| (a.quiver_bolt.unwrap(), *m)).collect()
}

fn lit_slots(scene: &nf_view::scene::Scene) -> Vec<u8> {
    quiver_rows(scene).iter().filter(|(_, m)| !collapsed(*m)).map(|(s, _)| *s).collect()
}

/// The bolt's world hit: a gravity bullet that DIES at the wall — no grenade, no bounce, no
/// second impact. Through the real fire path: give → draw → aim → one trigger press.
#[test]
fn bolt_is_a_gravity_bullet_that_dies_at_the_wall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x11]).expect("load 05 with the crossbow");
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x11, 9), "give the Delta Repeater");
    assert!(draw(&mut scene, 0x11), "draw the Delta Repeater");
    arm(&mut scene, 3);
    // A SHALLOW down-aim (2 ticks: the look input pitches ~0.16 rad/tick, so more would meet the
    // floor inside a few ticks): the bolt must fly forward (a non-vertical direction) so the
    // gravity arm's curvature is measurable in dir[1] — a straight-down shot normalises back onto
    // −y. The bolt crawls at 0.667 u/tick (+0x20 [L]; +0x6C = 0x808 carries NO 0x200 quarter bit);
    // gravity's −0.00272/tick on vy drops it ~2 units over 40 ticks — the floor catches it
    // mid-room with dozens of flight samples.
    aim_down(&mut scene, 2, -0.08);

    let sim = scene.sim.as_mut().unwrap();
    let g0 = sim.drones.grenades.len();
    let mut input = Input::default();
    input.fire = true;
    let mut flight_dirs: Vec<f32> = Vec::new();
    let mut world_impacts = 0u32;
    let mut saw_bullet = false;
    for t in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(b) = sim.drones.player_bullets.iter().find(|b| b.weapon == 0x11) {
            saw_bullet = true;
            flight_dirs.push(b.dir[1]);
        }
        // `player_impacts` is per-tick (cleared at the head of `update_player_bullets`), so each
        // tick's World entries are this tick's hits; a bounce would surface as a second hit tick.
        world_impacts += sim.drones.player_impacts.iter().filter(|i| matches!(i, PlayerImpact::World(_))).count() as u32;
        if sim.drones.grenades.iter().any(|g| g.weapon_type == 0x11) {
            panic!("the crossbow's round spawned a GRENADE — the old gravity-only dispatch is back");
        }
    }
    assert!(saw_bullet, "the shot spawned a player bullet of weapon 0x11");
    // Gravity: `Bullet_update` case 1's arm adds WldGravity·(1/60)² to the velocity's y every
    // flight tick (−0.00272/tick on a 0.167 u/tick bolt — the lobbed arc), so dir[1] must FALL
    // over the flight.
    assert!(flight_dirs.len() >= 3, "the bolt flew several ticks (speed 0.667 [L +0x20]), got {}", flight_dirs.len());
    let last = *flight_dirs.last().unwrap();
    let first = flight_dirs[0];
    assert!(last < first, "the bolt's dir[1] must fall under gravity ({} → {})", first, last);
    // The wall hit: exactly ONE world impact and a dead bullet — the game's plain arm (state 2)
    // with no bounce (0x80), stick (0x100) or ricochet (0x10) on the record [L +0x70 = 0x2].
    assert_eq!(world_impacts, 1, "the bolt hits the world exactly once (no bounce, no ricochet)");
    let alive = scene.sim.as_ref().unwrap().drones.player_bullets.iter().filter(|b| b.weapon == 0x11).count();
    assert_eq!(alive, 0, "the bolt died at the wall (state 2, no revive)");
    assert_eq!(scene.sim.as_ref().unwrap().drones.grenades.len(), g0, "no grenade appeared");
}

/// Sister: the MGL 0x2A (+0x6C = 0x828, +0x70 = 0x4 [L]) KEEPS its grenade routing — the new
/// gate (gravity AND a family collision flag) leaves every other gravity record in place.
#[test]
fn mgl_still_spawns_a_grenade() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2a]).expect("load 05 with the MGL");
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x2a, 6), "give the MGL");
    assert!(draw(&mut scene, 0x2a), "draw the MGL");
    arm(&mut scene, 6);
    aim_down(&mut scene, 40, -1.0);
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.clip = 6;
    let mut input = Input::default();
    input.fire = true;
    let mut saw_grenade = false;
    for t in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
        if scene.sim.as_ref().unwrap().drones.grenades.iter().any(|g| g.weapon_type == 0x2a) { saw_grenade = true; break; }
    }
    assert!(saw_grenade, "the MGL's round is still a grenade (gravity + the 0x4 contact-explode flag)");
}

/// The loaded bolts: datum 4 when count > 0 (not in fire state 9), datum 5 when > 1, datum 6
/// when >= 3; count = clip[0x11], min(carried[0x0F], 3) in reload past frame 80. The FP skin
/// 0x05000083 (the datums on bone 40) lives in 07000014 [D 07000014.bin kind-3].
#[test]
fn quiver_shows_the_loaded_bolts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000014.bin", false, &[0x11]).expect("load 014 with the crossbow");
    tick(&mut scene, 30);
    assert!(scene.view_models.contains_key(&0x11), "the crossbow's first-person view model built (FP skin +0xDC in the level)");
    assert!(give(&mut scene, 0x11, 9), "give the Delta Repeater");
    assert!(draw(&mut scene, 0x11), "draw the Delta Repeater");
    arm(&mut scene, 3);
    tick(&mut scene, 3);

    let rows = quiver_rows(&mut scene);
    let mut ids: Vec<u8> = rows.iter().map(|(s, _)| *s).collect();
    ids.sort();
    assert_eq!(ids, vec![4, 5, 6], "three bolt slots on the skin's datums 4/5/6");
    assert_eq!(lit_slots(&scene), vec![4, 5, 6], "clip 3 → all three bolts show");

    // clip 1: only the top slot.
    if let Some(c) = scene.sim.as_mut().unwrap().drones.inventory.clip.get_mut(0x11) { *c = 1; }
    assert_eq!(lit_slots(&scene), vec![4], "clip 1 → only datum 4");
    // clip 0: none.
    if let Some(c) = scene.sim.as_mut().unwrap().drones.inventory.clip.get_mut(0x11) { *c = 0; }
    assert!(lit_slots(&scene).is_empty(), "clip 0 → no bolts");
    // The fire state (+1, datum 4 suppressed): clip 2 → count 3 → 5 and 6, not 4.
    {
        let sim = scene.sim.as_mut().unwrap();
        if let Some(c) = sim.drones.inventory.clip.get_mut(0x11) { *c = 2; }
        sim.gun.fire_state = true;
    }
    assert_eq!(lit_slots(&scene), vec![5, 6], "fire state 9 → count +1, datum 4 suppressed");
    {
        scene.sim.as_mut().unwrap().gun.fire_state = false;
    }
    // The reload arm: state 6 past frame 80 → min(carried[0x0F], clip_size 3).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.reload_state = 1;
        sim.view_time = 90.0;
        let at = sim.weapons.ammo_type(0x11);
        if let Some(c) = sim.drones.inventory.carried.get_mut(at) { *c = 2; }
        if let Some(c) = sim.drones.inventory.clip.get_mut(0x11) { *c = 0; }
    }
    assert_eq!(lit_slots(&scene), vec![4, 5], "reload past frame 80 → min(carried 2, 3) = 2 bolts");
    // Before frame 80 the count is still the clip (0): nothing.
    {
        scene.sim.as_mut().unwrap().view_time = 30.0;
    }
    assert!(lit_slots(&scene).is_empty(), "reload before frame 80 → the clip's count (0)");
}
