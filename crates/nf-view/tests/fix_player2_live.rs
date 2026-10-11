//! fix-player2 repro tests, driven through the REAL path the viewer uses (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's playtest on master 75fbad4 (07000001 unless noted):
//! 1. throwing a grenade does nothing and breaks shooting afterwards;
//! 2. Raptor drop pickups are unreliable (several passes needed, some never pick up, none after the first);
//! 3. the armour pickup is taken but its model is invisible;
//! 4. aiming at a civilian should show the red "no" crosshair (`Check_Target` 0x1a7810 / `HUD_UpdateCrossHair` 0x19e6d0);
//! 5. the picked-up sniper rifle (0x24) can't fire.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

/// Point Bond's view at `to` from his eye (setup helper; `turn` stays 0 so the aim holds).
fn aim_at(sim: &mut nf_view::scene::Sim, to: [f32; 3]) {
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Log one take test the way the task asks: Bond's position, the pickup centre, the distance against
/// radius + 1.1, the LOS ray, and the result.
fn take_test_line(sim: &nf_view::scene::Sim, k: usize) {
    let p = &sim.drones.pickups[k];
    let c = p.centre();
    let r = p.bounds.radius + 1.1;
    let d2: f32 = (c[0] - sim.bond.pos[0]).powi(2) + (c[1] - sim.bond.pos[1]).powi(2) + (c[2] - sim.bond.pos[2]).powi(2);
    let los = sim.world.line_of_sight(sim.bond.sphere_centre(), c);
    eprintln!("[take-test] tick {} pickup {k} weapon {:#x} kind {} state {}: bond {:?} centre {:?} dist {:.3} vs r {:.3} los {}",
        sim.drones.frame, p.weapon, p.kind, p.state, sim.bond.pos, c, d2.sqrt(), r, los);
}

// ── Bug 1: the grenade throw ────────────────────────────────────────────────────────────────────

/// Equip the frag grenade (0x35) through the real take path (a kind-0 pickup at Bond's feet), press fire,
/// and hold: the game plays the throw script and spawns the grenade at its release frame
/// (`fire_anim_len` 26 [L ee_start_07000001.bin weapon_data 0x35 +0x44]; `Player_SetWeaponAnimObj`
/// 0x1a2e38 case 9; `Player_WeaponInitBullet` 0x1a4bf0 with flags68 & 0x10000 = hand spawn).
/// Afterwards the weapon state machine must return to idle and the P2K must fire again.
#[test]
fn grenade_throw_spawns_at_the_release_frame_and_the_gun_recovers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // A frag-grenade pickup (the captain-drop form: kind 0, id 0x35 [M DroneFunc_DropGrenade 0x17a2a0])
    // at Bond's feet, built the way the game's drop constructor builds it.
    let id = 0x35usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.3 });
    let at = sim.bond.pos;
    let p = nf_game::pickup::Pickup::create_placed(at, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons)
        .expect("grenade pickup builds");
    sim.drones.pickups.push(p);
    tick_idle(sim, 5);
    assert!(sim.drones.taken.iter().any(|(_, t)| matches!(t, nf_game::pickup::Taken::Weapon { weapon: 0x35 })),
        "the grenade pickup is taken through the real path (taken = {:?})", sim.drones.taken);
    assert!(sim.drones.inventory.owned[0x35], "0x35 owned after the take");

    // Equip it through the switch state machine (the take leaves the P2K in hand, as in play).
    sim.gun.switch_request = 0x35;
    tick_idle(sim, 60);
    assert_eq!(sim.gun.weapon, 0x35, "the grenade is equipped after the holster/draw cycle");
    assert!(sim.gun.timer <= 0.0, "sister: the refire timer is run out before the throw ({})", sim.gun.timer);

    // Press fire, release. The port must NOT spawn a hitscan bullet on the press tick — the throw is
    // a 26-frame animation whose release frame spawns the grenade.
    sim.step_bond(Default::default(), true, false, false);
    let bullets_at_press = sim.drones.player_bullets.len();
    for _ in 0..80 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(bullets_at_press, sim.drones.player_bullets.len(),
        "no hitscan bullet on a grenade's fire press (the press only winds up)");
    assert_eq!(sim.drones.grenades.len(), 1,
        "the grenade spawns at the release frame (fire_anim_len 26 [M 0x1a2e38 case 9; Player_SetFiringAnim 0x1a3c70])");
    let g = &sim.drones.grenades[0];
    assert_eq!(g.weapon_type, 0x35);
    assert!((g.speed - 0.167).abs() < 1e-3, "speed = weapon_data+0x20 [L ee_start_07000001.bin record 0x35]: {}", g.speed);
    assert!(g.range >= 200.0, "the 210-tick fuse [M Bullet_init 0x12c5a0:192-233]: {}", g.range);

    // The state machine returns to idle (the throw script ends → idle script [M 0x1a2e38 case 9 → 0]).
    let idle = sim.weapons.idle_script(0x35);
    assert_eq!(sim.view_script, idle, "after the throw the view script is back to idle {idle:08x}");

    // Switch back to the P2K and fire — "afterwards he can't shoot any weapon" must not happen.
    sim.gun.switch_request = 2;
    tick_idle(sim, 60);
    assert_eq!(sim.gun.weapon, 2);
    assert!(sim.gun.timer <= 0.0, "the refire timer resets on the draw finishing [M 0x1a2e38 case 4: BLData+0x92C = 0] — got {}", sim.gun.timer);
    let before = sim.drones.player_bullets.len();
    sim.step_bond(Default::default(), true, false, false);
    // The impact/bullet lists are drained every tick — capture the press tick's own output.
    let fired_ok = !sim.drones.player_impacts.is_empty() || sim.drones.player_bullets.len() > before;
    for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    assert!(fired_ok, "the P2K fires after the grenade throw (the stuck-timer symptom)");
}

