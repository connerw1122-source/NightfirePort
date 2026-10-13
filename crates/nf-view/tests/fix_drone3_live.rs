//! fix-drone3 repro tests, driven through the real path (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's multi-level playtest (docs/playtests/2026-10-11.md, the fix-drone3 follow-ups):
//! C1 the 07000006 friendly driver (hands behind back, torso twisting to follow Bond, shootable);
//! C2/C3 the frozen non-guard characters (07000002 hostage-side NPC, 07000007 guests and bouncers —
//!       "NPCs only turn, never walk");
//! the 07000005 patrolling guard whose upper body looks the opposite way ("spine flipped");
//! O2 the armour plate on 07000006's helicopter pad pick-upable but not drawn.
//!
// State ids are written raw (`0x32` TruckDriverInit …) so this file compiles — and its assertions
//! fail — against the pre-fix master, where none of these states exist or are handled.
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Stand Bond at `at` with the P2K (armed) and look at `to`.
fn stand_armed_at(sim: &mut nf_view::scene::Sim, at: [f32; 3], to: [f32; 3]) {
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
    // Armed: the P2K in hand. 07000007 now starts Bond on the fists (fix-level2's F4 party weapon lock), so the test
    // must draw the gun itself; the drones read `Sim::gun.weapon` as `bond_weapon` each tick. fix-party-mission:
    // the AI reads the TARGET class mid-switch [M 0x1a2cc0], so the stale fists request must be cleared too —
    // otherwise the switch completes into the fists and Bond reads unarmed.
    sim.gun.weapon = 2;
    sim.gun.switch_request = 2;
    sim.gun.switch_state = 0;
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    sim.bond.yaw = d[0].atan2(d[2]);
}

/// C1 / the torso half. The friendly driver (skin 0x05000035, dtype 13 → dtypenew 0x17 [M DefaultInit
/// 0x14b300 case 0x5000035]) starts in TruckDriverInit (0x32, mode 0x17's start [M DroneTypeSettings]) and
/// — the port ran no handler for it — sat there forever while the old torso-aim gate
/// (`anim.class == 2 && state != IDLE`) twisted his chest toward Bond. The game: the start anim finishes →
/// TruckDriverIdle (0x34) [M 0x160bd0 case 5 with the +0x9f2 script cleared], and an unarmed driver never
/// carries the aim flag [M AnimObjectAimAt 0x1bfe58: `AnimDatumGetWeaponInfo < 0` returns].
#[test]
fn the_truck_driver_reaches_his_idle_state_and_never_twists() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let di = sim.drones.list.iter().position(|dr| dr.skin == 0x0500_0035).expect("the 07000006 driver (skin 0x05000035)");
    tick_idle(sim, 2);
    assert_eq!(sim.drones.list[di].state, 0x34,
        "the driver (no script link in the port) takes TruckDriverInit's script-less branch into TruckDriverIdle [M 0x160bd0 case 1 else]");
    // Walk armed Bond up to the driver: TruckDriverIdle's NearArmedPerson scares him into CivilianScared →
    // CivilianHiding [M 0x160ed0 case 3 → 0x1787c8], and the old build additionally twisted his torso all the
    // while. fix-party-mission: NearArmedPerson is prop 0x3c + Bond IN SIGHT (+0x228 & 4) + ObjectIsArmed
    // [M 0x1787c8 — the distance arg is dead], so Bond stands in the driver's facing (±π/2 cone) with LOS.
    let dp = sim.drones.list[di].pos;
    let dy = sim.drones.list[di].yaw;
    stand_armed_at(sim, [dp[0] + dy.sin() * 4.0, dp[1], dp[2] + dy.cos() * 4.0], dp);
    tick_idle(sim, 180);
    let dr = &sim.drones.list[di];
    assert!(matches!(dr.state, 0x12 | 0x13),
        "the driver reacts to armed Bond within 8 through TruckDriverIdle's scare [M 0x160ed0 case 3] — got {:#x}", dr.state);
    assert!(dr.aim_yaw.abs() < 1e-4,
        "an unarmed driver never aims his torso [M AnimObjectAimAt weapon gate] — aim_yaw {}", dr.aim_yaw);
    // Sister: he holds his truck-bed height (gravity keeps him standing on it — the old build's scripted
    // no-gravity stand-in would have let him drift).
    assert!((dr.pos[1] - dp[1]).abs() < 0.05, "the driver holds his truck-bed height ({} vs {})", dr.pos[1], dp[1]);
}

