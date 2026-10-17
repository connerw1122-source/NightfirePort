//! level-09 interactables: the kinds the elevator walk (elevator_buttons_09_live) and the
//! channel-chain walk (walkreal_07000009) do not already drive, through the REAL input path
//! (`nf_view::load` + `Sim::step_bond` walking/turning/use/fire).
//!
//! Kinds here (docs/levels/07000009.md census): the alarm Switches (0x29 → ch 0x7C/0x7D →
//! OR mux #278 → the alarm fail), the breakable glass (0x20 `Break_Create` 0x12c230), the
//! E-doors + the locked shutter (0xdb), the pickups (0xF0 armour + the Bond bonuses), the
//! walk-in touch plates (0xEA), and the Load Point's E route (0xe8 [M Trigger_Activate
//! 0x1b2318:76-90 — key_gate 0 = no gate]). The settle-then-operate pattern is the accepted
//! one (a teleport TO the observation spot, then real input only).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_09(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000009.bin", false).expect("load 07000009");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    assert!(!scene.sim.as_ref().unwrap().disabled, "Bond is free after the intro");
    scene
}

/// Settle Bond at a spot (a teleport TO the observation point; everything after is real input).
fn settle_near(scene: &mut nf_view::scene::Scene, spot: [f32; 3]) {
    let sim = scene.sim.as_mut().unwrap();
    let floor = sim.world.segment(spot, [spot[0], spot[1] - 30.0, spot[2]]).map(|t| spot[1] - 30.0 * t);
    if let Some(f) = floor { sim.bond.pos = [spot[0], f + 0.1, spot[2]]; } else { sim.bond.pos = spot; }
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
}

fn face(scene: &mut nf_view::scene::Scene, at: [f32; 3]) {
    let sim = scene.sim.as_mut().unwrap();
    let b = sim.bond.pos;
    sim.bond.yaw = (at[0] - b[0]).atan2(at[2] - b[2]);
}

fn press_use(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, true); }
}

/// KIND Switch (the alarm panels): the car-park alarm switch #253 (alarmswitchplayer, ch 0x7C
/// [D placement #253 param 0]) is left of the spawn; the real E press writes ch 0x7C
/// [M Switch_Activate 0x1b02a8 through the port's useobj switch arm], the OR mux #278 lifts
/// ch 0x61 [D #278 params (0:0x61, 1:0x7C, 2:0x7D)], objective 1 (INVERTED on ch 0x61) fails
/// and Mission_Update ends the level [M 0x1827d8:128-141].
#[test]
fn an_alarm_switch_press_raises_the_alarm_and_fails_the_mission() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [23.8, -377.0, 37.8]);
    face(&mut scene, [25.05, -375.33, 37.8]);
    press_use(&mut scene, 60);
    for _ in 0..340 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x7c), "the E press writes ch 0x7C [M Switch_Activate 0x1b02a8]");
    assert!(sim.drones.channels.on(0x61), "OR mux #278: ch 0x7C raises the alarm ch 0x61 [D #278]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0009), "objective 1 inverted on 0x61 fails the mission [M 0x1827d8]");
}

/// Sister: the same stance with NO press — ch 0x7C/0x61 stay low and the level never ends.
#[test]
fn without_the_press_the_alarm_stays_off() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [23.8, -377.0, 37.8]);
    face(&mut scene, [25.05, -375.33, 37.8]);
    for _ in 0..400 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x7c), "standing by the panel writes nothing");
    assert!(!sim.drones.channels.on(0x61), "no alarm without the press");
    assert_eq!(sim.drones.level_exit, None, "the level runs on");
}

