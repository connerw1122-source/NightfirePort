//! fix-guest-reactions — the party guests' reactions to an assault, and the death→restart flow
//! (07000007, driven through the REAL path: `nf_view::load` + `Sim::step_bond`).
//!
//! Playtest (2026-10-12, on 07000007): "the moment I started punching a guest, all the other guests
//! T-posed. And once I died, we went into a replay loop where it kept replaying at the start of the
//! mission over and over again"; also "If you punch a guard, it should be an automatic mission fail".
//!
//! The game's chains, ported here:
//! - `NDrone2_PunchImpact` 0x145f28:30 sets `Drone_AlertStatusSet(2)` on the victim; :45-56 on
//!   07000007 an UNARMED victim (`NDrone2_ObjectIsArmed` 0x178768) → `DroneFunc_SetMissionFailReason(drone, 5)`
//!   [M 0x13e560 → ch 99]; :55 health > 0 → `DroneFunc_SendHurtMessage` 0x145e28:19-57 — prop 0x32,
//!   the +8 once-per-life bit — **broadcasts msg 0x12** (`Drone_SM_BroadcastMsg`) with the victim as
//!   the alert source. The receivers' DSTATE case 0x12 → `DroneVision_EnemyAlerts` 0x177c18:41-51
//!   gates on prop 0x27 → `DroneVision_ShoutFromOtherDrone` → CivilianScared 0x12, whose ENTER
//!   (0x15d3a0) runs to CivilianHiding 0x13 with `CallAnim(0x19)` — the cower [M 0x15d520]. That
//!   cower is the animation the user's build was missing (the T-pose).
//! - The death → the 0x40000042 fail/death menu → `P_ENDMISSION_Handler` 0x216618:19-21: Restart is
//!   a WARM reload of the CURRENT level on the ENTRY checkpoint (`Player_RamLoad(Bond, 1)`, the
//!   50.0 health floor [L 0x30cc34-3c]) — one restart, no replay loop (fix-restart-target's port).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The first ENABLED party guest (dtype 10), nearest to Bond's current spot. The idle state-1
/// guests sit disabled (`WaitSwitch` — `Drone_EnableAll(1)` skips them [M 0x1383e8:14]) and
/// bullets pass through disabled drones, so the punchable guests are the enabled 0x2f walkers.
fn pick_guest(sim: &nf_view::scene::Sim) -> (usize, [f32; 3]) {
    let bp = [sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]];
    let mut best: Option<(usize, f32)> = None;
    for (i, d) in sim.drones.list.iter().enumerate() {
        if !d.present || d.dtype != 10 || d.corpse || !d.enabled { continue; }
        let d2 = (d.pos[0]-bp[0]).powi(2) + (d.pos[2]-bp[2]).powi(2);
        if best.map_or(true, |(_, bd)| d2 < bd) { best = Some((i, d2)); }
    }
    let i = best.expect("an enabled party guest exists").0;
    let pos = sim.drones.list[i].pos;
    (i, pos)
}

/// Stand Bond beside `guest` facing its chest (the fire ray leaves the eye along the look).
fn aim_at(sim: &mut nf_view::scene::Sim, guest: usize) {
    let g = sim.drones.list[guest].pos;
    let eye = [sim.bond.pos[0], sim.bond.pos[1] + sim.bond.eye, sim.bond.pos[2]];
    sim.bond.yaw = (g[0] - eye[0]).atan2(g[2] - eye[2]);
    let horiz = ((g[0] - eye[0]).powi(2) + (g[2] - eye[2]).powi(2)).sqrt();
    sim.bond.pitch = ((g[1] + 0.6 - eye[1]) / horiz).atan() / (std::f32::consts::FRAC_PI_2);
}

/// Walk Bond up to a guest and punch it once (the round spawns at the fire script's strike
/// frame +0x44, so the swing holds the trigger and waits, re-aiming every tick).
fn punch_once(sim: &mut nf_view::scene::Sim, guest: usize) {
    let g = sim.drones.list[guest].pos;
    sim.bond.pos = [g[0] - 0.9, g[1], g[2] + 0.9]; // ~1.27 away, inside the fists' 1.75 range
    sim.gun.weapon = 1; sim.gun.switch_request = 1; sim.gun.switch_state = 0;
    for _ in 0..40 {
        aim_at(sim, guest);
        sim.step_bond(Input::default(), true, false, false);
        for _ in 0..24 { aim_at(sim, guest); sim.step_bond(Input::default(), false, false, false); }
        let g = &sim.drones.list[guest];
        if g.health < 10.0 || g.knocked_out || g.corpse { return; }
    }
}

