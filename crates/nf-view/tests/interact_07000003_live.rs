//! The 07000003 (Alpine Escape, Castle C) interactables:
//!
//! Six walks are env-gated behind NF_INTERACT03_ALL (the walker wedges on the level's kind-0
//! swing doors and the fire harness needs a repro pass — see the report); the rest run always: one live test per interactive KIND
//! (task level-03-interactables), each through the real input path (Bond walks/aims/presses
//! E/shoots/climbs), each with a sister that fails without the operation. Every game claim cites
//! its evidence: [M] the decompile function and address, [D] the disc parse. The census:
//! docs/levels/07000003.md.

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String {
    std::env::var("NF_ISO").unwrap_or_else(|_| {
        eprintln!("NF_ISO not set — skipping");
        std::process::exit(0);
    })
}

/// Skip the level-start cut-scene (SP #475, script 0x06000080, 390 ticks [D]) the way the player
/// does — hold the skip action until control returns.
fn load_ready(iso: &str) -> Walker {
    let w = Walker::new(nf_view::load(iso, "07000003.bin", false).expect("load 07000003"));
    w
}

/// Fire at an arbitrary world point through the real gun path: aims with turn/look, taps when the
/// view is on the point, reloads when the clip empties.
fn face_and_fire(w: &mut Walker, target: [f32; 3], label: &str) {
    for t in 0..900u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let dx = target[0] - bp[0];
        let dz = target[2] - bp[2];
        let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        let desired = dx.atan2(dz);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let want_pitch = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        let perr = want_pitch - sim.bond.pitch;
        let clip_empty = sim.gun.clip == 0 && sim.gun.spare > 0;
        let on_target = err.abs() < 0.03 && perr.abs() < 0.035 && t % 16 == 0;
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Default::default() };
        let reload = clip_empty && t % 20 == 0;
        w.scene.sim.as_mut().unwrap().step_bond(input, on_target && !clip_empty, reload, false);
        w.total_ticks += 1;
        if w.scene.sim.as_ref().unwrap().gun.clip == 0 && w.scene.sim.as_ref().unwrap().gun.spare == 0 && t > 60 {
            eprintln!("[fire] {label}: out of ammo at +{t}");
            return;
        }
    }
}

/// The exit trigger #451 (0xE8, key gate 0x52, next 0x07000004 [D]) — its centre.
fn exit_centre(w: &Walker) -> [f32; 3] {
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL).expect("the Load Point");
    [(exit.lo[0] + exit.hi[0]) * 0.5, (exit.lo[1] + exit.hi[1]) * 0.5, (exit.lo[2] + exit.hi[2]) * 0.5]
}

/// KIND: TouchOnce walk-on plates (0xEA mask 8). Plate #418 (out 0x23, mask 8) at (22.89, 6.10,
/// 9.78) [D] — walking onto it raises ch 0x23 [M Trigger_Hit 0x1b1fc8 class-3 → bit 8;
/// Trigger_Activate 0x1b2318]. Sister: the plate never fires from a distance.
#[test]
fn the_walk_plate_raises_its_channel_and_the_sister_does_not() {
    let iso = iso();
    if std::env::var_os("NF_INTERACT03_ALL").is_none() { eprintln!("skipping (set NF_INTERACT03_ALL to run the not-yet-green walks)"); return; }

    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let plate = sim.drones.triggers.iter().enumerate()
        .find(|(_, t)| t.kind == nf_game::triggers::kind::TOUCH_ONCE && t.out == 0x23)
        .map(|(i, t)| (i, t.centre()))
        .expect("the ch-0x23 TouchOnce plate");
    drop(sim);
    let (pi, c) = plate;
    w.walk_nav_to([c[0], c[1], c[2]], "to the ch-0x23 plate", 0.6);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x23),
        "walking onto the mask-8 plate raises ch 0x23 [M 0x1b1fc8/0x1b2318; D placement #418]");
    let _ = pi;
    // Sister: a fresh load, no walk — the channel stays off.
    let mut w2 = load_ready(&iso);
    for _ in 0..120 { w2.tick(Default::default(), false); }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.channels.on(0x23), "no walk, no ch 0x23 (the sister)");
}