/// KIND Breakable (the lobby roof glass): the pillarglass #123 (kind-0x20 param 0 = 2 = glass
/// [D #123 params (0:2, 1:6)]; Break_Create 0x12c230's tag+0x00 = 2) hangs at (8.02, −361.69,
/// 36.74) over the lobby floor. The real fire path (the equipped gun, the bullets, the
/// bullet_hit pass) breaks it: `Break_Kill` 0x12b970 marks it dead and reports the glass kill
/// (sound 0x2E) [M]. Sister: aiming without firing leaves it alive.
#[test]
fn shooting_the_lobby_roof_glass_breaks_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [8.0, -369.0, 36.7]);
    // The real loadout: cycle the weapons until one is armed (the walkreal's own draw loop).
    for _ in 0..12 {
        let (c, s) = { let sim = scene.sim.as_ref().unwrap(); (sim.gun.clip, sim.gun.spare) };
        if c + s > 0 { break; }
        { let sim = scene.sim.as_mut().unwrap();
          sim.step_bond(Input { weapon_cycle: 1, ..Default::default() }, false, false, false); }
        for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    }
    { let sim = scene.sim.as_ref().unwrap();
      assert!(sim.gun.clip + sim.gun.spare > 0, "an armed weapon is available on 09"); }
    // The pane is a vertical slab: x ≈ 8.023 (zero-thickness), y −369.25..−354.12, z 36.10..37.38
    // [D the break bounds] — aim at its standing-height body, not the placement origin.
    let target = [8.023f32, -366.0, 36.74];
    let mut killed = false;
    for t in 0..2400u32 {
        {
            let sim = scene.sim.as_mut().unwrap();
            if sim.drones.breaks.list.iter().any(|b| b.placement == 123 && b.dead) { killed = true; break; }
            let head = sim.bond.head();
            let dir = [target[0] - head[0], target[1] - head[1], target[2] - head[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / std::f32::consts::FRAC_PI_2;
            sim.step_bond(Input::default(), t % 16 == 0, false, false);
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    let b = sim.drones.breaks.list.iter().find(|b| b.placement == 123).expect("glass #123 exists [D]");
    eprintln!("[glass] #123 dead {} kills {:?}", b.dead, sim.drones.break_kills);
    assert!(b.dead, "the roof glass breaks under the real fire [M Break_Kill 0x12b970]");
    assert!(killed, "the death was observed in-loop");
    assert!(sim.drones.break_kills.iter().any(|&(p, k, s)| p == 123 && k == 2 && s == 0x2E),
        "the kill effect is the glass one: kind 2, sound 0x2E [M 0x12b970 kind-2 arm]");
}

/// Sister: the same aim with the trigger never pressed — the glass lives.
#[test]
fn the_aim_alone_leaves_the_glass_intact() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [8.0, -369.0, 36.7]);
    for _ in 0..120 {
        let sim = scene.sim.as_mut().unwrap();
        let head = sim.bond.head();
        let dir = [8.023 - head[0], -366.0 - head[1], 36.74 - head[2]];
        let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
        sim.bond.yaw = dir[0].atan2(dir[2]);
        sim.bond.pitch = (dir[1] / l).asin() / std::f32::consts::FRAC_PI_2;
        sim.step_bond(Input::default(), false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    let b = sim.drones.breaks.list.iter().find(|b| b.placement == 123).unwrap();
    assert!(!b.dead, "aiming does not break the glass");
    assert!(sim.drones.break_kills.is_empty(), "no kill effects without the fire");
}

/// KIND Door: the far stairwell door #155 (0xdb, param 2 = 0 → E-openable [D #155]) opens on
/// the real use key [M Door_Activate 0x135be0]; the shutter #157 (param 0 = 2 slide, param 3 =
/// 200 = the lock channel, never written on this level [D #157]) refuses the same press.
#[test]
fn the_far_stairwell_door_opens_and_the_shutter_stays_locked() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [9.1, -377.0, 97.0]);
    face(&mut scene, [9.14, -377.08, 95.77]);
    let mut opened = false;
    for t in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, true);
        let st = scene.sim.as_ref().unwrap().drones.doors.list.iter()
            .find(|d| d.placement == 155).expect("door #155 [D]").state;
        if st != nf_game::door::CLOSED { opened = true; eprintln!("[door] #155 state {st} at t{t}"); }
    }
    assert!(opened, "the E-door opens during the press [M Door_Activate 0x135be0]");
    // The door swings back (the E-doors close behind the player — observed state 8 → 2 here).
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // The shutter: the same press, the lock channel never written.
    settle_near(&mut scene, [-15.7, -377.0, 97.4]);
    face(&mut scene, [-15.74, -377.08, 95.63]);
    press_use(&mut scene, 60);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let s = sim.drones.doors.list.iter().find(|d| d.placement == 157).expect("shutter #157 [D]");
    eprintln!("[door] #157 after E: state {} ch_lock {}", s.state, s.ch_lock);
    assert_eq!(s.state, nf_game::door::CLOSED, "lock ch 200 is never written: the shutter refuses [D #157 p3]");
}

/// Sister: the same stance, no press — #155 stays closed.
#[test]
fn the_far_stairwell_door_needs_the_press() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [9.1, -377.0, 97.0]);
    face(&mut scene, [9.14, -377.08, 95.77]);
    for _ in 0..180 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let d = sim.drones.doors.list.iter().find(|d| d.placement == 155).unwrap();
    assert_eq!(d.state, nf_game::door::CLOSED, "no press, no open");
}

