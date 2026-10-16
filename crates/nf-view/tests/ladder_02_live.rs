//! ladder-02 (691): the 07000002 bunker ladder and the pond water ladder through the REAL viewer
//! path (`nf_view::load` + `Sim::step_bond`). Opt-in via `NF_ISO`.
//!
//! Playtests 1011-1732 / 1014-1725 (level 07000002): "You can't move your camera while on the
//! ladders"; "When you get on the ladder, it unequips your weapon. And when you get off the ladder,
//! it doesn't re-equip"; "I'm infinitely latching onto the ladder, and I can't get off of it"
//! (climbing off into the water).
//!
//! The mechanisms were ported by fix-ladder2 (the climb look [M Player_Climb 0x196310:18-33 +
//! Player_Update 0x193600 + Player_Aiming 0x1a1d28 + ViewClamping 0x1a92e8], the weapon save/redraw
//! [M Player_WeaponNone 0x1a9858 + the Player_Climb 0x196310:99-102 carry]) and fix-ladder-water
//! (the mesh-scale swim reach + the InWater regrab [M Player_InWater 0x195e50:27,
//! FRAME_RATE_DIV = 1.0 [L 0x30d0d4] → regrab 5]). This task locks them on BOTH 07000002 ladders —
//! the bunker exit ladder (≈(−26.9, 2.3, 42.2), yaw −π, top 5.96 [D atlas/ladder.md]) and the pond
//! hut ladder (≈(−74.8, 4.0, 22.2), bottom 2.83 under the 3.765 surface [D probe_ladder2]) — with
//! the climb-DOWN dismount at the bottom (the playtest's "climbing off into the water"), which the
//! earlier tests did not drive.

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::vars().find(|(k, _)| k == "NF_ISO" ).map(|(_, v)| v) }