// ── Bug 2: the Raptor drops ─────────────────────────────────────────────────────────────────────

/// Kill Raptor guards (07000001, armed drones) with the P2K, let each drop land, walk onto every drop
/// and log every take test. Every drop must be taken.
#[test]
fn raptor_drops_are_all_taken_when_walked_over() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // The armed, alive, drawn guards the level starts with.
    let guards: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && (2..0x47).contains(&dr.weapon))
        .map(|(i, _)| i).collect();
    eprintln!("[drops] armed guards on 07000001: {guards:?} ({} total drones)", sim.drones.list.len());
    assert!(!guards.is_empty(), "07000001 starts armed guards");
    // Kill up to four of them from a few units away (the P2K: two torso hits at Easy [M Drone_ModBulletDamage]).
    let mut kills = 0;
    for &gi in guards.iter().take(6) {
        let gp = sim.drones.list[gi].pos;
        // Stand 3 units south of the guard on the floor below.
        let floor = sim.world.floor_below(gp, 6.0).unwrap_or(gp[1]);
        let stand = [gp[0], floor + 1.1, gp[2] - 3.0];
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, 0.0);
        aim_at(sim, [gp[0], gp[1] + 0.3, gp[2]]);
        tick_idle(sim, 10);
        let hp0 = sim.drones.list[gi].health;
        // Tap the trigger: the P2K is semi-auto (one shot per pull [M Player_WeaponFiring 0x1a5460]).
        for t in 0..600 {
            sim.step_bond(Default::default(), t % 9 == 0, false, false);
            if sim.drones.list[gi].health <= 0.0 || sim.drones.health.health <= 0.0 { break; }
        }
        if hp0 > 0.0 && sim.drones.list[gi].health <= 0.0 { kills += 1; }
        if sim.drones.health.health <= 0.0 { eprintln!("[drops] Bond died at guard {gi} — stopping"); break; }
        tick_idle(sim, 80); // the drop falls and lands
    }
    eprintln!("[drops] killed {kills} guards; pickups on the ground: {} (of which dropped: {})",
        sim.drones.pickups.len(), sim.drones.pickups.iter().filter(|p| p.from.is_some()).count());
    assert!(kills > 0, "at least one guard died");
    assert!(!sim.drones.pickups.is_empty(), "pickups exist (placed and/or dropped)");
    // Walk onto every pickup — placed ones and guard drops alike (the same `Pickup_Update` take test);
    // log every take test while doing it.
    let n = sim.drones.pickups.len();
    let mut taken_before = sim.drones.taken.len();
    for k in 0..n {
        let Some(p) = sim.drones.pickups.get(k) else { break };
        if p.gone { continue; }
        let at = [p.m[3][0], p.m[3][1] + 0.4, p.m[3][2]];
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
        for t in 0..30 {
            take_test_line(sim, k);
            tick_idle(sim, 1);
            if sim.drones.taken.len() > taken_before { taken_before = sim.drones.taken.len(); break; }
        }
    }
    let missing: Vec<_> = sim.drones.pickups.iter().filter(|p| !p.gone).map(|p| (p.weapon, p.kind, p.rounds, p.from.is_some())).collect();
    // The bug under test is the GUARD DROP (the user's symptom); placed pickups on pedestals/shelves can
    // be legitimately walled off from a teleport (the game takes them by walking the route). Every drop
    // must be taken; the log above shows the take tests for the rest.
    let drops_left: Vec<_> = sim.drones.pickups.iter().filter(|p| !p.gone && p.from.is_some()).map(|p| (p.weapon, p.rounds)).collect();
    assert!(drops_left.is_empty(), "every guard drop is taken; still on the ground: {drops_left:?} (taken = {:?})", sim.drones.taken);
    eprintln!("[drops] still on the ground (placed, not asserted): {missing:?}");
}