/// The 07000005 patrol look: "a patrolling guard walks one way while his upper body looks the opposite
/// way, as if his spine were flipped". The old gate aimed every non-IDLE danim-class-2 torso (including
/// PATROL and WAIT_SWITCH) and clamped only the negative side (`.max(-0.9)`), so a patroller's chest wound
/// up to ±π. The game aims only in the aim/fire states [M psiBuildMatrixPalette 0x111498 `(anim+0x58 & 0x180)`;
/// AnimObjectAimAt gives up beyond ±0.9 either side, 0x1c0368].
#[test]
fn a_patrolling_guard_walks_with_an_untwisted_torso() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 60);
    let pi = sim.drones.list.iter().position(|dr| dr.present && dr.enabled && dr.state == 0x07)
        .expect("07000005 starts a patrolling guard");
    // Stand armed Bond behind the patroller: the old build twisted his torso toward the clamp here.
    let pp = sim.drones.list[pi].pos;
    stand_armed_at(sim, [pp[0], pp[1], pp[2] - 6.0], pp);
    let mut input = Input::default();
    input.forward = 0.0;
    for _ in 0..90 { sim.step_bond(input, false, false, false); }
    let dr = &sim.drones.list[pi];
    assert_eq!(dr.state, 0x07, "the guard keeps patrolling");
    assert!(dr.aim_yaw.abs() < 1e-4,
        "a patrolling guard does not aim his torso [M the aim-state gate] — aim_yaw {}", dr.aim_yaw);
    // Sister: the walk keeps covering ground (the fix did not freeze the patrol).
    let moved = ((dr.pos[0] - pp[0]).powi(2) + (dr.pos[2] - pp[2]).powi(2)).sqrt();
    assert!(moved > 0.5 || dr.anim.state == nf_game::drone::anim_state::WALK || dr.anim.call.is_some(),
        "the patroller still walks (moved {moved:.2}, anim {:#x})", dr.anim.state);
}

/// C3. The party level (07000007): the female guests (skins 0x05000020/22/23/24 — DefaultInit's female
/// list [M 0x14b300]) are mode 0x12 → PartyGirlInit → PartyGirl (0x2f) [M GetDTYPENEW 0x14c938 case 10/0xb
/// female → 0x12; DroneTypeSettings mode 0x12 start 0x2e], and the male bouncers (dtypenew 0x13) are
/// CivilianGuard (0x30) [M GetDTYPENEW 07000007 case 0..9]. The port had NO handler for either: the guests
/// stood frozen and the bouncers never reacted. PartyGirl wanders the patrol path; her scare and
/// CivilianGuard's attack both run through `NDrone2_NearArmedPerson` 0x1787c8 — prop 0x3c + Bond in sight +
/// ObjectIsArmed(Bond); the distance args are dead and kind 0xD never qualifies [M disassembly].
#[test]
fn the_party_guests_wander_and_the_bouncers_react() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 150);
    let guest = sim.drones.list.iter().position(|dr| dr.present && dr.enabled
        && matches!(dr.skin & 0xffff, 0x20 | 0x22 | 0x23 | 0x24) && dr.dtype == 10)
        .expect("07000007 places female guests");
    let bouncer = sim.drones.list.iter().position(|dr| dr.present && dr.enabled
        && dr.team == 1 && dr.weapon == 1 && dr.mode == 0x13)
        .expect("07000007 places unarmed CivilianGuard bouncers");
    assert_eq!(sim.drones.list[guest].state, 0x2f,
        "a female guest runs PartyGirl [M 0x160140] — got {:#x}", sim.drones.list[guest].state);
    assert_eq!(sim.drones.list[bouncer].state, 0x30,
        "a bouncer runs CivilianGuard [M 0x1603a8] — got {:#x}", sim.drones.list[bouncer].state);

    // Armed Bond walks up to the bouncer: he attacks (0x56). (Pre-fix: stood in 0x30 forever.)
    // fix-party-mission: NearArmedPerson is prop 0x3c + Bond IN SIGHT + ObjectIsArmed [M 0x1787c8 —
    // the 20.0 distance arg is dead], so Bond stands in the bouncer's facing (±π/2 cone) with LOS.
    let bp = sim.drones.list[bouncer].pos;
    let by = sim.drones.list[bouncer].yaw;
    stand_armed_at(sim, [bp[0] + by.sin() * 6.0, bp[1], bp[2] + by.cos() * 6.0], bp);
    tick_idle(sim, 90);
    assert!(matches!(sim.drones.list[bouncer].state,
            0x56 | 0x66 | 0x67 | 0x68 | 0x69),
        "a bouncer leaves CivilianGuard into the attack chain when armed Bond is within 20 [M 0x1603a8 case 3 → 0x56; the unarmed draw is the game's Attack enter] — got {:#x}", sim.drones.list[bouncer].state);

    // Armed Bond walks up to a guest: fix-party-mission — PartyGirl's scare runs through the same
    // NearArmedPerson gate [M 0x160140 case 3], and kind 0xD never qualifies [M 0x1787c8 0x178840] —
    // so a guest does NOT scare from proximity alone: she stays in PartyGirl (stand idle + BondTalk).
    // (The old within-10 scare was the [S] distance reading the disassembly refutes.)
    let gp = sim.drones.list[guest].pos;
    let gy = sim.drones.list[guest].yaw;
    stand_armed_at(sim, [gp[0] + gy.sin() * 5.0, gp[1], gp[2] + gy.cos() * 5.0], gp);
    tick_idle(sim, 90);
    assert_eq!(sim.drones.list[guest].state, 0x2f,
        "a kind-0xD guest never scares from armed-Bond proximity [M 0x1787c8 0x178840; 0x160140 case 3] — got {:#x}",
        sim.drones.list[guest].state);
}

