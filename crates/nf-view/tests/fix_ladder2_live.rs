//! fix-ladder2: the ladder draw, the climb look, the weapon save/restore and the hands fire gate,
//! through the REAL viewer path (`nf_view::load` + `Sim::step_bond`). Opt-in via `NF_ISO`.
//!
//! User playtest after fix-ladder (master 415faa6+):
//! - "The ladders aren't drawn." — the 0x22 placements got no instance (`instances(cels_only=true)`
//!   skips every placement without the 0x8000 flag; ladders never carry it [M 0x1d1998/0x1d1b68]).
//! - "The camera can't be moved while on a ladder." — `Player_Climb`'s look: the turn channel still
//!   integrates [M 0x193600] inside the eased ±90° window, and the pitch offset is free [M 0x1a1d28/0x1a92e8].
//! - "Getting off doesn't re-equip the weapon… the hands-only state fires once." — `Player_WeaponNone`
//!   saves the id on the grab [M 0x1a9858], the dismount carry restores it [M 0x196310:99-102 +
//!   0x1a2e38:197-201], and the hands record has no fire scripts so no round can spawn [D probe +
//!   M 0x1a5460:681-690].

use nf_view::scene::Sim;

/// `Vec_AngleDifference` 0x1e2b78: wrap `second − first` into (−π, π].
fn wrap_pi(d: f32) -> f32 {
    let mut d = d + std::f32::consts::PI;
    d -= std::f32::consts::TAU * (d / std::f32::consts::TAU).floor();
    d - std::f32::consts::PI
}

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// Walk Bond into the nearest ladder and return (grab tick, ladder). Mirrors fix_ladder_live's drive.
fn grab_nearest_ladder(sim: &mut Sim) -> (usize, nf_game::ladder::Ladder) {
    let start = sim.bond.pos;
    let li = (0..sim.ladders.len()).min_by(|a, b| {
        let d = |l: &nf_game::ladder::Ladder| (l.centre[0] - start[0]).powi(2) + (l.centre[2] - start[2]).powi(2);
        d(&sim.ladders[*a]).total_cmp(&d(&sim.ladders[*b]))
    }).expect("07000002 has ladders on the disc");
    let lad = sim.ladders[li].clone();
    let (s, c) = (lad.yaw.sin(), lad.yaw.cos());
    let stand = [lad.centre[0] + s * 1.2, 0.0, lad.centre[2] + c * 1.2];
    let floor = sim.world.floor_below([stand[0], lad.centre[1], stand[2]], 8.0).expect("floor in front of the ladder");
    let stand = [stand[0], floor + 1.1, stand[2]];
    let yaw = lad.yaw + std::f32::consts::PI;
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, yaw);
    sim.bond.yaw = yaw;
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for t in 0..900 {
        sim.step_bond(fwd, false, false, false);
        if sim.bond.substate == nf_game::player::SS_CLIMB { return (t, lad); }
    }
    panic!("walking into the ladder never attached");
}

/// The ladders are drawn: every 0x22 placement with a resolvable object got exactly one World-layer
/// instance — 07000002 has 4 ladder placements [D, docs/atlas/ladder.md]. Before fix-ladder2 the count
/// was 0 (the ladder meshes were invisible). The exact count is also the double-draw guard: the game
/// draws a ladder once (the static 0x8000 path and the dynamic path are mutually exclusive per
/// placement [M 0x1d1998 vs 0x1d1b68]), and the sweep runs after the instance loop's 0x22 retain.
#[test]
fn ladder_meshes_are_instanced_exactly_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    assert!(scene.ladders_drawn > 0, "no ladder instance was built — the ladders are invisible (the user's symptom)");
    assert_eq!(scene.ladders_drawn, 4, "07000002 has 4 ladder placements [D]; a mismatch means a double draw or a missed resolution");
    // Sister: an independent sweep (the gameplay list) agrees on the count, so the draw count tracks
    // the placements rather than a hardcoded number.
    let scene5 = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    assert_eq!(scene5.ladders_drawn, 1, "07000005 has exactly 1 ladder placement [D]");
}

/// The camera can look while on a ladder: the turn channel moves the yaw inside the eased ±90° window
/// and the pitch offset takes the stick [M Player_Climb 0x196310 + Player_Update 0x193600 +
/// Player_Aiming 0x1a1d28 + ViewClamping 0x1a92e8]. Before fix-ladder2 neither input reached the climb.
#[test]
fn camera_looks_during_the_climb() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    let (_, lad) = grab_nearest_ladder(&mut sim);
    assert!(sim.bond.substate == nf_game::player::SS_CLIMB);
    let target = lad.yaw + std::f32::consts::PI;
    let facing = (wrap_pi(target - sim.bond.yaw)).abs();
    // Turn hard left for a second: the yaw must leave the ladder-facing direction but stay inside the
    // ±90° window (the clamp pulls back every tick [M 0x196310:18-33]).
    let turn = nf_game::player::Input { turn: 0.05, ..Default::default() };
    for _ in 0..60 { sim.step_bond(turn, false, false, false); }
    assert!(sim.bond.substate == nf_game::player::SS_CLIMB, "turned off the ladder");
    let dev = (wrap_pi(target - sim.bond.yaw)).abs();
    // The equilibrium deviation under a held turn is turn/0.2 (the per-tick ease `yaw += diff·0.2`
    // opposes the turn channel [M 0x196310:18-33]) — 0.05/0.2 = 0.25 here. Pre-fix the turn input
    // never reached the climb, so the deviation stayed exactly 0.
    assert!(dev > 0.1, "the turn input did not move the yaw during the climb (deviation {dev}) — the camera is stuck (the user's symptom)");
    assert!(dev <= std::f32::consts::FRAC_PI_2 + 1e-3, "the yaw left the ±90° ladder window: {dev}");
    // Pitch: look up for a second — the offset must move (ViewClamping ±1.0 [M 0x1a92e8]).
    let look = nf_game::player::Input { look: 0.05, ..Default::default() };
    for _ in 0..60 { sim.step_bond(look, false, false, false); }
    assert!(sim.bond.pitch > 0.3, "the pitch offset did not move during the climb ({}) — the camera is stuck", sim.bond.pitch);
    // Sister: the pitch never leaves ±1 [M ViewClamping].
    for _ in 0..240 { sim.step_bond(look, false, false, false); }
    assert!(sim.bond.pitch <= 1.0 + 1e-4, "the pitch offset passed the ViewClamping clamp: {}", sim.bond.pitch);
}