// ── Bug 3: the invisible armour ─────────────────────────────────────────────────────────────────

/// The kind-3 (armour) pickup placed in the level must have a drawn instance at its placement matrix
/// (`Pickup_Create` 0x18dda0 links the object into its cel and the renderer draws `obj+0xD8` at `obj+0x90`
/// [M View_DrawObjectGlist 0x1e6780:43-44]). The port drew only drone-dropped guns.
#[test]
fn the_placed_armour_pickup_is_drawn() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    let armours: Vec<usize> = sim.drones.pickups.iter().enumerate()
        .filter(|(_, p)| p.kind == nf_game::pickup::kind::ARMOUR && !p.gone)
        .map(|(i, _)| i).collect();
    assert!(!armours.is_empty(), "07000005 places a kind-3 armour pickup");
    let k = armours[0];
    // The draw pose the renderer uses: the pickup's own matrix while it lies there, None once taken.
    let m = sim.placed_pickup_pose(k).expect("the armour has a drawn instance (View_DrawObjectGlist)");
    let want = sim.drones.pickups[k].m;
    for r in 0..3 { for c in 0..4 { assert!((m[r][c] - want[r][c]).abs() < 1e-4, "pose matches obj+0x90 [{r}][{c}]"); } }
    // Sister: once taken (gone), nothing is drawn.
    sim.drones.pickups[k].gone = true;
    assert!(sim.placed_pickup_pose(k).is_none(), "a taken pickup draws nothing");
}

// ── Bug 4: the civilian crosshair ───────────────────────────────────────────────────────────────