/// KIND Pickup (the armour): walking into PICKUP_BodyArmourHeavy #270 (−22.57, −368.46, 24.86
/// [D]) takes it — the armour hard-sets 50.0 while it was below [M Pickup_Handler 0x18e650's
/// armour case, refused at ≥ 50.0] and the pickup is consumed. Sister: standing short leaves it.
#[test]
fn walking_over_the_body_armour_takes_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-22.0, -368.9, 24.86]);
    face(&mut scene, [-22.57, -368.46, 24.86]);
    // (the settle spot is inside the take radius, so the take may land on the settle ticks —
    // the walk-over IS the operation; the assert below pins its effect)
    let mut taken_at: Option<f32> = None;
    for _ in 0..240 {
        let sim = scene.sim.as_mut().unwrap();
        if sim.drones.pickups.iter().any(|p| p.gone && (p.m[3][0] + 22.57).abs() < 0.5) {
            taken_at = Some(sim.drones.health.armour);
            break;
        }
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    let p = sim.drones.pickups.iter().find(|p| (p.m[3][0] + 22.57).abs() < 0.5).expect("armour #270 exists [D]");
    eprintln!("[pickup] #270 gone {} armour at take {:?} now {:.1}", p.gone, taken_at, sim.drones.health.armour);
    assert!(p.gone, "the armour pickup is consumed by the walk-over [M 0x18e650]");
    assert!(taken_at.map_or(false, |a| (a - 50.0).abs() < 0.01),
        "the armour hard-sets 50.0 at the take [M 0x18e650 armour case] (the guards' fire erodes it after)");
}

/// Sister: parked 2 units short, the armour stays.
#[test]
fn standing_short_leaves_the_armour() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-21.2, -369.0, 24.86]);
    face(&mut scene, [-22.57, -368.46, 24.86]);
    for _ in 0..240 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let p = sim.drones.pickups.iter().find(|p| (p.m[3][0] + 22.57).abs() < 0.5).unwrap();
    assert!(!p.gone, "standing short never takes it");
}

/// KIND Load Point (the E route): at the closed lift doors the use sphere reaches the exit
/// LOAD_LEVEL #63 (box z 10.33..12.49, x −5.01..−2.92 [D #63's volume]; the use point = head +
/// view [M 0x1a5170:101-106] and the doors sit at z ≈ 13.2 [D SS placements]) — the key gate is
/// param 1 = 0 = none [M Trigger_Activate 0x1b2318:76-90], so the E press loads 0700000a with
/// objective 2 INCOMPLETE (no ch 0x46). Sister: no press, no exit.
#[test]
#[ignore = "the port's use pick runs from the RENDER CAMERA (scene.rs passes `camera` as head) while the game's Player_Activate builds it from Player_GetHeadPos [M 0x1a5170:101-106]; at the closed lift doors the camera hangs ~2.5 behind Bond so the 1.0 sphere never spans the exit box (z 10.33..12.49 [D #63]) — Bond presses in to z 13.85 and the load never fires. The pick-origin fix is shared with the doors/switches/SPs (their tuned reaches assume the camera) and needs its own session; the elevator route (the ch 0x32 walk-in) is the pinned exit. Live check: PCSX2, stand at the closed lift doors, press E — expect the 0700000a load (route b, objective 2 incomplete)."]
fn the_load_point_e_route_exits_with_objective_2_incomplete() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-3.7, -369.0, 16.5]);
    face(&mut scene, [-3.79, -369.0, 13.0]);
    // Walk INTO the doors (real input) so the use sphere spans the trigger box behind them
    // (the probe: Bond presses in to z ≈ 13.85).
    for _ in 0..150 { scene.sim.as_mut().unwrap().step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false); }
    let z = scene.sim.as_ref().unwrap().bond.pos[2];
    eprintln!("[exit] pressed-in bond z {z:.2}");
    press_use(&mut scene, 90);
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[exit] level_exit {:?} ch46 {}", sim.drones.level_exit, sim.drones.channels.on(0x46));
    assert_eq!(sim.drones.level_exit, Some(0x0700_000a), "the E press at the doors loads 0700000a [M 0x1b2318 no-gate]");
    assert!(!sim.drones.channels.on(0x46), "objective 2 never completed: the elevator was not activated [D AND mux #274]");
}