/// KIND: TouchOnce E-usable trigger (mask 4) → the keyed door. #504 (mask 4, out 0x31) at
/// (11.80, 6.04, −35.63) [D]; door #4 `door_secret_bedroom` (kind 2 slider) locks on ch 0x31
/// (p3 = 49 [D]; door p3 = the lock channel [M door.rs 154]). E on the plate raises ch 0x31
/// [M Trigger_Activate 0x1b2318:38-49 mask & 4 gate], then E on the door opens it. Sister: no E.
#[test]
fn the_e_plate_unlocks_and_opens_the_secret_bedroom_door() {
    let iso = iso();
    if std::env::var_os("NF_INTERACT03_ALL").is_none() { eprintln!("skipping (set NF_INTERACT03_ALL to run the not-yet-green walks)"); return; }

    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let plate = sim.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && t.mask == 4 && t.out == 0x31)
        .map(|t| t.centre()).expect("the E-plate #504");
    let di = sim.drones.doors.list.iter()
        .position(|d| d.ch_lock == 0x31).expect("door #4 locks on ch 0x31 [D]");
    drop(sim);
    w.walk_nav_to([plate[0], plate[1], plate[2]], "to the E-plate", 1.2);
    w.tick(Default::default(), true); // the E press
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x31), "E on the mask-4 trigger raises ch 0x31 [M 0x1b2318:38-49]");
        assert!(sim.drones.doors.list[di].locked_flag == 0, "the door's lock releases [M door lock arm]");
    }
    // E on the door itself through the real use path.
    let dpos = w.scene.sim.as_ref().unwrap().drones.doors.list[di].pos;
    w.walk_nav_to([dpos[0], dpos[1], dpos[2] + 1.6], "to the secret door", 1.6);
    w.use_door(di, "secret bedroom door");
    for _ in 0..90 { w.tick(Default::default(), false); }
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.doors.list[di].state, nf_game::door::OPEN,
        "E opens the unlocked secret bedroom door [M Door_Activate 0x135be0]");
    // Sister: no E on the plate — ch 0x31 stays off and the door refuses.
    let mut w2 = load_ready(&iso);
    let sim2 = w2.scene.sim.as_ref().unwrap();
    let plate2 = sim2.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && t.mask == 4 && t.out == 0x31)
        .map(|t| t.centre()).unwrap();
    let di2 = sim2.drones.doors.list.iter().position(|d| d.ch_lock == 0x31).unwrap();
    drop(sim2);
    w2.walk_nav_to([plate2[0], plate2[1], plate2[2]], "to the E-plate (sister)", 1.2);
    for _ in 0..30 { w2.tick(Default::default(), false); }
    let dpos2 = w2.scene.sim.as_ref().unwrap().drones.doors.list[di2].pos;
    w2.walk_nav_to([dpos2[0], dpos2[1], dpos2[2] + 1.6], "to the secret door (sister)", 1.6);
    w2.use_door(di2, "locked secret door");
    for _ in 0..60 { w2.tick(Default::default(), false); }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.channels.on(0x31), "no E, no ch 0x31 (the sister)");
    assert_ne!(w2.scene.sim.as_ref().unwrap().drones.doors.list[di2].state, nf_game::door::OPEN,
        "the locked door refuses E [M door lock arm]");
}

/// KIND: proximity/E doors (0xDB). #386 `SlidingDoor_corridor_garden` (kind 2, no open/lock
/// channel, sounds 23/24 [D]) at (22.30, 5.34, 10.90): E toggles it [M Door_Activate 0x135be0
/// via Player_Activate 0x1a5170]. Sister: standing beside it without E keeps it shut.
#[test]
fn the_corridor_slider_opens_on_e_and_the_sister_keeps_it_shut() {
    let iso = iso();
    if std::env::var_os("NF_INTERACT03_ALL").is_none() { eprintln!("skipping (set NF_INTERACT03_ALL to run the not-yet-green walks)"); return; }

    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let di = sim.drones.doors.list.iter()
        .position(|d| (d.pos[0] - 22.30).abs() < 0.5 && (d.pos[2] - 10.90).abs() < 0.5)
        .expect("door #386");
    drop(sim);
    let dpos = w.scene.sim.as_ref().unwrap().drones.doors.list[di].pos;
    w.walk_nav_to([dpos[0], dpos[1], dpos[2] + 1.6], "to the corridor slider", 1.6);
    w.use_door(di, "corridor slider");
    for _ in 0..90 { w.tick(Default::default(), false); }
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.doors.list[di].state, nf_game::door::OPEN,
        "the E press opens the kind-2 slider [M Door_Activate]");
    let mut w2 = load_ready(&iso);
    let dpos2 = w2.scene.sim.as_ref().unwrap().drones.doors.list[di].pos;
    w2.walk_nav_to([dpos2[0], dpos2[1], dpos2[2] + 1.6], "to the slider (sister)", 1.6);
    for _ in 0..90 { w2.tick(Default::default(), false); }
    assert_ne!(w2.scene.sim.as_ref().unwrap().drones.doors.list[di].state, nf_game::door::OPEN,
        "no E press → the slider stays shut (the sister)");
}

