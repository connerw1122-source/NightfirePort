//! walkreal-g: 07000007 (Mayhew C, the party) walked with the player's own movement — no
//! teleports after the spawn. Route (docs/levels/07000007.md):
//!
//! 1. the party start #0 (33.4, 44.5, −70.6) [D placements; the loadout is the FISTS with the
//!    weapon cycle locked while ch 0x48 is off (M Player_Weapon 0x1a24c8:242-275)]; the spawn
//!    plate TouchOnce #411 (33.6, 43.8, −70.6) fires ch 0x47 under Bond → objective 1
//!    "Rendezvous with undercover agents…" reveals [L ee_start_05 record 2];
//! 2. through the party to the hall walk-in TouchOnce #412 (94.3, 43.7, −45.9) → ch 0x48: the
//!    weapon lock lifts, the P2K auto-draws (Upgrade_Weapon(6) [M Player_Weapon 0x1a24c8:247]),
//!    objective 1 completes, and SP #420 (0x06000073, the control-taking cut-scene) plays;
//! 3. upstairs by the west stairs (nav nodes 41→42→43: (83.1,43.4,−61.1)→(83.2,46.4,−70.2)→
//!    (95.6,49.4,−70.6) [D path 0 dump]) to Drake's reveal TouchOnce #356 (box 92.9–98.0,
//!    49.0–53.8, −79.8–−78.9 [D]) → ch 0x15 → SP #419 (0x06000072, the party fly-through where
//!    Bond first sees Drake, 3000 ticks [M levels.md §5.1]) plays;
//! 4. east along the upper floor to the exit LOAD_LEVEL #418 (135.9, 50.1, −76.5 [D]) and E:
//!    the game's own exit to 07000008 (the exit's gate is 0 — it fires unconditionally [D §5]);
//! 5. the carried load into 07000008 (RamSave + mission states [M 0x1b2318:111-116]), the
//!    gondola intro cut-scene skipped with the game's own skip action, then the intro plate
//!    TouchOnce #237 (139.2, 49.5, −81.2 [D 07000008 placements]) fires ch 0x49 → objective 2
//!    "Spy on secret meeting" completes on the next level, exactly as the mission record's
//!    reveal chain implies.
//!
//! The party rules along the way: Bond is unarmed until ch 0x48 (the bouncers stay calm [M
//! CivilianGuard 0x1603a8 case 3]), and after the auto-draw the walk cycles BACK to the fists
//! (the player's own weapon cycle) so the armed-Bond scare [M 0x160140:44-76] and the bouncer
//! attack range [M NearArmedPerson 0x1787c8] never trip while he crosses the party.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};
use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The mission objective states for 07000007 (LEVELS[2] [M mission.rs LEVELS table]).
fn obj07(w: &Walker) -> (u32, u32) {
    let m = &w.scene.sim.as_ref().unwrap().drones.mission;
    (m.states[2][0], m.states[2][1])
}

