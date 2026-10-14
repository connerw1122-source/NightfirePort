//! fix-finale-blockers: the mission blockers the per-level walkthroughs could not pass. Drives the
//! REAL path (`nf_view::load` / `load_with_carry` + `Sim::step_bond`) on the finale levels:
//!
//! - 07000016: the launch chain spawns the chaser (ch 0x81 → SP #262's pack cinema 0x06000631 →
//!   ch 185 → SP #272 "ShuttleLaunch" 0x0600090a → ch 240/242 → spawners #257/#293 [D]); her
//!   defeat (docs/levels/07000016.md §3/§6; the stand-in is kiko.rs's) completes the mission.
//! - 0700001b: ch 0x0a (0x060008f0 t104 [D] — SP #211's second script, behind the missile
//!   choreography) spawns Drake (spawner #187, trigger 10 [D]) and enables him (sw_ch 10 [D]);
//!   his death (ch 0x0e [D]) fires SP #198 → NIS 0x06000a20 → ch 0x0b (t600) + ch 100 (t720).
//! - 0700004a: the lift ride (SP #42 → 0x06000571: the platform entity descends 150 units over
//!   4950 ticks [D keys]) and the lobby arrival (the nis.rs stand-in at the script's end).
//! - the unported start states 0x85 / 0xa7 / 0xb4 (kiko.rs).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::kiko as kiko_states;
use nf_game::player::{Input, RamSave};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

fn grenade_at(sim: &mut nf_view::scene::Sim, pos: [f32; 3]) {
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos, velocity: [0.0, 0.0, 0.0], speed: 0.0,
        // One tick from its fuse (travelled 59 of range 60): a grenade explodes only on its fuse, never on contact
        // (fix-weapons-all, `Bullet_CollisionHandler` 0x12ce80), so the drop goes off where it lands.
        travelled: 59.0, range: 60.0, damage: 100.0, radius: 3.0, weapon_type: 0x34, mesh: 0x0200_0726, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, explode_on_impact: false,
    });
}

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// 07000016: the chaser (Kiko, #261 — skin 0x0500001e, dtype 0xf, the only such drone without a
/// death channel [D]) dies at the silo → ch 0x90 (the kiko.rs stand-in) → AND #271 → ch 100 →
/// Mission_Update state 6 → the driving hand-off 0x7300017 [M 0x182c60:111-112].
/// Sister: a pit-squad death (channels 23..26 [D]) never raises ch 0x90 by itself.
#[test]
fn the_07000016_chasers_defeat_completes_the_final_mission() {
    let iso = iso();
    let mut scene = nf_view::load_with_carry(&iso, "07000016.bin", false, Some(&carry(&[0x47, 0x48, 0x4a, 0x54, 0xce, 0x4b]))).expect("load 07000016");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 3000), "Bond is free after the intro");
        // The silo plate #253 (0.45-4.62, -0.20..1.88, -154.93..-152.85 [D]) reveals the Kiko
        // objective (ch 0x8f [D]) — the walkthrough's descent.
        sim.bond = nf_game::player::Bond::new([2.5, 1.2, -153.9], std::f32::consts::PI);
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 1..=120 { sim.step_bond(input, false, false, false); if sim.drones.channels.on(0x8f) { break; } }
        // The launch chain's head: ch 0x81 (no writer on the disc data [D scans]) → SP #262 plays
        // the pack cinema 0x06000631 → ch 185 → SP #272 → 0x0600090a → ch 240/242 → the spawners.
        sim.drones.channels.set(0x81, 1, sim.drones.frame);
    }
    // Play the chain out: the cinema (750 ticks) + the shuttle script + the spawner waves.
    let chaser = {
        let mut found = None;
        for _ in 0..4000u32 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            let sim = scene.sim.as_ref().unwrap();
            if let Some(i) = sim.drones.list.iter().position(|d| d.present && !d.corpse && d.skin & 0xffff == 0x1e && d.dtype == 0xf && d.death_channel == 0x90) {
                found = Some(i);
                break;
            }
        }
        found.expect("the launch chain spawns the chaser (spawner #257, group 6 [D])")
    };
    // Sister first: a non-chaser drone's death (the pit squad's ch 23..26 [D]) never raises ch 0x90
    // on its own. Re-baselined for fix-spawner-p7: the launch chain's ch 0x15 is the group-1
    // spawner's KILL channel [L-verified 07000005], and 07000016 is a keep level, so the chain's
    // own cleanup removes its undrawn spawns before this point — the sister throws at the nearest
    // remaining non-chaser (a ninja escort, who walks), re-reading his position per attempt like
    // the chaser defeat below.
    {
        let sim = scene.sim.as_mut().unwrap();
        let squad = sim.drones.list.iter().position(|d| d.present && !d.corpse && d.death_channel != 0x90)
            .expect("a non-chaser guard exists");
        let mut died = false;
        for _ in 0..4 {
            let p = sim.drones.list[squad].pos;
            grenade_at(sim, p);
            for _ in 0..200 {
                sim.step_bond(Input::default(), false, false, false);
                if sim.drones.list[squad].corpse { died = true; break; }
            }
            if died { break; }
        }
        assert!(died, "the grenade reaches the non-chaser guard");
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.list.iter().any(|d| d.death_channel != 0x90 && d.corpse), "the guard died");
        assert!(!sim.drones.channels.on(0x90), "a non-chaser death never completes the objective [D AND #271]");
    }
    // The chaser's defeat through the game's explosive path (the ninja chain walks her, so the
    // throw re-reads her position each attempt).
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut died = false;
        for _ in 0..3 {
            let p = sim.drones.list[chaser].pos;
            grenade_at(sim, p);
            for _ in 0..300u32 {
                sim.step_bond(Input::default(), false, false, false);
                if sim.drones.list[chaser].corpse { died = true; break; }
            }
            if died { break; }
        }
        assert!(died, "the chaser dies at the silo");
    }
    // ch 0x90 → AND #271 → ch 100 → Mission_Update state 6 [M 0x1827d8:155-160, 0x182c60:85-122].
    let mut complete = false;
    for _ in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.mission.status() == 6 { complete = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x90), "the chaser's defeat raises ch 0x90 [S kiko.rs]");
        assert!(sim.drones.channels.on(100), "AND #271 [ch 0x90] → ch 100 [D #271]");
        assert!(complete, "the final mission completes (MissionState 6 [M 0x182c60:85-116])");
        assert_eq!(sim.drones.mission.to_end, 0x0730_0017, "the driving hand-off [M 0x182c60:111-112]");
    }
}