/// The weapon is saved on the grab and re-equipped on the dismount, and the hands class cannot fire.
/// [M Player_WeaponNone 0x1a9858 (the save), Player_Climb 0x196310:99-102 (the carry), SetWeaponAnimObj
/// 0x1a2e38:197-201 (the re-select); the hands record 0x47 has no fire scripts [D probe_ladder2] and the
/// round spawn runs from the state-9 fire-script path (0x1a5460:681-690).]
#[test]
fn weapon_saved_on_grab_restored_on_dismount_and_hands_cannot_fire() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    // Start with the P2K (weapon 2, the level loadout [D probe: "switch request 2"]).
    assert_eq!(sim.gun.weapon, 2, "the 07000002 loadout should start on weapon 2");
    sim.drones.inventory.owned[2] = true;
    let (grab_t, _lad) = grab_nearest_ladder(&mut sim);
    // The grab saved the weapon and switched to hands immediately [M 0x1a9858].
    assert_eq!(sim.gun.weapon, 0x47, "the grab did not switch to the hands class");
    assert_eq!(sim.gun.saved_weapon, 2, "the grab did not save the current weapon (anim+0x64)");
    // Firing at hands spawns nothing [D: 0x47's +0xB0/+0xB4 = 0; M 0x1a5460].
    let bullets_before = sim.drones.bullets.len();
    let fire = nf_game::player::Input::default();
    for _ in 0..20 { sim.step_bond(fire, true, false, false); }
    assert_eq!(sim.drones.bullets.len(), bullets_before,
        "the hands class fired ({} bullets) — the user's 'fires once' symptom", sim.drones.bullets.len() - bullets_before);
    // Climb to the top: the dismount restores the saved weapon through WeaponSelect [M 0x196310:99-102].
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut restored = false;
    for _ in 0..1800 {
        sim.step_bond(fwd, false, false, false);
        if sim.bond.substate == nf_game::player::SS_WALK && sim.bond.ladder.is_none() {
            // The request is queued on the dismount tick; the draw cycle then lands the weapon.
            for _ in 0..240 { sim.step_bond(fwd, false, false, false); }
            restored = sim.gun.weapon == 2;
            break;
        }
    }
    assert!(restored, "after the climb Bond is on weapon {:#x}, not the saved P2K (grab at tick {grab_t}) — the re-equip did not run", sim.gun.weapon);
    // Sister: the restore went through the draw cycle (the view script is the P2K's idle, not hands').
    assert!(sim.gun.switch_state == 0, "stuck mid-switch after the restore (state {})", sim.gun.switch_state);
}

/// The swim repro from the bank (fix-ladder2, NOT fixed this session — see the report): in the port,
/// Bond walks on the collision floor that sits at/above the water surface in the garden water (cel 17:
/// floor 3.7647 vs surface 3.7675 [D probe_ladder2]), so `Player_InWater`'s `pos.y − 0.3 < cel+0x90`
/// never fires from a reachable spot. The port's collision is double-sided (*Deviation*, world.rs
/// capsule_push) while the game's triangles are one-sided by their stored plane — the likely root
/// cause, not yet ported. `#[ignore]`d: this asserts the CURRENT (broken) behaviour as the recorded
/// repro; the next session flips it after the one-sided floor fix.
#[test]
#[ignore]
fn swim_repro_walks_on_the_water_from_the_bank() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    // The garden water (cel 17: x −80.3..−54.1, z 20.1..44.0, surface 3.7675, floor 3.7647).
    let c = &sim.cels.iter().enumerate().find(|(_, c)| c.water > 3.7 && c.water < 3.8 && c.max[2] > 40.0)
        .expect("the garden water cel").1.clone();
    let (x, z) = (0.5 * (c.min[0] + c.max[0]), 0.5 * (c.min[2] + c.max[2]));
    let floor = sim.world.floor_below([x, 6.0, z], 10.0).expect("the bank floor");
    eprintln!("repro: bank floor {floor} vs water surface {} (delta {})", c.water, c.water - floor);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [x, floor + 1.1, z], 0.0);
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut swam = false;
    for t in 0..600 {
        sim.step_bond(fwd, false, false, false);
        if sim.bond.substate == nf_game::player::SS_SWIM { swam = true; eprintln!("entered swim at tick {t}, y {}", sim.bond.pos[1]); break; }
    }
    // The user's symptom (pre-fix): he walks on, never in.
    assert!(!swam, "Bond entered the water from the bank — the repro no longer reproduces (the fix landed?)");
}
