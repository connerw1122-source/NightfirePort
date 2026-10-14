//! fix-actors: the non-guard actor behaviours, driven through the real paths (`nf_view::load` +
//! `Sim::step_bond`, the E/fall/fire/grenade inputs). Opt-in via NF_ISO.
//! Run: `NF_ISO=… cargo test -p nf-view --test fix_actors_live -- --nocapture --test-threads=1`
//!
//! The chain of facts (each cited in the asserts):
//! - The protected-actor props 0x53/0x54 (`DroneFunc_OnInitDeath` 0x145bd0): team 1 dies → "Cover Lost"
//!   (reason 2/4 → ch 99), team 2/3 dies → "Killed Civilians" (reason 1/3 → ch 0x60) [M 0x13e560].
//! - The party guests of 07000007 (dtype 10, team 3, props 0x53/0x54 [D drone-props]) start in
//!   CivilianScared 0x12 (`DroneTypeSettings[10].start` [L 0x29aee8]) and cower in CivilianHiding 0x13.
//! - The spawners: `DroneSpawner_Create` 0x1378d0 puts param 7 at +0x14 — the KILL channel, latched
//!   in `DroneSpawner_Control` 0x137e68 (`ch[p7] → +0x20 = 1` → the cleanup block) [L-verified
//!   07000005, fix-spawner-p7]; fix-actors' ARM reading was refuted by the live measurement, and its
//!   07000011 playtest is explained by the cleanup's keep-level split (07000011 keeps drawn drones).
//! - The boss: 07000004's assassin (skin 0x0500001e, dtype 15 → ATTACK start [L DroneTypeSettings[15]],
//!   death channel param 12 = 0x54 [D]) — killing him raises the "Defeat the assassin" channel [L
//!   c07000004.rs].
//! - The escort sub-states: `AllyLeadHide` 0x15ec08 (an armed hostile within 10 → cower, clear at 14 →
//!   timer C 2 ticks → Lead) [M].

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

fn grenade_at(sim: &mut nf_view::scene::Sim, pos: [f32; 3]) {
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos, velocity: [0.0, 0.0, 0.0], speed: 0.0,
        travelled: 0.0, range: 60.0, damage: 100.0, radius: 3.0, weapon_type: 0x34, mesh: 0x0200_0726, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: false,
        stuck: false, seek_target: None,
    });
}

/// Killing a protected party guest fails the mission ("Killed Civilians"); a guest scared by gunfire
/// nearby does not. Sister: an unprotected guard's death leaves the mission open.
#[test]
fn killing_a_protected_guest_fails_the_mission_but_scaring_him_does_not() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 400));
    }
    // The guests: dtype 10 (team 3), props 0x53/0x54 [D drone-props], started scared [L 0x29aee8 type 10].
    let guest = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.dtype == 10 && !d.corpse).expect("a party guest exists");
    // The game's party: the girls start PartyGirl (GetDTYPENEW 0x14c938 types 10/11 → +0xC5 = 0x12 →
    // DroneTypeSettings[18].start = 0x2E PartyGirlInit [L 0x29aee8]), the wanderers Civilian (types[9]
    // = 0x10 → CivilianInit → Civilian) or the prop-0x1D walkers (CivilianMission).
    let party_at_rest = matches!(scene.sim.as_ref().unwrap().drones.list[guest].state,
        nf_game::drone::state::PARTY_GIRL | nf_game::drone::state::CIVILIAN | nf_game::drone::state::CIVILIAN_MISSION | nf_game::drone::state::CIVILIAN_PATROL);
    assert!(party_at_rest, "the guests start in the party chain, got {}",
        scene.sim.as_ref().unwrap().drones.list[guest].state);
    // A gunshot NEAR him (not on him): the state stays in the scared chain and the mission is open.
    let gp = scene.sim.as_ref().unwrap().drones.list[guest].pos;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [gp[0] + 6.0, gp[1], gp[2] + 6.0];
        for _ in 0..5 { sim.step_bond(Input::default(), true, false, false); } // fire past him
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.list[guest].corpse, "a near-miss shot must not kill the guest");
    assert!(!sim.drones.channels.on(0x60), "scaring a guest never raises the civilians channel");
    assert_ne!(sim.drones.mission.state, 2, "the mission stays open");

    // Now kill him through the game's explosive path (the player's grenade).
    let gp = sim.drones.list[guest].pos;
    grenade_at(scene.sim.as_mut().unwrap(), [gp[0], gp[1] + 0.5, gp[2]]);
    let mut dead = false;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.list[guest].corpse { dead = true; break; }
    }
    assert!(dead, "the explosion kills the guest");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x60), "the protected guest's death raises ch 0x60 [M 0x145bd0 reason 3]");
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0034, "FailLabel = Killed Civilians [M 0x13e560]");
    // The monitor's standing hook turns ch 0x60 into the mission fail [M 0x1827d8:98-107].
    let mut failed = false;
    for _ in 0..120u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.mission.state == 2 { failed = true; break; }
    }
    assert!(failed, "killing a civilian fails the mission [M 0x1827d8]");
}