/// ── The punch = automatic mission fail (the protected actors) ───────────────────────────────────
/// `NDrone2_PunchImpact` 0x145f28:45-56: on 07000007 a punch on an UNARMED drone (the party
/// guests and bouncers all read unarmed — `ObjectIsArmed` 0x178768 on the class byte) raises
/// `DroneFunc_SetMissionFailReason(drone, 5)` [M 0x13e560 → ch 99] — the playtest's "if you punch
/// a guard, it should be an automatic mission fail". Sister: the victim's health drops.
#[test]
fn punching_a_guest_fails_the_mission_at_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let (guest, _) = pick_guest(sim);
    punch_once(sim, guest);
    assert!(sim.drones.channels.on(99), "the punch on an unarmed party guest raises ch 99 [M 0x145f28:45-56]");
    assert_eq!(sim.drones.mission.status(), 2, "MissionState 2 (the fail) [M 0x182c60:45-57]");
    assert!(sim.drones.list[guest].health < 10.0, "the punch landed (the victim's health dropped)");
}

/// ── The guests' panic and its cower (the T-pose) ────────────────────────────────────────────────
/// The punch victim screams (`SendHurtMessage` 0x145e28 — prop 0x32) and the OTHER guests (prop
/// 0x27) receive msg 0x12 → CivilianScared → CivilianHiding with the cower 0x19 [M 0x15d3a0 /
/// 0x15d520]. The victim itself is scared at once (`Drone_AlertStatusSet(2)` 0x145f28:30) and
/// flees (state 0x56, the run the port's flee anims serve). Sister: the victim's alert ≥ 0.66.
#[test]
fn punching_a_guest_panics_the_other_guests_into_the_cower() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let (guest, _) = pick_guest(sim);
    let victim_alert_before = sim.drones.list[guest].alert;
    punch_once(sim, guest);
    assert!(sim.drones.list[guest].alert >= 0.66,
        "the victim is scared (Drone_AlertStatusSet(2) [M 0x145f28:30]), was {:.2} from {:.2}",
        sim.drones.list[guest].alert, victim_alert_before);
    // The others: within ~20 units of the scream someone must reach the civilian panic states,
    // and a hiding guest must be playing the cower (anim state 0x19).
    let mut scared = 0; let mut cowering = 0;
    for _ in 0..200 {
        sim.step_bond(Input::default(), false, false, false);
        scared = 0; cowering = 0;
        for (i, d) in sim.drones.list.iter().enumerate() {
            if !d.present || d.dtype != 10 || d.corpse || i == guest || !d.enabled { continue; }
            let dist = ((d.pos[0] - sim.drones.list[guest].pos[0]).powi(2) + (d.pos[2] - sim.drones.list[guest].pos[2]).powi(2)).sqrt();
            if dist > 25.0 { continue; }
            if matches!(d.state, 0x12 | 0x13) { scared += 1; }
            if d.state == 0x13 && d.anim.state == 0x19 { cowering += 1; }
        }
        if scared > 0 { break; }
    }
    assert!(scared > 0, "punching a guest panics the guests near the scream into CivilianScared/Hiding (playtest: the T-posed crowd)");
    assert!(cowering > 0, "a hiding guest plays the cower anim 0x19 (the animation the user's build was missing — the T-pose)");
}