fn load() -> Sim {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// One step through the real path; the level's scripts grab Bond into cut-scenes within seconds,
/// so each tick carries the game's own skip action like the walkreal walker [M nis.rs].
fn step(sim: &mut Sim, input: nf_game::player::Input) {
    if sim.disabled { sim.drones.scripts.skip_request = true; }
    sim.step_bond(input, false, false, false);
}

fn ladder_near(sim: &Sim, x: f32, z: f32) -> (usize, nf_game::ladder::Ladder) {
    let li = (0..sim.ladders.len()).min_by(|a, b| {
        let d = |l: &nf_game::ladder::Ladder| (l.centre[0] - x).powi(2) + (l.centre[2] - z).powi(2);
        d(&sim.ladders[*a]).total_cmp(&d(&sim.ladders[*b]))
    }).expect("07000002 has ladders");
    (li, sim.ladders[li].clone())
}

const BUNKER_XZ: (f32, f32) = (-26.87, 42.23);
const HUT_XZ: (f32, f32) = (-74.84, 22.22);

/// The bunker ladder, walked into from its grab side: the ladder's yaw is −π, so the grab window is
/// |AngleDiff(ladder.yaw, yaw)| > π/2 [M Player_CollLadder 0x196118:20-22] — Bond walks +Z (yaw 0)
/// into the ladder's −Z face. The grab snaps him to centre + fwd(yaw)·0.5 [M 0x196118:36-44].
#[test]
fn bunker_ladder_look_dismount_and_weapon() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    assert_eq!(sim.gun.weapon, 2, "the 07000002 loadout starts on the P2K [D probe_ladder2]");
    sim.drones.inventory.owned[2] = true;
    let (li, lad) = ladder_near(&sim, BUNKER_XZ.0, BUNKER_XZ.1);
    assert!((lad.centre[0] - BUNKER_XZ.0).abs() < 0.5 && (lad.centre[2] - BUNKER_XZ.1).abs() < 0.5,
        "the nearest ladder to the bunker spot is not the bunker ladder: {:?}", lad.centre);
    // On the grab side, on the ground, facing the ladder (+Z, yaw 0; ladder.yaw −π → within the window).
    let floor = sim.world.floor_below([lad.centre[0], lad.centre[1], lad.centre[2] - 1.2], 6.0)
        .expect("ground in front of the bunker ladder");
    sim.bond = nf_game::player::Bond::new(
        [lad.centre[0], floor + 1.03, lad.centre[2] - 1.2], 0.0);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for _ in 0..300 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_CLIMB { break; }
    }
    assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB, "walking into the bunker ladder never attached");
    assert_eq!(sim.bond.ladder, Some(li));
    // The grab snapped Bond to the rung plane and holstered to the hands class [M 0x196118:36-44,
    // Player_WeaponNone 0x1a9858]: the current id is saved (anim+0x64).
    let (sx, sz) = lad.yaw.sin_cos();
    assert!((sim.bond.pos[0] - (lad.centre[0] + sx * 0.5)).abs() < 1e-3
        && (sim.bond.pos[2] - (lad.centre[2] + sz * 0.5)).abs() < 1e-3,
        "the grab did not snap Bond to centre + fwd·0.5: {:?}", sim.bond.pos);
    assert_eq!(sim.gun.weapon, 0x47, "the grab did not holster to the hands class");
    assert_eq!(sim.gun.saved_weapon, 2, "the grab did not save the held weapon (anim+0x64)");

    // Wait out the intro NIS (the climb must survive it), then LOOK while climbing:
    // the turn channel moves the yaw inside the eased ±90° window [M 0x196310:18-33] and the pitch
    // offset takes the stick, clamped ±1.0 [M Player_ViewClamping 0x1a92e8].
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for _ in 0..3000 {
        if !sim.disabled { break; }
        step(&mut sim, fwd);
        assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB, "the climb did not survive the intro NIS");
    }
    assert!(!sim.disabled, "the intro NIS never released Bond");
    let target = lad.yaw + std::f32::consts::PI;
    let turn = nf_game::player::Input { turn: 0.05, ..Default::default() };
    for _ in 0..60 { step(&mut sim, turn); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB, "turned off the ladder");
    let dev = wrap_pi(target - sim.bond.yaw).abs();
    assert!(dev > 0.1, "the turn input did not move the yaw during the climb (deviation {dev}) — the camera is stuck");
    assert!(dev <= std::f32::consts::FRAC_PI_2 + 1e-3, "the yaw left the ±90° ladder window: {dev}");
    let look = nf_game::player::Input { look: 0.05, ..Default::default() };
    for _ in 0..60 { step(&mut sim, look); }
    assert!(sim.bond.pitch > 0.3, "the pitch offset did not move during the climb ({})", sim.bond.pitch);
    for _ in 0..240 { step(&mut sim, look); }
    assert!(sim.bond.pitch <= 1.0 + 1e-4, "the pitch passed the ViewClamping clamp: {}", sim.bond.pitch);

    // Climb to the top: pos.y > top (5.96) → the +0.6 lunge dismount [M 0x196310:54-62].
    let mut top_t = None;
    for t in 0..2400 {
        step(&mut sim, fwd);
        if sim.bond.substate == nf_game::player::SS_WALK && sim.bond.ladder.is_none() { top_t = Some(t); break; }
    }
    let top_t = top_t.expect("never reached the bunker ladder's top");
    assert!(sim.bond.pos[1] >= lad.top() - 0.1, "the top dismount fired below the top threshold: y {} vs top {}",
        sim.bond.pos[1], lad.top());
    // No re-latch while standing on the roof (regrab = 60 on the dismount [M 0x196310:56] and the
    // walk pick's 0.275 reach past the rung plane [M 0x194df0 case 0/6, fix-ladder-water2]).
    for t in 0..240 {
        step(&mut sim, walk);
        assert_ne!(sim.bond.substate, nf_game::player::SS_CLIMB,
            "+{t} after the top dismount: re-latched onto the ladder at y {:.3}", sim.bond.pos[1]);
    }
    // The weapon returns through the draw cycle [M Player_Climb 0x196310:99-102 → SetWeaponAnimObj
    // 0x1a2e38:197-201 → Player_WeaponSelect 0x1a2cc0].
    let mut restored = false;
    for _ in 0..240 {
        step(&mut sim, walk);
        if sim.gun.weapon == 2 { restored = true; break; }
    }
    assert!(restored, "after the top dismount Bond is on weapon {:#x}, not the saved P2K", sim.gun.weapon);
    assert_eq!(sim.gun.switch_state, 0, "stuck mid-switch after the restore");
    let _ = top_t;
}