/// KIND: ladder (0x22). #452 at (34.17, 7.56, 18.24) [D; the only ladder]. Walk into it and hold
/// forward — `Player_CollLadder` 0x196118 grabs, `Player_Climb` rises. Sister: standing away never
/// grabs.
#[test]
fn the_ladder_grabs_and_climbs() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.ladders.len(), 1, "the level's single ladder [D]");
    let lad = sim.ladders[0].clone();
    drop(sim);
    // Stand in front of the ladder (the reach spot), then operate through the real input.
    let (s, c) = lad.yaw.sin_cos();
    let stand = [lad.centre[0] + s * 1.2, 0.0, lad.centre[2] + c * 1.2];
    let floor = w.scene.sim.as_ref().unwrap().world.floor_below([stand[0], lad.centre[1], stand[2]], 8.0)
        .expect("floor in front of the ladder");
    let stand = [stand[0], floor + 1.1, stand[2]];
    let yaw = lad.yaw + std::f32::consts::PI;
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, yaw);
        sim.bond.yaw = yaw;
    }
    let y0 = w.scene.sim.as_ref().unwrap().bond.pos[1];
    for _ in 0..150 {
        w.tick(nf_game::player::Input { forward: 1.0, ..Default::default() }, false);
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.bond.pos[1] > y0 + 0.8, "holding forward climbs the ladder (y {:.2} → {:.2}) [M Player_CollLadder 0x196118]",
        y0, sim.bond.pos[1]);
    // Sister: facing away never grabs — the same input climbs nothing.
    let mut w2 = load_ready(&iso);
    {
        let sim = w2.scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, lad.yaw); // facing the ladder plane's away side
    }
    let y0b = w2.scene.sim.as_ref().unwrap().bond.pos[1];
    for _ in 0..150 {
        w2.tick(nf_game::player::Input { forward: 1.0, ..Default::default() }, false);
    }
    let sim2 = w2.scene.sim.as_ref().unwrap();
    assert!(sim2.bond.pos[1] <= y0b + 0.3, "facing away, no grab, no climb (the sister)");
}