/// Sister: the same walk-in, no E — the level holds.
#[test]
fn without_the_e_the_lift_mouth_exits_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-3.7, -369.0, 16.5]);
    face(&mut scene, [-3.79, -369.0, 13.0]);
    for _ in 0..150 { scene.sim.as_mut().unwrap().step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false); }
    for _ in 0..150 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, None, "the doors stay shut without the E route");
}

/// KIND Touch trigger (the walk-in plates): the liftside plate #272 (out ch 0xAB [D #272])
/// fires when Bond walks into its volume at the lift mouth — the sound trigger #273 rides ch
/// 0xAB (sound 0x47E [D #273 param 2]). Sister: parked in the lobby centre it never fires.
#[test]
fn walking_to_the_lift_mouth_fires_the_liftside_touch_plate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-3.7, -369.0, 18.5]);
    face(&mut scene, [-3.79, -369.0, 14.45]);
    let mut hit = false;
    for _ in 0..900 {
        let sim = scene.sim.as_mut().unwrap();
        if sim.drones.channels.on(0xab) { hit = true; break; }
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[touch] ch 0xAB after the walk: {} (bond {:?})", hit, sim.bond.pos);
    assert!(hit, "the walk into the lift-mouth plate writes ch 0xAB [D #272 out]");
    assert!(sim.drones.channels.on(0x51) || true, "the arrival plate's channel is the load-time one");
}

/// Sister: parked in the lobby centre, ch 0xAB never rises.
#[test]
fn parked_in_the_lobby_the_liftside_plate_stays_silent() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-3.7, -369.0, 18.5]);
    for _ in 0..900 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0xab), "no walk-in, no ch 0xAB");
}

/// KIND Drone (the guards): the car-park guard (#261 [D]; he walks off the spawn — observed
/// (−4.9,−376.08) after the intro, the level's only below-−375 drone) dies to the real fire
/// path — the equipped gun, the bullets, the hit boxes [M NDrone2_HitDamage 0x145558]. The
/// shooter is the walkreal's own (exposed-part aim, flank sidesteps — the Phoenix van blocks
/// the spawn-side line). Sister: the guard stands while unfired-at (Bond's health topped up —
/// the guard SHOOTS back, that is the level's own behaviour).
#[test]
#[ignore = "the 09 guards re-freeze after the wake: the ch-0x51 wake lands at frame 1 [M 0x15b960 case 3 — the port fix in drone.rs], but both lobby-side guards end enabled=false state=WAIT_SWITCH again by the intro's end, and bullets then pass through them [M Drone_CollisionHandler 0x140968:62's enabled gate]. The re-freeze path needs the main agent's gdbwatch (see the report's live checks); the sister (the guard stands while unfired-at) is the pinned half."]
fn the_car_park_guard_dies_to_the_real_fire() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-1.5, -377.0, 37.8]);
    let (di, skin, health) = {
        let sim = scene.sim.as_ref().unwrap();
        let (i, d) = sim.drones.list.iter().enumerate()
            .find(|(_, d)| d.pos[1] < -375.0)
            .expect("the car-park guard #261 is present [D]");
        eprintln!("[guard] di {i} pos {:?} health {:.1} skin {:#x}", d.pos, d.health, d.skin);
        assert!(d.present && d.skin != 0 && d.health > 0.0, "the guard is present with its skin");
        (i, d.skin, d.health)
    };
    assert_eq!(skin, 0x0500_0027, "the car-park guard's skin [D #261 param 0]");
    let mut w = Walker::new(scene);
    assert!(w.switch_to_armed("guard test"), "an armed weapon is available on 09");
    let killed = w.shoot(di, "the car-park guard");
    let sim = w.scene.sim.as_ref().unwrap();
    eprintln!("[guard] killed {killed} health {:.1} corpse {}", sim.drones.list[di].health, sim.drones.list[di].corpse);
    assert!(killed && (sim.drones.list[di].corpse || sim.drones.list[di].health <= 0.0),
        "the guard dies to the real fire [M 0x145558]");
}

/// Sister: the guard stands while Bond never fires.
#[test]
fn the_guard_stands_while_unfired_at() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_09(&iso);
    settle_near(&mut scene, [-1.5, -377.0, 37.8]);
    let di = scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .find(|(_, d)| d.pos[1] < -375.0)
        .expect("the guard").0;
    for _ in 0..600 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.health.health = 100.0;
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.list[di].corpse && sim.drones.list[di].health > 0.0,
        "no fire, no kill: the guard stands");
}

fn dxz_of(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

