//! civilian-no-shoot — the red no-shoot crosshair over non-enemies [M `Check_Target`
//! 0x1a7810], what it does and does NOT gate. The playtest premise ("the game refuses to
//! fire") is contradicted by the disc: the repo's own PCSX2 trace of this exact play
//! (tests/traces/02-civilian.trace, 2026-10-11) shows the P2K clip falling on EVERY trigger
//! pull under the red mark [L], and the whole fire pipeline reads nothing the mark writes —
//! BLData+0x133's only reader is the HUD draw [M `HUD_UpdateCrossHair` 0x19e6d0:69], and
//! BLData+0x880 (the aimed object) is read only by the gadget/use weapon types 0x56-0x58
//! [M `Player_WeaponFiring` 0x1a5460:531-581]; the damage path has no team gate either
//! [M `NDrone2_DealWithObjHit` 0x13fff0 case 5 — a class-3 (player) owner hits any team].
//! What the game DOES gate: (a) the icon itself — keyed on the held weapon id
//! [M 0x1a7810:152-183: ids {1, 0x35, 0x36, 0x43, 0x44} — the fists and the throw family —
//! keep the NORMAL crosshair; levels 0x7000005..0x7000008 never mark]; (b) the auto-aim lock
//! [M `Check_AutoAim` 0x1a73f8:83-87 — team != 1 → no lock; already ported, pinned here on
//! 07000002's data]; (c) the mission fail on killing the civilian [M `DroneFunc_
//! SetMissionFailReason` 0x13e560 → "Killed Civilians"; already ported]. Opt-in via NF_ISO.

use nf_game::player::Input;

fn load_and_skip() -> Option<nf_view::scene::Scene> {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return None };
    let mut scene = match nf_view::load(&iso, "07000002.bin", false) {
        Ok(s) => s,
        Err(e) => { eprintln!("load: {e}"); return None }
    };
    let mut n = 0u32;
    let sim = scene.sim.as_mut().unwrap();
    while n < 400 {
        if sim.drones.scripts.players.iter().any(|p| p.state == 2 && p.scripts.contains(&0x0600_0084)) {
            sim.drones.scripts.skip_request = true; // the game's skip action [M SP_Update 0x1dbda8]
        }
        sim.step_bond(Input::default(), false, false, false);
        n += 1;
    }
    drop(sim);
    Some(scene)
}

/// The civilian beside Bond's start: kind 9, team 2, drawn, ~3.5 units ahead [D 07000002's
/// placement; the playtest's target]. Aims Bond's eye at the target's chest.
const CIVILIAN: usize = 21;
const GUARD: usize = 0;

fn aim_at(sim: &mut nf_view::scene::Sim, target: [f32; 3]) {
    let head = sim.bond.head();
    let d = [target[0] - head[0], target[1] - head[1], target[2] - head[2]];
    let horiz = (d[0] * d[0] + d[2] * d[2]).sqrt();
    sim.bond.yaw = d[0].atan2(d[2]); // look = [cos(p)·sin(yaw), sin(p), cos(p)·cos(yaw)]
    sim.bond.pitch = (d[1].atan2(horiz)) / (std::f32::consts::FRAC_PI_2);
}

/// Semi-auto fires once per press [M the fire action's mask-4 edge]: `presses` trigger pulls
/// with the release between them (the game's pad rhythm).
fn burst(sim: &mut nf_view::scene::Sim, presses: usize) -> bool {
    let mut fired = false;
    for _ in 0..presses {
        for _ in 0..3 { sim.step_bond(Input::default(), true, false, false); if sim.shot_tick { fired = true; } }
        for _ in 0..7 { sim.step_bond(Input::default(), false, false, false); }
    }
    fired
}

/// The intro's end leaves Bond beside the civilian with the start gun: the no-shoot icon keys
/// on the target's team AND the held weapon [M 0x1a7810:152-183] — the gun marks (state 3),
/// the fists (id 1) keep the normal crosshair (state 2) over the SAME civilian, aiming away
/// returns to 2. (Pre-fix the port marked every weapon: the fists arm failed.)
#[test]
fn the_no_shoot_crosshair_keys_on_the_held_weapon() {
    let Some(mut scene) = load_and_skip() else { return };
    let sim = scene.sim.as_ref().unwrap();
    let civ = sim.drones.list[CIVILIAN].pos;
    let weapon = sim.gun.weapon;
    assert!(sim.drones.list[CIVILIAN].team != 1, "the intro civilian is a non-enemy (team 2)");
    assert!(sim.drones.list[CIVILIAN].drawn && sim.drones.list[CIVILIAN].enabled, "the civilian is live after the skip");
    assert!(matches!(weapon, 2..=0x34), "the start gun {} is a firearm (not the fists/throw family)", weapon);
    let mut sim = scene.sim.as_mut().unwrap();
    aim_at(&mut sim, [civ[0], civ[1] + 0.4, civ[2]]);
    for _ in 0..8 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 3, "the gun over the civilian shows the red no-shoot mark");
    // Aim off: the normal crosshair.
    sim.bond.yaw += 0.6;
    for _ in 0..8 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 2, "aiming off the civilian returns to the normal crosshair");
    // The fists: id 1 keeps the NORMAL crosshair over the same civilian [M 0x1a7810's class
    // table: cVar1 == 1 → bVar7 = false → no mark].
    sim.bond.yaw -= 0.6; // back on the civilian
    aim_at(&mut sim, [civ[0], civ[1] + 0.4, civ[2]]);
    let input = Input { switch_to: Some(1), ..Input::default() };
    for _ in 0..60 { sim.step_bond(input, false, false, false); }
    assert_eq!(sim.gun.weapon, 1, "the fists are drawn through the real switch path");
    for _ in 0..8 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 2, "the fists over the civilian keep the NORMAL crosshair (you may punch)");
}