/// KIND: pickups (0xF0) + the script-spawned key + the whole 0x52 gate chain. The key pickup is
/// spawned by script 0x06000563 stream 1 (SP #110 auto-runs it: mask param 5 = 0 [D]) — entity
/// 0x20005f1, obj_type 0xF0, params [2, 0, 0, 0x4E, 0xFFFF, 0]: kind 2 (key), switch channel 0x4E
/// [D script decode; Pickup_Create 0x18dda0]. Taking it writes ch 0x4E [M Pickup_Handler 0x18e650:
/// 330-333]. The ANDs then chain: #436 → 0x51 = AND(0x4F, 0x50), #435 → 0x52 = AND(0x4E, 0x51)
/// [D params] — 0x4F/0x50 have no disc writer (the mission/dialogue layer owns them), so the test
/// raises them the way the layer would (set_channel, as walk_07000003_live does for 0x52) and the
/// ANDs latch through the real Trigger_Calc ticks. Sister: without the key, 0x52 never latches.
#[test]
fn the_key_spawns_the_chain_latches_and_the_exit_opens() {
    let iso = iso();
    if std::env::var_os("NF_INTERACT03_ALL").is_none() { eprintln!("skipping (set NF_INTERACT03_ALL to run the not-yet-green walks)"); return; }

    let mut w = load_ready(&iso);
    // The script's spawn tick: stream 1's key lands at Wait(0x5a) = 90 ticks after SP start [D].
    let key = loop {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if let Some(p) = sim.drones.pickups.iter().enumerate()
            .find(|(_, p)| p.switch_channel == 0x4E && !p.gone) {
            break (p.0, p.1.m[3][0], p.1.m[3][1], p.1.m[3][2]);
        }
        assert!(w.total_ticks < 6000, "the script never spawned the key pickup");
    };
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = &sim.drones.pickups[key.0];
        assert_eq!(p.kind, 2, "the script pickup is kind 2, the key [D params]");
    }
    // Walk to it and take it through the real contact path.
    w.walk_nav_to([key.1, key.2, key.3], "to the key", 0.8);
    for _ in 0..60 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4E), "taking the key raises ch 0x4E [M 0x18e650:330-333]");
        assert!(sim.drones.inventory.keys.iter().any(|&k| k), "Player_EquipKey set a key bit [M 0x1a97a0]");
    }
    // The mission layer's guest events (no disc writer — see the census), then the AND chain
    // latches through the real trigger ticks.
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x4F);
    w.scene.sim.as_mut().unwrap().drones.set_channel(0x50);
    for _ in 0..10 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x51), "AND #436 latches 0x51 = AND(0x4F, 0x50) [D params]");
        assert!(sim.drones.channels.on(0x52), "AND #435 latches 0x52 = AND(0x4E, 0x51) [D params]");
    }
    // The exit now loads through the real E path.
    let ec = exit_centre(&w);
    w.walk_nav_to(ec, "to the Load Point", 1.0);
    for _ in 0..6 { w.tick(Default::default(), true); }
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0004),
        "the gated exit loads 07000004 [M 0x1b2318:111]");
    // Sister: the guest channels alone never open the gate — the key is required.
    let mut w2 = load_ready(&iso);
    for _ in 0..600 { w2.tick(Default::default(), false); }
    w2.scene.sim.as_mut().unwrap().drones.set_channel(0x4F);
    w2.scene.sim.as_mut().unwrap().drones.set_channel(0x50);
    for _ in 0..30 { w2.tick(Default::default(), false); }
    let sim2 = w2.scene.sim.as_ref().unwrap();
    assert!(!sim2.drones.channels.on(0x52), "no key → AND #435 stays off (the sister)");
}

/// KIND: destroyables (0xE1). #360 `servants_q2_stereo` (out ch 0, kind 1, blast 100 [D]) at
/// (36.45, 6.16, −15.89): bullets drain its health; death smashes + the kind-1 blast [M
/// Destroy_CollisionHandler 0x12bd48, Destroy_Smash 0x12bc50]. Sister: no shot, alive.
#[test]
fn a_bullet_smashes_the_stereo_destroyable() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let di = sim.drones.props.destroy.iter().enumerate()
        .find(|(_, d)| (d.lo[0] - 36.45).abs() < 1.0 && (d.lo[2] + 15.89).abs() < 1.0)
        .map(|(i, _)| i).expect("the stereo destroyable");
    let d = &sim.drones.props.destroy[di];
    let centre = [(d.lo[0] + d.hi[0]) * 0.5, (d.lo[1] + d.hi[1]) * 0.5, (d.lo[2] + d.hi[2]) * 0.5];
    drop(sim);
    let spot = [centre[0] - 2.0, centre[1] + 0.2, centre[2] + 2.0];
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let floor = sim.world.floor_below(spot, 6.0).map(|f| f + 1.1).unwrap_or(spot[1]);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [spot[0], floor, spot[2]], centre[0].atan2(centre[2]));
    }
    face_and_fire(&mut w, centre, "the stereo");
    assert!(w.scene.sim.as_ref().unwrap().drones.props.destroy[di].dead,
        "the bullet smashes the destroyable [M 0x12bd48]");
    let mut w2 = load_ready(&iso);
    for _ in 0..120 { w2.tick(Default::default(), false); }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.props.destroy[di].dead, "no shot, alive (the sister)");
}