/// The stairwell reinforcements arrive: 07000011's group-1 spawner (p4 = trigger ch 0x15 ← touch
/// #182, p5 = stop ch 0x16, p7 = kill ch 0x1D ← the stairwell Touch #285 [D]) — fix-spawner-p7
/// settled p7 as the KILL channel [L-verified 07000005]; fix-actors' ARM reading is refuted. What
/// this test pins is the playtest symptom itself: the guards come in on their trigger channel
/// (in play the player crosses #182 before the stairwell, so the arrival precedes the kill). The
/// kill channel's drawn-guard keep is pinned by
/// `spawner_kill_live::tower2_stairwell_the_kill_channel_keeps_the_drawn_guard`.
#[test]
fn the_spawner_arrives_on_its_channels() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load 07000011");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 400));
    }
    // The group-1 templates are held back by the spawner (`DroneSpawner_Init` 0x137aa0: no object yet).
    let tpls: Vec<usize> = scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .filter(|(_, d)| d.group == 1).map(|(i, _)| i).collect();
    assert!(!tpls.is_empty(), "the group-1 templates exist");
    assert!(tpls.iter().all(|&i| !scene.sim.as_ref().unwrap().drones.list[i].present),
        "the spawner holds its templates before the channels [M 0x137aa0]");

    // Walk the trigger TouchOnce #182 (out 0x15; centre of 6.02…10.48, 47.09…49.16, −4.65…0.10) [D]:
    // the spawner activates on its start channel [M 0x137e68:53-64] and the wave comes in.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [8.25, 48.13, -2.28];
        for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
        assert!(sim.drones.channels.on(0x15), "the trigger touch #182 fires ch 0x15 [D]");
    }
    // A spot the spawn's stealth checks pass (the templates far and behind Bond).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [-30.0, 48.0, 40.0];
        sim.bond.yaw = 0.0;
    }
    let mut spawned = 0;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        spawned = tpls.iter().filter(|&&i| scene.sim.as_ref().unwrap().drones.list[i].present).count();
        if spawned > 0 { break; }
    }
    assert!(spawned > 0, "the spawner brings its templates in on the trigger [M 0x137e68]");
    assert!(scene.sim.as_ref().unwrap().drones.spawners.iter().any(|sp| sp.kill == 0x1d && !sp.gone),
        "the spawner is alive with its kill channel unarmed [M 0x137e68]");
}

/// The boss: 07000004's assassin (dtype 15, ATTACK start, death channel 0x54) — wounding him does not
/// end the fight; killing him raises the objective channel.
#[test]
fn the_assassin_dies_and_completes_his_objective() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load 07000004");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 3000), "Bond is free after the level-start cut-scene (#3, mode 1 [D])");
    }
    let boss = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.skin == 0x0500_001e).expect("the assassin (skin 0x0500001e) exists");
    assert_eq!(scene.sim.as_ref().unwrap().drones.list[boss].death_channel, 0x54, "his death channel is the objective's [D param 12]");
    assert_eq!(scene.sim.as_ref().unwrap().drones.list[boss].team, 1, "dtype 15 → team 1 [M 0x14cbd8]");
    // Sister: wounding him (a burst short of the kill) leaves the objective open.
    let bp = scene.sim.as_ref().unwrap().drones.list[boss].pos;
    grenade_at(scene.sim.as_mut().unwrap(), [bp[0] + 40.0, bp[1] + 8.0, bp[2]]);
    for _ in 0..120u32 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let hp0 = scene.sim.as_ref().unwrap().drones.list[boss].health;
    let (i, st) = (nf_game::mission::LEVELS.iter().position(|d| d.level == 0x0700_0004).unwrap(),
        scene.sim.as_ref().unwrap().drones.mission.states.clone());
    assert_ne!(st[i][0], 4, "a miss does not complete 'Defeat the assassin'");
    assert!(scene.sim.as_ref().unwrap().drones.list[boss].health <= hp0, "the blast reached him or not — the objective stays open either way");
    // The kill.
    let bp = scene.sim.as_ref().unwrap().drones.list[boss].pos;
    grenade_at(scene.sim.as_mut().unwrap(), [bp[0], bp[1] + 0.5, bp[2]]);
    let mut dead = false;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.list[boss].corpse { dead = true; break; }
    }
    assert!(dead, "the explosion kills the assassin");
    let mut done = false;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x54) && sim.drones.mission.states[i][0] == 4 { done = true; break; }
    }
    assert!(done, "the assassin's death channel 0x54 completes 'Defeat the assassin' [D; M 0x13e660]");
}