/// The full walk: start → the hall (ch 0x48, the control-taking scene) → Drake's reveal
/// (ch 0x15, the fly-through) → the exit → 07000008's intro plate (ch 0x49, objective 2).
#[test]
fn walk_the_party_start_to_finish() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000007.bin", false).expect("load 07000007"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 33.4).abs() < 2.0 && (p[2] - -70.6).abs() < 2.0, "the party start #0 [D], got {p:?}");
        assert_eq!(sim.gun.switch_request, 1, "the loadout request is the fists [M 0x1a7ef8 case 0x7000007]");
    }
    w.wait_enabled(3000, "the party start");
    w.wait(60);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, 1, "Bond holds the fists once the draw settles [M 0x1a7ef8]");
        assert!(sim.drones.channels.on(0x47), "the spawn plate #411 fired ch 0x47 [D placements]");
        assert_eq!(obj07(&w).0, 3, "objective 1 announced by ch 0x47 (state 3 = announced [M mission.rs states])");
    }

    // ── the hall walk-in: ch 0x48 + the control-taking scene ────────────────────────────────────
    // The level's own AI graph splits into components (the guests' walk roads; [D the path-0 link
    // dump: {0..7} the west end, {8..17}, {18,19}, {20,21}, {22..25}, {26..44+}, {45..47}]), so the
    // walk bridges them with the nodes' own positions — the same floor the plates sit on. Walking
    // through them fires the guests' group-enable plates (ch 6 #476, ch 0x17 #415, ch 0x16 #417
    // [D census §4]) — the party wakes group by group on the way.
    for (name, at) in [
        ("west aisle south", [35.7f32, 43.6, -70.6]),
        ("west aisle north", [42.6, 43.6, -70.5]),
        ("the west wall run", [42.6, 43.6, -45.0]),
        ("the north door", [49.4, 43.6, -44.7]),
        ("the north row west", [62.7, 43.6, -44.3]),
        ("the north row east", [76.5, 43.6, -44.9]),
        ("the hall west mouth", [76.6, 43.6, -50.7]),
        ("the hall floor", [83.1, 43.6, -50.5]),
        ("the hall east", [93.3, 43.6, -50.4]),
    ] {
        w.walk_nav_to(at, name, 1.5);
    }
    eprintln!("[07] at the hall mouth tick {} bond {:?}", w.total_ticks, w.scene.sim.as_ref().unwrap().bond.pos);
    // The hall's E-door (94.9, −47.2 [D door 2 of the free set]) stands between the mouth and the
    // plate: E it open (the player's own use), then the manual steps below cross into the plate
    // with the per-tick scene poll.
    w.walk_to(Wp { at: [94.6, 43.6, -47.4], act: Act::Use { use_ticks: 240 }, arrive: 0.7 }, "E the hall door");
    {
        let target = [94.3f32, 43.6, -45.9];
        let mut still = 0u32;
        let mut last = [0.0f32, 0.0];
        for t in 0..1200u32 {
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            if sim.drones.channels.on(0x48) { break; }
            // A wedged stretch means the hall door swung shut again: pulse E (the player's own use)
            // — Door_Activate is a TOGGLE [M walker.rs's door pass], so only pulse a CLOSED door.
            let door_closed = w.scene.sim.as_ref().unwrap().drones.doors.list.iter()
                .filter(|d| walker::dxz(d.pos, p) < 3.0)
                .any(|d| d.state & 0xc == 0);
            let use_key = if (p[0] - last[0]).abs() < 0.01 && (p[2] - last[1]).abs() < 0.01 {
                still += 1; door_closed && still % 30 == 0
            } else { still = 0; last = [p[0], p[2]]; false };
            let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let input = Input { turn: err.clamp(-0.08, 0.08), forward: if err.abs() > 1.0 { 0.0 } else { 1.0 }, ..Default::default() };
            w.tick(input, use_key);
            if t % 300 == 0 {
                let sim = w.scene.sim.as_ref().unwrap();
                eprintln!("[07 hall t{t}] bond {:?} 0x48 {}", sim.bond.pos, sim.drones.channels.on(0x48) as u8);
            }
        }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x48), "the hall walk-in plate #412 fired ch 0x48 [D placements]");
    }
    // The mission monitor folds the objective on its own tick [M mission.update].
    let mut done = 3u32;
    for _ in 0..120 {
        done = obj07(&w).0;
        if done == 4 { break; }
        w.tick(Input::default(), false);
    }
    assert_eq!(done, 4, "objective 1 completes on ch 0x48 [L ee_start_05 record 2]");
    // The scene holds Bond (the game's own disable); the walker's tick carries the game's skip
    // action [M nis.rs skip_request = Input_Action(−1, 0x19, 1)], then Bond hands back at the
    // stream-6 entity's key-498 spot [D fix_cutscene_actors_live].
    w.wait_enabled(1200, "the control-taking scene");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.players.iter()
            .any(|p| p.scripts.contains(&0x0600_0073) && p.state >= 3),
            "the control-taking cut-scene (0x06000073) ran to its end through the real trigger [M levels.md §5.1]");
        let p = sim.bond.pos;
        assert!((p[0] - 91.52).abs() < 1.0 && (p[1] - 43.85).abs() < 1.0 && (p[2] + 31.09).abs() < 1.0,
            "Bond hands back at the stream-6 entity key 498 [D fix_cutscene_actors_live], got {p:?}");
    }
    // The lock LIFTED with ch 0x48: a cycle request survives weapon_lock_input now [M 0x1a24c8:243-246]
    // (level_carry_live pins the lock itself; the view loop owns the rising-edge auto-draw
    // Upgrade_Weapon(6) — the walk harness steps the Sim directly, so Bond keeps the fists he
    // holds, which is also the cover-safe play across the rest of the party).
    {
        let s = w.scene.sim.as_ref().unwrap();
        let mut input = Input { weapon_cycle: 1, ..Default::default() };
        nf_game::player::weapon_lock_input(s.level, s.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL), &mut input);
        assert_eq!(input.weapon_cycle, 1, "the cycle survives once ch 0x48 is on [M 0x1a24c8:243-246]");
        assert_eq!(s.gun.weapon, 1, "Bond stays on the fists across the party");
    }

    // ── the west stairs up to the balcony ───────────────────────────────────────────────────────
    // Bond was handed back by the scene (the stream-6 entity, key 498 [D fix_cutscene_actors_live]).
    // The hall floor's own nodes thread the furniture ([D path-0 dump]: 22 (95.2,−44.6) → 21
    // (93.3,−50.4) → 20 (83.1,−50.5) → 26 (83.1,−56.2) → 41 (83.1,−61.1) the stair foot); the
    // direct diagonal wedges on the seating (the lenient give-up's geometry).
    for (name, at) in [
        ("the hall south node", [95.2f32, 43.6, -44.6]),
        ("the hall centre node", [93.3, 43.6, -50.4]),
        ("the hall west node", [83.1, 43.6, -50.5]),
        ("the hall south-west node", [83.1, 43.6, -56.2]),
        ("to the west stair foot", [83.1, 43.6, -61.1]),
    ] {
        w.walk_nav_to(at, name, 1.4);
    }
    w.climb_stairs([83.2, 46.4, -70.2], "the west stairs", 1.6, 4200);
    w.climb_stairs([95.6, 49.4, -70.6], "the stair head", 1.6, 4200);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[07] upstairs at tick {} bond {:?}", w.total_ticks, sim.bond.pos);
        assert!(sim.bond.pos[1] > 48.0, "Bond must stand on the upper floor (y {:.2})", sim.bond.pos[1]);
    }

    // ── Drake's reveal: ch 0x15 + the fly-through ───────────────────────────────────────────────
    w.walk_nav_to([95.5, 49.4, -76.5], "the balcony north", 1.5);
    {
        let target = [95.4f32, 49.4, -79.4]; // inside #356's XZ (92.9–98.0, −79.8–−78.9 [D])
        for t in 0..1500u32 {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x15) { break; }
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let input = Input { turn: err.clamp(-0.08, 0.08), forward: if err.abs() > 1.0 { 0.0 } else { 1.0 }, ..Default::default() };
            w.tick(input, false);
            if t % 300 == 0 { eprintln!("[07 reveal t{t}] bond {:?} 0x15 {}", w.scene.sim.as_ref().unwrap().bond.pos,
                w.scene.sim.as_ref().unwrap().drones.channels.on(0x15) as u8); }
        }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x15), "Drake's reveal plate #356 fired ch 0x15 [D placements]");
    }
    // The fly-through holds Bond; the walker's tick carries the game's skip action until it ends.
    w.wait_enabled(4200, "the Drake fly-through");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.players.iter()
            .any(|p| p.scripts.contains(&0x0600_0072) && p.state >= 3),
            "the Drake fly-through (0x06000072) ran through the real trigger [M levels.md §5.1]");
    }

    // ── the exit: the east south wing, E on LOAD_LEVEL #418 ─────────────────────────────────────
    // The upper floor splits into four rooms ([D path-0 nodes + the walk's wedges]): the west
    // north balcony (x 92-98) and the west south wing behind door 4 (94.8,−73.2); the east north
    // room and the east south wing behind door 6 (132.4,−73.2). The wings meet only on the GROUND
    // floor: down the west stairs, east along the north row ([D nodes 30 (105,−55.9) → 38
    // (135.3,−56) → 48 (145.1,−61)]), up the east stairs ([D 48 (145.1,43.4,−61) → 62
    // (144,46.4,−70.2) → 50/63 (132.9/131.8, 49.4, −70.5)]), then south through door 6 to the
    // exit's wing ([D 52 (133.2,−76.1)]).
    w.walk_nav_to([95.6, 49.4, -70.6], "back on the west balcony", 1.4);
    w.climb_stairs([83.2, 46.4, -70.2], "down the west stairs", 1.6, 4200);
    for (name, at) in [
        ("the west stair foot", [83.1f32, 43.4, -61.1]),
        ("the north row mid", [98.6, 43.6, -55.8]),
        ("the north row east 1", [105.0, 43.6, -55.9]),
        ("the north row east 2", [111.0, 43.6, -61.8]),
        ("the north row east 3", [126.5, 43.6, -55.8]),
        ("the east stair foot", [145.1, 43.4, -61.0]),
    ] {
        w.walk_nav_to(at, name, 1.5);
    }
    w.climb_stairs([144.0, 46.4, -70.2], "up the east stairs", 1.6, 4200);
    w.climb_stairs([132.9, 49.4, -70.5], "the east stair head", 1.6, 4200);
    w.walk_to(Wp { at: [132.4, 49.4, -73.6], act: Act::Use { use_ticks: 240 }, arrive: 1.4 }, "south through door 6");
    w.walk_nav_to([133.2, 49.4, -76.1], "the east wing south node", 1.4);
    let exit = w.scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL)
        .map(|t| t.centre()).expect("the exit LOAD_LEVEL #418");
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 120 }, arrive: 1.6 }, "E the exit");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.level_exit, Some(0x0700_0008), "the exit #418 loads 07000008 unconditionally [D §5]");
    }
    eprintln!("[07] the party walked start to exit in {} ticks; falls {:?}", w.total_ticks, w.report.falls);

    // ── the next level: the carried load, the intro plate, objective 2 ─────────────────────────
    let carry = {
        let sim = w.scene.sim.as_ref().unwrap();
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    let mut n = Walker::new(nf_view::load_with_carry(&iso, "07000008.bin", false, Some(&carry)).expect("carry 07000008"));
    n.wait_enabled(4000, "the gondola intro");
    n.wait(60);
    // Walk to 07000008's intro plate #237 (139.2, 49.5, −81.2 [D 07000008 placements]) — ch 0x49.
    n.walk_nav_to([139.2, 49.5, -81.2], "to the intro plate", 1.4);
    let mut ch49 = false;
    for _ in 0..600 {
        if n.scene.sim.as_ref().unwrap().drones.channels.on(0x49) { ch49 = true; break; }
        n.tick(Input::default(), false);
    }
    assert!(ch49, "07000008's intro plate #237 fired ch 0x49 [D 07000008 placements]");
    let m = &n.scene.sim.as_ref().unwrap().drones.mission;
    assert_eq!(m.states[2][1], 4, "objective 2 'Spy on secret meeting' completes on the next level's plate [L ee_start_05 record 2]");
    eprintln!("[07] objectives after the carry: 07 [{}] [{}], done in {} more ticks",
        m.states[2][0], m.states[2][1], n.total_ticks);
}