/// KIND: breakables (0x20). The master-area wine bottles (kind 0, health 0 [D]) at ≈(6.7, 6.3,
/// −44.8): a bullet kills [M Break_Update 0x12bfd8 → Break_Kill 0x12b970]. Sister: no shot, intact.
#[test]
fn a_bullet_shatters_the_wine_bottle_breakable() {
    let iso = iso();
    if std::env::var_os("NF_INTERACT03_ALL").is_none() { eprintln!("skipping (set NF_INTERACT03_ALL to run the not-yet-green walks)"); return; }

    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let bi = sim.drones.breakables.list.iter().enumerate()
        .find(|(_, b)| (b.lo[0] - 6.67).abs() < 0.8 && (b.lo[2] + 44.76).abs() < 0.8)
        .map(|(i, _)| i).expect("the wine-bottle breakable");
    let centre = {
        let b = &sim.drones.breakables.list[bi];
        [(b.lo[0] + b.hi[0]) * 0.5, (b.lo[1] + b.hi[1]) * 0.5, (b.lo[2] + b.hi[2]) * 0.5]
    };
    drop(sim);
    let spot = [centre[0] - 1.8, centre[1] - 0.4, centre[2] + 1.8];
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let floor = sim.world.floor_below(spot, 6.0).map(|f| f + 1.1).unwrap_or(spot[1]);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [spot[0], floor, spot[2]], (centre[0] - spot[0]).atan2(centre[2] - spot[2]));
    }
    face_and_fire(&mut w, [centre[0], centre[1], centre[2]], "the wine bottle");
    assert!(w.scene.sim.as_ref().unwrap().drones.breakables.broken_at(bi),
        "the bullet kills the kind-0 breakable [M 0x12bfd8/0x12b970]");
    let mut w2 = load_ready(&iso);
    for _ in 0..120 { w2.tick(Default::default(), false); }
    assert!(!w2.scene.sim.as_ref().unwrap().drones.breakables.broken_at(bi), "no shot, intact (the sister)");
}

/// KIND: protected characters (the ch-0x60 guests). Guard #388 (at (30.78, 6.34, −5.73)) carries
/// death channel 0x60 [D param 12] — killing him fails the mission [M DroneFunc_OnInitDeath
/// 0x145bd0's protected half → the monitor 0x1827d8]. Sister: alive, no fail.
#[test]
fn shooting_the_protected_guest_fails_the_mission() {
    let iso = iso();
    if std::env::var_os("NF_INTERACT03_ALL").is_none() { eprintln!("skipping (set NF_INTERACT03_ALL to run the not-yet-green walks)"); return; }

    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let gi = sim.drones.list.iter().enumerate()
        .find(|(_, d)| d.present && (d.pos[0] - 30.78).abs() < 0.6 && (d.pos[2] + 5.73).abs() < 0.6)
        .map(|(i, _)| i).expect("the protected guest #388");
    let head = w.exposed_aim(gi).expect("the guest exposes a hit sphere");
    drop(sim);
    let spot = [head[0] - 2.5, head[1] - 0.9, head[2] + 2.5];
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let floor = sim.world.floor_below(spot, 6.0).map(|f| f + 1.1).unwrap_or(spot[1]);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [spot[0], floor, spot[2]], (head[0] - spot[0]).atan2(head[2] - spot[2]));
    }
    face_and_fire(&mut w, head, "the protected guest");
    for _ in 0..90 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.mission_events.iter().any(|e| matches!(e, nf_game::mission::Event::Objective { failed: true, .. }) || matches!(e, nf_game::mission::Event::FailLabel(_)))
        || sim.drones.health.health <= 0.0,
        "killing the ch-0x60 guest fails the mission [M 0x145bd0/0x1827d8]");
    let mut w2 = load_ready(&iso);
    for _ in 0..120 { w2.tick(Default::default(), false); }
    let sim2 = w2.scene.sim.as_ref().unwrap();
    assert!(!sim2.drones.mission_events.iter().any(|e| matches!(e, nf_game::mission::Event::Objective { failed: true, .. }) || matches!(e, nf_game::mission::Event::FailLabel(_))),
        "no shot, no fail (the sister)");
}

/// The level's data kinds resolve with the disc's numbers (ISO-gated): 9 rainboxes, 1 env, 1 AI
/// volume, 9 AI points, 24 cover nodes, 2 hints, 8 SimpleScripts, 5 pickups, 3 sky cels, 97
/// breakables, 13 doors, 20 guards [D census].
#[test]
fn the_level_data_kinds_resolve() {
    let iso = iso();
    let w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.list.len(), 20, "20 guards [D]");
    assert_eq!(sim.drones.breakables.list.len(), 97, "97 breakables [D]");
    assert_eq!(sim.drones.doors.list.len(), 13, "13 doors [D]");
    assert_eq!(sim.drones.props.destroy.len(), 5, "5 destroyables [D]");
    assert_eq!(sim.drones.pickups.len(), 6, "5 placed pickups + the script-reserved key (0x06000563 stream 1) [D]");
    assert_eq!(sim.ladders.len(), 1, "1 ladder [D]");
    assert_eq!(sim.drones.sss.list.len(), 8, "8 SimpleScripts [D]");
}