/// `Check_Target` (0x1a7810, every 6th frame): Bond's 50-unit view ray onto a drone whose `+0x44 != 1`
/// (non-enemy: civilians, hostages) sets the crosshair state to 3 — the red "no" mark
/// (`HUD_UpdateCrossHair` 0x19e6d0 reads BLData+0x133; `HUDCrossCoords` state 3 [L 0x2bec48]).
/// Aiming at an enemy (or at nothing) keeps state 2.
#[test]
fn aiming_at_a_non_enemy_shows_the_no_crosshair() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 8);
    let enemy = sim.drones.list.iter().enumerate()
        .find(|(_, dr)| dr.present && dr.enabled && dr.drawn && dr.team == 1 && dr.health > 0.0)
        .map(|(i, _)| i);
    let civilian = sim.drones.list.iter().enumerate()
        .find(|(_, dr)| dr.present && dr.enabled && dr.drawn && dr.team != 1 && dr.health > 0.0)
        .map(|(i, _)| i);
    let Some(ci) = civilian else {
        // 07000001 starts only enemies. The civilian arm needs a non-enemy target: mark the nearest guard
        // non-enemy (a stand-in for a civilian drone — the game's test is the target's BLData+0x44 != 1
        // [M 0x1a7810:136-139], the port's `team`), aim at it, and expect the no-crosshair.
        let Some(ei) = enemy else { eprintln!("no drones at all — skipping"); return };
        let ep = sim.drones.list[ei].pos;
        sim.drones.list[ei].team = 2;
        aim_at(sim, [ep[0], ep[1] + 0.3, ep[2]]);
        for _ in 0..7 { sim.step_bond(Default::default(), false, false, false); }
        assert_eq!(sim.crosshair_state(), 3, "a non-enemy target shows the no-crosshair [M 0x1a7810:181-182]");
        return;
    };
    // Aim at the non-enemy: state 3 (the red "no" crosshair).
    let cp = sim.drones.list[ci].pos;
    aim_at(sim, [cp[0], cp[1] + 0.3, cp[2]]);
    for _ in 0..7 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 3, "a non-enemy target shows the no-crosshair [M 0x1a7810:181-182]");
    // Sister: an enemy (or the sky) is the normal crosshair.
    if let Some(ei) = enemy {
        let ep = sim.drones.list[ei].pos;
        aim_at(sim, [ep[0], ep[1] + 0.3, ep[2]]);
        for _ in 0..7 { sim.step_bond(Default::default(), false, false, false); }
        assert_eq!(sim.crosshair_state(), 2, "an enemy keeps the normal crosshair");
    }
    aim_at(sim, [sim.bond.head()[0], sim.bond.head()[1] + 5.0, sim.bond.head()[2] + 10.0]);
    for _ in 0..7 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 2, "nothing aimed at: the normal crosshair");
}

// ── Bug 5: the sniper rifle ─────────────────────────────────────────────────────────────────────

/// Take the silenced sniper (0x24) through the real pickup path, equip it, press fire: a bullet with
/// weapon 0x24 must leave (the game spawns it on the press tick: fire_anim_len 0 [L ee_start_07000001.bin
/// record 0x24 +0x44]; `Player_WeaponFiring` 0x1a5460:688-691 latches +0x95C and calls WeaponInitBullet).
/// Level note: the take gate is the game's first-person-model check (`Player_EquipWeapon` 0x1a8a58:20,
/// `hashtable_getitem(+0xDC)`); the port stands it in with the level's skin map — 07000001's archive has
/// no 0x05000043 (byte-scan [L]), so the repro runs on 07000005, whose archive has it (both [L]).
#[test]
fn the_sniper_fires_after_pickup() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    let fp = sim.weapons.first_person(0x24);
    eprintln!("[sniper] first-person {fp:08x}; in level skins: {}", sim.skins.contains_key(&fp));
    let id = 0x24usize;
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0, 0.05, 0.0], radius: 0.4 });
    let at = sim.bond.pos;
    let p = nf_game::pickup::Pickup::create_placed(at, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 10, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons)
        .expect("sniper pickup builds");
    sim.drones.pickups.push(p);
    tick_idle(sim, 5);
    assert!(sim.drones.taken.iter().any(|(_, t)| matches!(t, nf_game::pickup::Taken::Weapon { weapon: 0x24 })),
        "the sniper pickup is taken (taken = {:?})", sim.drones.taken);
    assert!(sim.drones.inventory.owned[0x24], "0x24 owned; clip {}", sim.drones.inventory.clip[0x24]);
    sim.gun.switch_request = 0x24;
    tick_idle(sim, 60);
    assert_eq!(sim.gun.weapon, 0x24, "the sniper is equipped");
    assert!(sim.gun.clip >= 1, "the clip holds the pickup's rounds: {}", sim.gun.clip);
    let before = sim.drones.player_bullets.len();
    // Tap the trigger for a few pulls (the draw may still be finishing when the first press lands); the
    // impact/bullet lists are drained every tick — capture the tick a press lands.
    let mut fired_ok = false;
    for t in 0..30 {
        sim.step_bond(Default::default(), t % 10 == 0, false, false);
        if t % 10 == 0 && (!sim.drones.player_impacts.is_empty() || sim.drones.player_bullets.iter().any(|b| b.weapon == 0x24) || sim.drones.player_bullets.len() > before) { fired_ok = true; }
    }
    assert!(fired_ok, "the sniper fires after pickup (bullets {:?})", sim.drones.player_bullets.iter().map(|b| b.weapon).collect::<Vec<_>>());
}