/// The party's zoning rules, walked: the guests stand in groups until their plate fires, then the
/// prop-0x1D walkers take their kind-8 mission routes — the user report's "the guests' party rules"
/// and the hall walkers' walk (the six kind-8 paths [D the level's nav dump: paths 2-7, kind 0x8]).
/// [M] the chain: `NDrone2_DSTATE_Global` 0x15b830 ENTER → WaitSwitch (switch_channel param 3) →
/// `DSTATE_WaitSwitch` 0x15b960 case 3 (the channel on) → the start state → `PartyGirlInit`
/// 0x1728c0 / `CivilianInit` 0x172730 ENTER: prop 0x1D → `AssignAIPath(8)` 0x154190 →
/// `CivilianMission` 0x15d860 walks `UpdateMissionRoute` 0x154a80. The ch-6 plate is TouchOnce
/// #476 (49.26, 45.4, −46.71 [D]); the walk also holds the GUARDS' rule — no bouncer enters combat
/// while Bond crosses unarmed [M CivilianGuard 0x1603a8 case 3: attack only armed+seen].
#[test]
fn the_group_plate_wakes_the_walkers_and_the_guards_stay_calm() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000007.bin", false).expect("load 07000007"));
    w.wait_enabled(3000, "the party start");
    w.wait(60);
    // The ch-6 waiters before the plate: WAIT_SWITCH (state 1), standing.
    let waiters = |w: &Walker| -> Vec<(usize, [f32; 3], u8, bool)> {
        w.scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
            .filter(|(_, d)| d.present && !d.corpse && d.state == crate_wait_switch()
                && (d.switch_channel == 6))
            .map(|(i, d)| (i, d.pos, d.switch_channel, d.enabled)).collect()
    };
    fn crate_wait_switch() -> u8 { nf_game::drone::state::WAIT_SWITCH }
    let before = waiters(&w);
    eprintln!("[07 zone] ch-6 waiters before: {:?}", before.iter().map(|(i, p, _, _)| (*i, p[0], p[2])).collect::<Vec<_>>());
    assert!(before.len() >= 4, "the party's ch-6 group stands in WaitSwitch before their plate [D census §4], got {before:?}");

    // Walk to the plate through the real path (the same first anchors the full walk uses; the
    // plate sits 2 units south of the north-door node, so the last stretch steps into it).
    for (name, at) in [
        ("west aisle south", [35.7f32, 43.6, -70.6]),
        ("west aisle north", [42.6, 43.6, -70.5]),
        ("the west wall run", [42.6, 43.6, -45.0]),
        ("the north door", [49.4, 43.6, -44.7]),
    ] {
        w.walk_nav_to(at, name, 1.3);
    }
    {
        let target = [49.26f32, 43.6, -46.71];
        let mut still = 0u32;
        let mut last = [0.0f32, 0.0];
        for t in 0..900u32 {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(6) { break; }
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            let door_closed = sim.drones.doors.list.iter()
                .filter(|d| walker::dxz(d.pos, p) < 3.0).any(|d| d.state & 0xc == 0);
            let use_key = if (p[0] - last[0]).abs() < 0.01 && (p[2] - last[1]).abs() < 0.01 {
                still += 1; door_closed && still % 30 == 0
            } else { still = 0; last = [p[0], p[2]]; false };
            let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let input = Input { turn: err.clamp(-0.08, 0.08), forward: if err.abs() > 1.0 { 0.0 } else { 1.0 }, ..Default::default() };
            w.tick(input, use_key);
            if t % 300 == 0 { eprintln!("[07 zone t{t}] bond {:?} ch6 {}", w.scene.sim.as_ref().unwrap().bond.pos,
                w.scene.sim.as_ref().unwrap().drones.channels.on(6) as u8); }
        }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(6), "plate #476 fired ch 6 as Bond crossed [D census §4]");
    }
    // The guards' rule: Bond crossed half the party unarmed — no combat anywhere.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let combat: Vec<usize> = sim.drones.list.iter().enumerate()
            .filter(|(_, d)| d.present && !d.corpse && matches!(d.state, 0x56..=0x7e | 0x85..=0x99))
            .map(|(i, _)| i).collect();
        assert!(combat.is_empty(), "no bouncer entered combat against an unarmed Bond [M 0x1603a8 case 3], got {combat:?}");
    }
    // The walkers wake: out of WaitSwitch, onto their routes, MOVING within 20 s of sim.
    w.wait(60);
    let mut moved = Vec::new();
    let awake: Vec<usize> = w.scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && !d.corpse && d.switch_channel == 6 && d.state != crate_wait_switch())
        .map(|(i, _)| i).collect();
    eprintln!("[07 zone] awake after ch 6: {:?} (states {:?})", awake,
        awake.iter().map(|&i| w.scene.sim.as_ref().unwrap().drones.list[i].state).collect::<Vec<_>>());
    let starts: Vec<(usize, [f32; 3])> = awake.iter()
        .map(|&i| (i, w.scene.sim.as_ref().unwrap().drones.list[i].pos)).collect();
    for _ in 0..1200 {
        w.tick(Input::default(), false);
    }
    for &(i, p0) in &starts {
        let p1 = w.scene.sim.as_ref().unwrap().drones.list[i].pos;
        let d = ((p0[0] - p1[0]).powi(2) + (p0[2] - p1[2]).powi(2)).sqrt();
        if d > 0.5 { moved.push((i, d)); }
        eprintln!("[07 zone] waiter {i} moved {d:.2} state {:#x}", w.scene.sim.as_ref().unwrap().drones.list[i].state);
    }
    assert!(!moved.is_empty(), "the ch-6 walkers walk their kind-8 routes once the plate fires [M 0x1728c0/0x172730 → 0x154190 → 0x15d860], moved {moved:?}");
}