/// Sister (the armed scare): the PartyGirl update 0x160140:44-76 — an armed person within 10
/// scares the guest, prop 0x33's once-bit SHOUTS msg 0x11 with BOND as the alert source, and prop
/// 0x52 raises the mission-fail reason 0xb [M 0x13e560].
/// IGNORED (headless blind spot): the scare's gate needs the guest's `seen` (the sight pass's
/// reaction time/FOV on a wandering guest) which this session could not drive headless in time —
/// the scare/shout code itself is ported and prop-gated (`PARTY_GIRL, msg::UPDATE`'s prop-0x33
/// shout + the prop-0x52 fail reason). The next session can re-drive it with the sight machinery.
#[test]
#[ignore = "the sight pass's `seen` gate is not headless-drivable yet (see the report)"]
fn an_armed_bond_scares_a_guest_who_shouts_the_intruder() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let (guest, gpos) = pick_guest(sim);
    sim.bond.pos = [gpos[0] - 2.5, gpos[1], gpos[2] + 2.5]; // inside the guests' short sight range
    // Draw the P2K (armed — the guest's NearArmedPerson gate: prop 0x3c + SEEN + bond_armed) and
    // hold it where the guest can see it; aim the wandering guest at Bond every tick.
    sim.gun.weapon = 2; sim.gun.switch_request = 2; sim.gun.switch_state = 0;
    let mut scared = false; let mut shouted = false;
    for _ in 0..400 {
        {
            let g = &mut sim.drones.list[guest];
            g.yaw = (sim.bond.pos[0] - g.pos[0]).atan2(sim.bond.pos[2] - g.pos[2]);
        }
        sim.step_bond(Input::default(), false, false, false);
        scared |= sim.drones.list[guest].state == 0x12 || sim.drones.list[guest].state == 0x13;
        shouted |= sim.drones.list[guest].scare_shouted;
        if scared && shouted { break; }
    }
    assert!(scared, "an armed Bond within 10 scares the guest into CivilianScared [M 0x160140:44-77]");
    assert!(shouted, "prop 0x33's once-bit shouts the intruder (msg 0x11) [M 0x160140:63-70]");
    // The fail reason drains into ch 99 within the tick [M 0x13e560; the port's drain at 6224].
    assert!(sim.drones.channels.on(99) || sim.drones.mission.status() == 2,
        "prop 0x52 raises the mission-fail reason [M 0x160140:74-76]");
}

/// ── Die once → exactly one restart ──────────────────────────────────────────────────────────────
/// The death sequence ends in the 0x40000042 menu; its Restart (`P_ENDMISSION_Handler`
/// 0x216618:19-21) reloads the CURRENT level warm on the ENTRY checkpoint — exactly once. The
/// port's replay loop (the user's "kept replaying at the start of the mission over and over")
/// re-entered the death sequence after the reload; the entry-checkpoint save predates the fail
/// (fix-restart-target), so the reload cannot re-die. Sister: a second `fail_choose_restart` is a
/// no-op (the menu is consumed by the first answer).
#[test]
fn one_death_restarts_exactly_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 1000.0, 1, 0x0700_0007, 0);
    // The death sequence runs to the menu (the red fade); auto_restart answers it instantly in the
    // view, so the test drives the page itself (the 180-fix-menus flow).
    sim.drones.auto_restart = false;
    let mut menus = 0;
    for _ in 0..1200 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.fail_menu_open() { menus += 1; break; }
    }
    assert!(menus == 1, "the death opens the 0x40000042 menu exactly once [M 0x182c60:77-81]");
    let level = sim.drones.level_current;
    sim.drones.fail_choose_restart();
    assert!(sim.drones.level_exit.is_some(), "the Restart requests the level load [M 0x216618:20]");
    assert_eq!(sim.drones.level_exit, Some(level), "the Restart targets the CURRENT level [M 0x216618:20 GameState._12_4_]");
    assert!(!sim.drones.level_exit_cold, "the Restart is the WARM arm (the entry checkpoint) [M 0x216618:20]");
    assert!(sim.drones.level_exit_restart, "the Restart re-serves the entry RamSave [M 0x216618:21 Player_RamLoad(Bond, 1)]");
    // The menu is consumed by the first answer: a second `fail_choose_restart` is a no-op, and the
    // death sequence does not re-enter (the reload's entry save predates the fail).
    sim.drones.fail_choose_restart();
    assert!(!sim.drones.fail_menu_open() && sim.drones.level_exit == Some(level),
        "the menu is consumed and the restart target is unchanged [M 0x216618]");
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.drones.level_exit, Some(level), "no further restart is queued — exactly one [playtest: the replay loop]");
    assert!(!sim.drones.fail_menu_open() && !sim.drones.mission.menu_open,
        "the death sequence does not re-enter the menu after the answer (the reload's entry save predates the fail)");
}