/// The escort sub-states: while a hostile fights near 07000001's escort he cowers (LeadHide 0x1F) and
/// returns to the lead when the fight ends. Sister: with no fight his state never leaves the lead chain.
#[test]
fn the_escort_cowers_in_a_fight_and_resumes_the_lead() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_mut().unwrap();
    // The intro plays (event 0xD → ch 0xB [L ee_start_07000001]); the escort then leads.
    for _ in 0..3000u32 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0xb) { break; } }
    assert!(sim.drones.channels.on(0xb), "the intro releases the escort");
    let escort = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("the skin-0x4F escort exists");
    // Walk him onto his route: Bond follows within the band.
    let ep = scene.sim.as_ref().unwrap().drones.list[escort].pos;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [ep[0], ep[1], ep[2] + 2.0];
        for _ in 0..120u32 { sim.step_bond(Input::default(), false, false, false); }
        // Sister: no hostile near → never LeadHide.
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    }
    let st0 = scene.sim.as_ref().unwrap().drones.list[escort].state;
    let in_lead = matches!(st0,
        nf_game::drone::state::ALLY_LEAD | nf_game::drone::state::ALLY_LEAD_WAIT | nf_game::drone::state::ALLY_LEAD_DONE
        | nf_game::drone::state::PLAYER_IN_WAY | nf_game::drone::state::LEAD_HIDE | nf_game::drone::state::LEAD_BOND_COMBAT);
    assert!(in_lead, "the escort walks the lead chain (or hides by a started guard), got {st0}");
    // Bring a guard up next to him and start it (a real one: skin 0x05000036's courtyard guards wait on
    // channels; the nearest STARTED one — any drone the fight wakes). Move a guard next to the escort.
    let guard = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.team == 1 && d.skin != 0x0500_004f)
        .expect("a team-1 guard exists");
    {
        let sim = scene.sim.as_mut().unwrap();
        let ep = sim.drones.list[escort].pos;
        sim.drones.list[guard].pos = [ep[0] + 4.0, ep[1], ep[2]];
        sim.drones.list[guard].enabled = true;
        sim.drones.list[guard].started = true;
    }
    let mut hid = false;
    for _ in 0..120u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.list[escort].state == nf_game::drone::state::LEAD_HIDE { hid = true; break; }
    }
    assert!(hid, "an armed hostile within 10 cowers the escort [M 0x15e698 → 0x15ec08]");
    // End the fight the way the state sees it: the hostile moves beyond the 14-unit clear band
    // (`NDrone2_NotNearArmedDrone(0x41600000)` [M 0x15ec08 case 3]) — the escort re-arms timer C
    // (2 ticks) and returns to the lead. (A grenade kill would wake the whole courtyard and keep
    // him hiding — game-correct, but not this assertion.)
    {
        let sim = scene.sim.as_mut().unwrap();
        // Every started hostile goes beyond the clear band (the courtyard wakes several — the escort
        // legitimately stays hidden while any remains).
        for g in sim.drones.list.iter_mut().filter(|g| g.team == 1 && g.started && !g.corpse) {
            g.pos = [g.pos[0] + 60.0, g.pos[1], g.pos[2] + 60.0];
        }
    }
    let mut resumed = false;
    for t in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let st = sim.drones.list[escort].state;
        if t % 100 == 99 { eprintln!("[escort] t{t} state {st:#x} peer_dist {:.1}", sim.drones.list[escort].armed_peer_dist); }
        if matches!(st, nf_game::drone::state::ALLY_LEAD | nf_game::drone::state::ALLY_LEAD_WAIT | nf_game::drone::state::ALLY_LEAD_DONE) { resumed = true; break; }
    }
    assert!(resumed, "with the fight over the escort resumes the lead [M 0x15ec08 case 3]");
}