/// The trigger fires UNDER the red mark exactly like the traced game [L
/// tests/traces/02-civilian.trace: the clip fell on every pull]: the shot spends ammo, the
/// bullet lands on the civilian and the damage applies (the port's pipeline, like
/// `NDrone2_DealWithObjHit` 0x13fff0 case 5's owner-class-3 arm, has no team gate). A held
/// trigger keeps firing while the aim stays on him; the moment the aim leaves, the crosshair
/// clears and the next shot flies where Bond points.
#[test]
fn the_trigger_fires_under_the_mark_and_the_civilian_takes_the_hit() {
    let Some(mut scene) = load_and_skip() else { return };
    let civ = scene.sim.as_ref().unwrap().drones.list[CIVILIAN].pos;
    let health0 = scene.sim.as_ref().unwrap().drones.list[CIVILIAN].health;
    let mut sim = scene.sim.as_mut().unwrap();
    aim_at(&mut sim, [civ[0], civ[1] + 0.4, civ[2]]);
    for _ in 0..8 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 3, "the mark is up before the shots");
    let clip0 = sim.drones.inventory.clip[sim.gun.weapon as usize];
    let fired = burst(&mut sim, 8);
    assert!(fired, "the trigger fires under the no-shoot mark (the game's traced behavior)");
    let clip1 = sim.drones.inventory.clip[sim.gun.weapon as usize];
    assert!(clip1 < clip0, "the shots spend ammo (clip {} → {}) — the game does not gate the trigger", clip0, clip1);
    let health1 = sim.drones.list[CIVILIAN].health;
    assert!(health1 < health0, "the civilian takes the hit (health {} → {})", health0, health1);
    // The civilian died → the mission fails with "Killed Civilians" [M 0x13e560 reason 1 →
    // Mission_SetFailLabel(0x4000034), the fail channel 0x60] — the game's real protection.
    assert!(health1 <= 0.0, "the P2K kills the 10-health civilian");
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0034, "Killed Civilians [M 0x13e560]");
    // Aim off the body: the crosshair clears and a shot flies (the aim-leaves half of the task's
    // question — nothing latches).
    sim.bond.yaw += 0.9;
    let clip2 = sim.drones.inventory.clip[sim.gun.weapon as usize];
    burst(&mut sim, 2);
    assert_eq!(sim.crosshair_state(), 2, "the mark clears when the aim leaves");
    assert!(sim.drones.inventory.clip[sim.gun.weapon as usize] < clip2 || clip2 == 0,
        "a shot flies once the aim is off (or the gun ran dry)");
}

/// The sister: a GUARD beside the civilian is shootable — the normal crosshair, the lock, the
/// damage. And the auto-aim never locks the civilian [M Check_AutoAim 0x1a73f8:83-87 — team
/// != 1 → skip], while it does take the guard.
#[test]
fn the_guard_beside_the_civilian_is_shootable_and_the_lock_skips_the_civilian() {
    let Some(mut scene) = load_and_skip() else { return };
    let civ = scene.sim.as_ref().unwrap().drones.list[CIVILIAN].pos;
    {
        let sim = scene.sim.as_mut().unwrap();
        // The nearest team-1 guard beside the civilian (a test stand-in for the level's own
        // alert placements — the guards enable as Bond approaches [D 07000002's drones 0..20]).
        let g = &mut sim.drones.list[GUARD];
        g.pos = [civ[0] + 1.2, civ[1], civ[2] - 1.2];
        g.enabled = true;
        g.drawn = true;
        g.health = 40.0; // survives the burst so the lock arm can reuse him
    }
    let guard = scene.sim.as_ref().unwrap().drones.list[GUARD].pos;
    let health0 = scene.sim.as_ref().unwrap().drones.list[GUARD].health;
    let mut sim = scene.sim.as_mut().unwrap();
    aim_at(&mut sim, [guard[0], guard[1] + 0.4, guard[2]]);
    for _ in 0..8 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.crosshair_state(), 2, "a guard under the crosshair is a normal (shootable) target");
    let clip0 = sim.drones.inventory.clip[sim.gun.weapon as usize];
    burst(&mut sim, 2);
    assert!(sim.drones.list[GUARD].health < health0 && sim.drones.list[GUARD].health > 0.0,
        "the guard takes the shot and lives on (health {} → {})", health0, sim.drones.list[GUARD].health);
    assert!(sim.drones.inventory.clip[sim.gun.weapon as usize] < clip0 || clip0 == 0, "the guard shot spends ammo");
    // The auto-aim lock: on the guard's aim it takes him; on the civilian's aim it never locks.
    // (The game's option default is ON — PlayerSetting+1 = 1 [L ee_start_02]; the test env boots
    // it off, so the test sets it the way the game's options screen would. The lock scan runs on
    // the fired shot's direction [M Player_WeaponInitBullet 0x1a4bf0's aim-point ray], so the
    // lock arms fire.) The chest point is the drone's bone-2 palette position the lock measures.
    sim.auto_aim = true;
    sim.drones.difficulty = 2; // Normal: the auto-aim windows are live (Hard's multiplier is 0)
    aim_at(&mut sim, [guard[0], guard[1] + 0.6, guard[2]]);
    burst(&mut sim, 2);
    let guard_locked = sim.drones.aim_target == Some(GUARD);
    aim_at(&mut sim, [civ[0], civ[1] + 0.4, civ[2]]);
    burst(&mut sim, 2);
    assert!(guard_locked, "the lock takes the guard (aim_target was {:?})", sim.drones.aim_target);
    assert_ne!(sim.drones.aim_target, Some(CIVILIAN),
        "the auto-aim never locks the civilian [M Check_AutoAim 0x1a73f8:83-87]");
}