/// O2. The armour plate on 07000006's helicopter pad: fix-player2's placed-pickup instancing must cover
/// this level too — the kind-3 pickup has a drawn instance at its placement matrix until taken
/// [M View_DrawObjectGlist 0x1e6780 draws obj+0xD8 at obj+0x90].
#[test]
fn the_helipad_armour_pickup_is_drawn_on_07000006() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let armours: Vec<usize> = sim.drones.pickups.iter().enumerate()
        .filter(|(_, p)| p.kind == nf_game::pickup::kind::ARMOUR && !p.gone)
        .map(|(i, _)| i).collect();
    assert!(!armours.is_empty(), "07000006 places a kind-3 armour pickup");
    for k in armours {
        let m = sim.placed_pickup_pose(k).expect("the armour has a drawn instance (View_DrawObjectGlist)");
        let want = sim.drones.pickups[k].m;
        for r in 0..3 { for c in 0..4 { assert!((m[r][c] - want[r][c]).abs() < 1e-4, "pose matches obj+0x90 [{r}][{c}]"); } }
    }
    // Sister: once taken, nothing is drawn.
    let k = sim.drones.pickups.iter().position(|p| p.kind == nf_game::pickup::kind::ARMOUR && !p.gone).unwrap();
    sim.drones.pickups[k].gone = true;
    assert!(sim.placed_pickup_pose(k).is_none(), "a taken pickup draws nothing");
}

/// The searchlight spotted → alarm path on 07000006. The four gate lights start OFF (param 1 = the
/// on-channel 64/63/61/62 → state 3 [M Searchlight_Update 0x1cb900 gate; the atlas param table]); with the
/// on-channel raised they sweep, and when Bond stands in the beam the spot arms the head (0→1, spot-dist
/// ≤ 2.0 [M 0x1cbf14-0x1cbf54]) which raises the alarm channel — param 3 = 55 [M state-1 entry
/// `switch_channels[inst+0x36] = 1`, 0x1cbf58 area] — every tracking tick. The ch-55 waiters (the gatehouse
/// guards) then wake through their WaitSwitch.
#[test]
fn the_searchlight_beam_arms_raises_the_alarm_channel_and_wakes_its_guards() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..2 { sim.step_bond(Default::default(), false, false, false); }
    // fix-searchlight-alarm: the level boots NIS 0x06000066 (its stream 0 raises the heads' on-channels
    // 64/63/61/62 at waits 1/59/116/167 [D 07000006 NIS 0x06000066]), so head part 0 (on-channel 64) is
    // already lit here — the manual raise below only re-writes the same channel. The spot stays parked
    // at the −10000 offset until the first update [M Searchlight_Create].
    let waiters: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.switch_channel == 55).map(|(i, _)| i).collect();
    assert!(!waiters.is_empty(), "07000006 places guards waiting on the searchlight alarm channel 55");
    // Raise the light's on-channel the way a trigger would, then stand Bond in the beam: the spot's ray
    // repositions it every tick, and the arm test is spot-pos vs Bond ≤ 2.0 [M 0x1cbf14-0x1cbf54] — so walk
    // Bond onto wherever the spot is each tick (the game's "standing in the beam"). The chase caps his
    // rise at +3 over his start: climbing onto the lit courtyard (the spot's floor is ~2.8 up from the
    // sunken approach) is wanted, but with the player-pick sphere in the spot ray (fix-npcs,
    // Collide_PickObj 0x1eae80) the spot rides his sphere once the beam is on him — an uncapped chase
    // would walk him up into the sky.
    let head_pos = sim.drones.searchlights.parts[0].pos;
    stand_armed_at(sim, [head_pos[0] + 8.0, head_pos[1] - 9.0, head_pos[2] + 2.0], head_pos);
    let foot_y = sim.bond.pos[1];
    let mut lit = false; let mut alarmed = false; let mut woke = false;
    for _ in 0..600 {
        let spot = sim.drones.searchlights.parts.iter().find(|p| p.kind == 2 && p.pos[0] > -9000.0).map(|p| p.pos);
        if let Some(sp) = spot { stand_armed_at(sim, [sp[0], (sp[1] + 0.4).min(foot_y + 3.0), sp[2]], sp); }
        sim.step_bond(Default::default(), false, false, false);
        let sl = &sim.drones.searchlights.parts[0];
        if sl.state == 0 || sl.state == 1 { lit = true; }
        if sl.state == 1 && sim.drones.channels.on(55) { alarmed = true; }
        if sim.drones.list[waiters[0]].enabled { woke = true; break; }
    }
    assert!(lit, "the light lights (state 0 sweep) once its on-channel is raised");
    assert!(alarmed, "tracking raises the alarm channel 55 [M inst+0x36 = param 3]");
    assert!(woke, "the ch-55 guards wake through WaitSwitch (the user's frozen-gatehouse guards)");
}