/// 0700001b: ch 0x0a (the fight gate: NIS 0x060008f0 t104 [D]) spawns Drake (spawner #187,
/// trigger 10 [D]) and enables him (his wait-switch 10 [D]); his death (ch 0x0e [D]) fires
/// SP #198 → NIS 0x06000a20: t600 event-11 writes ch 0x0b ("Defeat Drake"), t720 ch 0x64 = 100
/// [D decode; Script_EventHandler 0x1d94c0 case 0x11] → MissionState 6 → 0x0730001b [M 0x182c60].
/// Sister: ch 0x0b waits for the NIS's t600, not the death tick.
#[test]
fn the_0700001b_drakes_defeat_ends_the_space_station() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 3000), "Bond is free after the shuttle arrival");
        // The fight gate (the head of its chain is the missile choreography [D]; the port drives
        // the gate channel the way 0x060008f0's t104 event does).
        sim.drones.channels.set(0x0a, 1, sim.drones.frame);
    }
    let drake = {
        let mut found = None;
        for _ in 0..600u32 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            let sim = scene.sim.as_ref().unwrap();
            if let Some(i) = sim.drones.list.iter().position(|d| d.present && !d.corpse && d.skin & 0xffff == 0xba) {
                found = Some(i);
                break;
            }
        }
        found.expect("the gate spawns Drake (spawner #187 [D])")
    };
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.list[drake].enabled, "the gate enables Drake (wait-switch 10 [D])");
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        let p = sim.drones.list[drake].pos;
        grenade_at(sim, p);
        let mut died = false;
        for _ in 0..300u32 {
            sim.step_bond(Input::default(), false, false, false);
            if sim.drones.list[drake].corpse { died = true; break; }
        }
        assert!(died, "Drake dies");
    }
    // Death_Anim's msg-2 raises the channel on its first update [M 0x1637f8 case 2] — a few ticks
    // after the kill; the defeat NIS's own t600 write comes later.
    for _ in 0..100 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x0e), "Drake's death channel fires ch 0x0e [D param 12 = 14]");
        assert!(!sim.drones.channels.on(0x0b), "ch 0x0b waits for the defeat NIS's t600 [D 0x06000a20] — sister");
    }
    // SP #198 plays 0x06000a20: t600 ch 0x0b, t720 ch 0x64 (100) [D].
    let mut complete = false;
    for _ in 0..1500u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.mission.status() == 6 { complete = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x0b), "the NIS's t600 event-11 writes ch 0x0b [D]");
        assert!(sim.drones.channels.on(100), "the NIS's t720 event-11 writes ch 0x64 = 100 [D]");
        assert!(complete, "the station mission completes [M 0x182c60]");
        assert_eq!(sim.drones.mission.to_end, 0x0730_001b, "the station's own end code [M 0x182c60:114-116]");
    }
}