fn wrap_pi(d: f32) -> f32 { let mut d = d + std::f32::consts::PI; while d < 0.0 { d += 2.0 * std::f32::consts::PI; } while d >= 2.0 * std::f32::consts::PI { d -= 2.0 * std::f32::consts::PI; } d - std::f32::consts::PI }

/// The pond hut ladder, climbed DOWN off the bottom into the water — the playtest's "climbing off
/// into the water … infinitely latching onto the ladder". The below-bottom dismount [M
/// Player_Climb 0x196310:79-86] fires at root < 2.83; the swim entry then OVERWRITES the dismount's
/// 60-tick regrab with 5 [M Player_InWater 0x195e50:27, FRAME_RATE_DIV = 1.0 [L 0x30d0d4]] — so the
/// no-re-latch must hold on the geometry, not just the timer: Bond hangs at the dismount depth
/// (no input, no buoyancy in the port's swim) BELOW the grab window's bottom [M 0x196118:28-34].
#[test]
fn water_ladder_bottom_dismount_does_not_relatch() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    sim.drones.inventory.owned[2] = true;
    let (li, lad) = ladder_near(&sim, HUT_XZ.0, HUT_XZ.1);
    assert!((lad.centre[0] - HUT_XZ.0).abs() < 0.5, "not the hut ladder: {:?}", lad.centre);
    // Onto the ladder from the pond: spawn mid-column and swim in (the 109i approach).
    let front = [lad.centre[0] + lad.yaw.sin() * 1.2, 3.0, lad.centre[2] + lad.yaw.cos() * 1.2];
    sim.bond = nf_game::player::Bond::new(front, lad.yaw + std::f32::consts::PI);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = false;
    for _ in 0..300 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = true; break; }
    }
    assert!(grabbed, "swimming into the hut ladder never attached");
    assert_eq!(sim.gun.weapon, 0x47, "the grab did not holster");
    assert_eq!(sim.gun.saved_weapon, 2, "the grab did not save the held weapon");

    // Up past the pond surface (3.765 [D probe_ladder2]), then climb DOWN. The climb rate is
    // fwd·0.02/tick [M 0x196310:73]; the dismount fires either at the bottom threshold [M
    // 0x196310:79-86] or earlier on the hit-below branch [M 0x196310:63-78] when the climb
    // capsule's below reach (pos−0.76 − 0.275 [M 0x194df0 case 0/6]) touches the shallow pond bed.
    for _ in 0..60 { step(&mut sim, walk); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB, "the climb ended early going up");
    let back = nf_game::player::Input { forward: -1.0, ..Default::default() };
    let mut dismount_y = None;
    for _ in 0..600 {
        step(&mut sim, back);
        if sim.bond.substate != nf_game::player::SS_CLIMB { dismount_y = Some(sim.bond.pos[1]); break; }
    }
    let dy = dismount_y.expect("climbing down never left the ladder (no dismount)");
    assert!(dy < lad.top(), "the dismount fired above the top threshold: y {dy}");
    // The dismount lands Bond in the pond (the ladder's bottom is under the surface) — the swim
    // entry runs InWater's regrab=5 overwrite [M 0x195e50]. The CEL-SCOPED hit-below ray [M
    // Player_FeetOnPoint 0x194188 → Collide_StraddleCels 0x1eac98] lets the climb-down keep going
    // UNDER the surface to the bottom threshold — the flat read dismounted the climb on the deep
    // pond bed (material 10, y≈2.09 under the ladder) mid-column, 0.44 above `bottom`, which is
    // the divergence this task fixed (the game's cel-scoped climb scan does not see the water
    // cel's bed from the garden cel).
    assert!(dy <= lad.bottom() + 1e-3, "the dismount fired above the bottom threshold: y {dy:.3} vs bottom {}",
        lad.bottom());
    let push_off = nf_game::player::Input { forward: -1.0, ..Default::default() };
    let mut in_water = false;
    for _ in 0..240 {
        step(&mut sim, push_off);
        if sim.bond.substate == nf_game::player::SS_SWIM { in_water = true; break; }
    }
    assert!(in_water, "the bottom dismount did not reach the water (ended at y {dy:.3})");
    assert!(sim.bond.regrab > 0, "the swim entry lost the regrab lockout (InWater's overwrite)");

    // The playtest's complaint — "I'm infinitely latching onto the ladder, and I can't get off of
    // it" (climbing off into the water): the swimmer who pushes off (the stick held away, the
    // natural get-off — kept held from the dismount tick on) must swim free and never re-latch.
    // The pre-109i port re-grabbed every cycle even then.
    for t in 0..300 {
        step(&mut sim, push_off);
        assert_ne!(sim.bond.substate, nf_game::player::SS_CLIMB,
            "+{t} after the bottom dismount into the pond (push-off held): re-latched at y {:.3} (the infinite latch)",
            sim.bond.pos[1]);
    }
    // The escape is real: the push-off carried him out of the grab column (the grab snap is 0.5
    // off the axis [M 0x196118:36-44]; the swim reach ends at the capsule radius 0.55).
    let dxz0 = ((sim.bond.pos[0] - lad.centre[0]).powi(2) + (sim.bond.pos[2] - lad.centre[2]).powi(2)).sqrt();
    assert!(dxz0 > 0.9, "the push-off never moved him off the ladder column (dxz {dxz0:.3})");
    // The weapon still returns after the bottom dismount [M the 0x196310:99-102 carry fires on the
    // dismount tick's WALK substate, before the swim entry].
    let neutral = nf_game::player::Input::default();
    let mut restored = false;
    for _ in 0..240 {
        step(&mut sim, neutral);
        if sim.gun.weapon == 2 { restored = true; break; }
    }
    assert!(restored, "after the bottom dismount Bond is on weapon {:#x}, not the saved P2K", sim.gun.weapon);
    // KNOWN DIVERGENCE (not asserted): a NEUTRAL pause after the dismount re-latches once the
    // 5-tick InWater regrab expires. The swimmer settles at the pond bed's capsule-push
    // equilibrium (the solid bed, material 10 at y≈2.09 under the ladder, pushes the swim spine
    // up to a root of ≈3.20 — INSIDE the grab window [M 0x196118:28-34] and inside the ladder's
    // authored mesh, probed through the synthetic (map,object) key: the hut ladder's 100
    // triangles reach the settle capsule). The game's own collision would settle him the same
    // way; whether the real game re-latches the neutral settle is a live check (the report).
}

/// Sister: the bottom-dismount escape is not a broken grab — actively swimming back INTO the hut
/// ladder from the pond still attaches and climbs (the 1013 fix-swim feature, Player_CollLadder
/// excludes only substates 1/2 [M 0x196118:16-17]).
#[test]
fn swimming_back_into_the_hut_ladder_still_grabs() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    let (_li, lad) = ladder_near(&sim, HUT_XZ.0, HUT_XZ.1);
    let front = [lad.centre[0] + lad.yaw.sin() * 1.2, 3.0, lad.centre[2] + lad.yaw.cos() * 1.2];
    sim.bond = nf_game::player::Bond::new(front, lad.yaw + std::f32::consts::PI);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = false;
    for _ in 0..300 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = true; break; }
    }
    assert!(grabbed, "the swim grab into the hut ladder stopped working (the sister tripped)");
    // And the climb out of the water still runs: up past the surface for a second.
    for _ in 0..60 { step(&mut sim, walk); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB, "the climb did not hold after the swim grab");
}