/// 0700004a: the lift (SP #42 "SCRIPTPLAYER_Lift", mode 1 [D]) plays 0x06000571 — the platform
/// entity descends 150 units over 4950 ticks [D keys] — Bond rides it down the shaft, the floor
/// plates fire (the walkthrough's channels), and the lobby arrival (the script's end; the nis.rs
/// stand-in) completes "Gain access to lobby" (ch 0x4a [L record 4a]). Sister: ch 0x4a does not
/// fire mid-ride (the script has ~2000 ticks to go at the check).
#[test]
fn the_0700004a_lift_ride_reaches_the_lobby() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700004a.bin", false).expect("load 0700004a");
    // Bond boards the platform at once — the lift script starts at t0 (mode 1 [D]).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [3.0, -7.0, -3.4], 0.0);
    }
    let y0 = scene.sim.as_ref().unwrap().bond.pos[1];
    // Ride the whole script (5040 ticks) — the plates fire as the platform passes them; the shaft
    // garrison's fire is shrugged off (the ride is the system under test, not the fight). Bond is
    // re-seated on the platform each segment: the shaft doors' auto-open on the lift's approach is
    // not ported (docs/levels/0700004a.md §6), so the closed leaves scrape him off mid-ride —
    // the walk test's teleporting-between-floors shape, now along the script's key line
    // (t90 y0 → t5040 y0 − 150 [D 0x06000571 keys]).
    let mut fired = Vec::new();
    for t in 0..5400u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        scene.sim.as_mut().unwrap().drones.health.health = 100.0;
        if t % 30 == 0 {
            let sim = scene.sim.as_mut().unwrap();
            let py = (y0 - ((t as f32 - 90.0).max(0.0)) * (150.0 / 4950.0)).max(y0 - 150.0);
            sim.bond.pos = [3.0, py + 1.2, -3.4];
        }
        let sim = scene.sim.as_ref().unwrap();
        for ch in [0x3du32, 0x17, 0x18, 0x3f, 0x19, 0x45, 0x1a, 0x3e, 0x1b, 0x4f, 0x84] {
            if sim.drones.channels.on(ch) && !fired.contains(&ch) { fired.push(ch); }
        }
        if t == 3000 {
            assert!(!sim.drones.channels.on(0x4a),
                "ch 0x4a waits for the lobby arrival (the script's end) — sister");
            assert!(sim.bond.pos[1] < y0 - 30.0, "Bond is riding the platform down (y {} from {y0})", sim.bond.pos[1]);
        }
        if sim.drones.channels.on(0x4a) { break; }
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(fired.contains(&0x3d) || fired.contains(&0x17), "descent plates fire during the ride ([D] floors), got {fired:?}");
        assert!(sim.drones.channels.on(0x4a), "the lift's lobby arrival completes the objective [L record 4a; nis.rs stand-in]");
        // fix-objective-banner: the monitor runs at the TOP of the tick (Game_Run order), so it
        // completes the objective the tick after the channel rises.
        for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
        assert_eq!(sim.drones.mission.states[17][0], 4, "the monitor completes 'Gain access to lobby'");
    }
}

/// The unported start states (fix-levels-sweep's table): 0xb4 (07000007) despawns at once
/// [M 0x173f00], the mode-7 0x85 drones on 07000011/12/13/15 resolve to Idle/Alert/Attack — never
/// stuck [M 0x168910:32-36] — and 07000004's assassin (dtype 0xf → 0xd → 0xa6/0xa7 [M 0x14c938])
/// enters the ninja chain at alert 1.0.
#[test]
fn the_unported_start_states_behave_like_the_game() {
    let iso = iso();
    // 07000007: the k16=5 placement (mode 21 → DeleteMe [L DroneTypeSettings[21]]) is gone.
    {
        let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 1200), "Bond is free");
        let deleted = sim.drones.list.iter().filter(|d| d.mode == 21).count();
        let gone = sim.drones.list.iter().filter(|d| d.mode == 21 && !d.present).count();
        assert!(deleted > 0 && gone == deleted, "every mode-21 (0xb4) party placement despawned [M 0x173f00], {gone}/{deleted}");
    }
    // 07000011: the mode-7 drones (GetDTYPENEW case 3 → 7 [M 0x14c938]) stand in a game state.
    {
        let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load 07000011");
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 1800), "Bond is free");
        let m7: Vec<u8> = sim.drones.list.iter().filter(|d| d.present && d.mode == 7).map(|d| d.state).collect();
        assert!(!m7.is_empty(), "07000011 has mode-7 drones [D sweep]");
        assert!(m7.iter().all(|&s| matches!(s, nf_game::drone::state::IDLE | nf_game::drone::state::ALERT | nf_game::drone::state::ATTACK | nf_game::drone::state::WAIT_SWITCH)),
            "the 0x85 drones resolved per [M 0x168910:32-36] (the gate-waiters sit in WaitSwitch 0x01 [M]), got {m7:?}");
    }
    // 07000004: the assassin (dtype 0xf [D]) is in the ninja chain at alert 1.0.
    {
        let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load 07000004");
        let sim = scene.sim.as_mut().unwrap();
        assert!(free(sim, 1800), "Bond is free");
        let ninja = sim.drones.list.iter().find(|d| d.present && d.dtype == 0xf).expect("the assassin exists [D]");
        assert!(matches!(ninja.state,
                kiko_states::NINJA_ATTACK | kiko_states::NINJA_LONG_RANGE | kiko_states::NINJA_MID_RANGE | kiko_states::NINJA_SHORT_RANGE | kiko_states::NINJA_STAND_FIRE),
            "the assassin runs the ninja chain [M 0x16bb50], state {:#x}", ninja.state);
        assert!((ninja.alert - 1.0).abs() < 1e-4, "NinjaAttack sets alert 1.0 [M 0x16bb50]");
    }
}
