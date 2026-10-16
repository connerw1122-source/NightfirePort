//! escort-01: 07000001's escort walked from the meet-up to the exit with BOND AT WALKING SPEED
//! (the analog half-press, `forward ≤ 0.55` — the walkreal-a harness runs full-stick). The two
//! game rules under test:
//!
//! 1. **The follow distance / catch-up** [M 0x15e698 case 3 + 0x15eda8 case 3 +
//!    `DroneMove_ObjectMissionPathDistance` 0x153f50]: while Mayhew LEADS he keeps within the
//!    game's distance — Bond more than 9 route-units behind (path < −9.0) parks him in
//!    AllyLeadWait, and he resumes when Bond is back within 7 (path > −7.0). The signed path
//!    value is the drone's `mission_path_dist` (the game's SM+0x20 scratch).
//! 2. **The exit's condition**: the transition is NOT gated on Mayhew reaching the door — the
//!    exit doorway (gallery_secretdoor #224/#225) has no channels at all and Bond can E-open it
//!    with Mayhew anywhere [D docs/atlas/door.md]; what fires 07000002 is the AND plate (Touch
//!    out 0x10, mask 0x18, filter player 0xE + skin 0x4F): BOTH bodies inside the volume → ch
//!    0x10 → the LOAD_LEVEL gate [M Trigger_Hit 0x1b1fc8 / Trigger_Update case 10 0x1b1a78].

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

use nf_game::drone::state;

/// The states the escort machine legitimately parks in while Bond walks behind him
/// ([M 0x15e698/0x15ea28/0x15ec08/0x15efc8/0x15f120/0x15f2e8] + the route's own OpenDoor 0x81).
const ESCORT_STATES: [u8; 9] = [
    state::ALLY_LEAD_INIT, state::ALLY_LEAD, state::ALLY_LEAD_WAIT, state::PLAYER_IN_WAY,
    state::LEAD_HIDE, state::LEAD_MISSION_WAIT, state::LEAD_BOND_COMBAT, state::ALLY_LEAD_DONE,
    state::OPEN_DOOR,
];

#[test]
fn escort_at_walking_speed_keeps_the_games_distance_and_the_exit_opens() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000001.bin", false).expect("load 07000001"));
    // The intro cut-scene writes ch 0xB (the contact's enable) ~tick 2119; its tail re-places
    // Bond — the same wait-out walkreal-a runs.
    let mut enabled_at = None;
    for t in 0..4000u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) { enabled_at = Some(t); break; }
    }
    enabled_at.expect("the intro must write ch 0xB");
    w.wait_enabled(6000, "the intro tail");
    w.wait(300);

    // Mayhew (skin 0x4F) and the AND plate.
    let ci = w.scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("the contact");
    let plate = walker::trigger_centre(w.scene.sim.as_ref().unwrap(), walker::trigger_kind::TOUCH, 0x10).expect("the AND-touch plate");

    // The AllyLead machine hides him while an armed, started guard stands within 14 [M 0x15ec08];
    // the real player clears them through the real fire path (walkreal-a's choreography) BEFORE
    // the escort leg — so the walking-speed leg starts with the lead actually leading.
    let mut cleared = std::collections::HashSet::new();
    for _round in 0..12 {
        let (cp, cstate) = {
            let sim = w.scene.sim.as_ref().unwrap();
            (sim.drones.list[ci].pos, sim.drones.list[ci].state)
        };
        let guards: Vec<usize> = {
            let sim = w.scene.sim.as_ref().unwrap();
            (0..sim.drones.list.len()).filter(|&i| {
                let d = &sim.drones.list[i];
                i != ci && d.team == 1 && d.started && !d.corpse && d.enabled
                    && walker::dxz(d.pos, cp) < 14.0 && !cleared.contains(&i)
            }).collect()
        };
        if guards.is_empty() && cstate != state::LEAD_HIDE { break; }
        for g in guards {
            let (gpos, los_clear) = {
                let sim = w.scene.sim.as_ref().unwrap();
                let d = &sim.drones.list[g];
                let bp = sim.bond.pos;
                let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
                (d.pos, sim.world.segment(eye, [d.pos[0], d.pos[1] + 0.3, d.pos[2]]).is_none())
            };
            let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
            if walker::dxz(gpos, bpos) > 9.0 || !los_clear {
                w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.0);
            }
            if w.scene.sim.as_ref().unwrap().gun.clip + w.scene.sim.as_ref().unwrap().gun.spare < 8 { w.resupply("ammo"); }
            let killed = w.shoot(g, &format!("guard {g}")) || { w.resupply("ammo retry"); w.shoot(g, &format!("guard {g}")) };
            let sim = w.scene.sim.as_ref().unwrap();
            if killed || sim.drones.list[g].corpse || sim.drones.list[g].health <= 0.0 { cleared.insert(g); }
        }
    }
    let cpos = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
    w.walk_nav_to(cpos, "to the meet-up", 2.5);

    // ── the walking-speed escort: Bond follows at the analog half-press ────────────────────────
    // The lead RUNS (anim 0x1e [M 0x15e698 case 3]) — he pulls ahead until Bond is more than 9
    // route-units behind, then waits (AllyLeadWait); Bond's walk closes to 7 and he resumes. The
    // follower keeps 4.5..10 behind, out of the PlayerInWay band, and every second asserts the
    // game's own bands off the drone's `mission_path_dist` (SM+0x20).
    let mut checked = 0u32;
    let mut last_round = [0.0f32; 3];
    let settled = 'escort: {
        for round in 0..12 {
            let mp = w.scene.sim.as_ref().unwrap().drones.list[ci].pos;
            if walker::dxz(mp, plate) < 5.0 { break 'escort mp; }
            let mut last = mp;
            let mut still = 0u32;
            let start = w.total_ticks;
            loop {
                if w.total_ticks - start >= 9000
                    || w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break; }
                let sim = w.scene.sim.as_ref().unwrap();
                let d = &sim.drones.list[ci];
                let (dstate, dpos, pd) = (d.state, d.pos, d.mission_path_dist);
                last_round = dpos;
                let bond = sim.bond.pos;
                let db = walker::dxz(bond, dpos);
                // The distance asserts — the game's own bands, evaluated whenever the escort
                // machine has computed the path value this tick (Lead/Wait update it).
                if matches!(dstate, state::ALLY_LEAD | state::ALLY_LEAD_WAIT) {
                    checked += 1;
                    if dstate == state::ALLY_LEAD {
                        assert!(pd >= -9.0, "the lead abandoned Bond: path {pd:.1} < -9.0 at ({:.1},{:.1},{:.1}) [M 0x15e698 case 3]", dpos[0], dpos[1], dpos[2]);
                    } else {
                        assert!(pd <= -7.0 || pd == 0.0,
                            "the lead resumed outside the game's band: path {pd:.1} in AllyLeadWait [M 0x15eda8 case 3]");
                    }
                    assert!(db < 24.0, "the escort left Bond behind: {db:.1} straight-line at ({:.1},{:.1},{:.1})", dpos[0], dpos[1], dpos[2]);
                }
                assert!(ESCORT_STATES.contains(&dstate), "the escort left the ally machine: state {dstate:#x}");
                drop(sim);
                still = if walker::dxz(dpos, last) < 0.0025 { still + 1 } else { 0 };
                last = dpos;
                if walker::dxz(dpos, plate) < 5.0 { break 'escort dpos; }
                if still > 900 { break; } // 15 s parked — the round loop recovers (as walkreal's)
                // Steer BEHIND the lead at the walk pace: full press only past 10 (the catch-up
                // closing), the 0.55 half-press (the walk) in 4.5..10, stand inside 4.5. The band
                // is 3D — a lead waiting at a stair head must be FOLLOWED (the 2D dxz is blind to
                // the floors and would stand Bond under him forever).
                let sim = w.scene.sim.as_ref().unwrap();
                let desired = (dpos[0] - sim.bond.pos[0]).atan2(dpos[2] - sim.bond.pos[2]);
                let err = walker::wrap_pi(desired - sim.bond.yaw);
                let dy = dpos[1] - sim.bond.pos[1];
                let d3 = (walker::dxz(sim.bond.pos, dpos).powi(2) + dy * dy).sqrt();
                let input = Input {
                    turn: err.clamp(-0.07, 0.07),
                    forward: if d3 > 10.0 { 1.0 } else if d3 > 4.5 { 0.55 } else { 0.0 },
                    ..Default::default()
                };
                drop(sim);
                w.tick(input, false);
            }
            // Stall recovery — the walkreal-a rules: clear armed started guards within 14, step
            // out of the way band, or walk past the wedged lead so he re-paths.
            let (cstate, cp, cpd) = {
                let sim = w.scene.sim.as_ref().unwrap();
                let d = &sim.drones.list[ci];
                (d.state, d.pos, d.mission_path_dist)
            };
            eprintln!("[escort] round {round} stall: contact ({:.1},{:.1},{:.1}) state {cstate:#x} path {cpd:.1} bond ({:.1},{:.1},{:.1}) db {:.1}",
                cp[0], cp[1], cp[2], w.scene.sim.as_ref().unwrap().bond.pos[0], w.scene.sim.as_ref().unwrap().bond.pos[2], w.scene.sim.as_ref().unwrap().bond.pos[1], walker::dxz(cp, w.scene.sim.as_ref().unwrap().bond.pos));
            {
                let sim = w.scene.sim.as_ref().unwrap();
                let d = &sim.drones.list[ci];
                if let Some(r) = &d.route {
                    let nodes: Vec<String> = r.nodes.iter().take(r.index.max(0) as usize + 3).rev().take(4)
                        .map(|&n| format!("({:.1},{:.1},{:.1})", sim.nav.paths[r.path].nodes[n as usize].pos[0], sim.nav.paths[r.path].nodes[n as usize].pos[1], sim.nav.paths[r.path].nodes[n as usize].pos[2]))
                        .collect();
                    eprintln!("[escort] route idx {} of {}: behind {:?} target {:?}", r.index, r.nodes.len(), nodes, r.target);
                }
            }
            let guards: Vec<usize> = {
                let sim = w.scene.sim.as_ref().unwrap();
                (0..sim.drones.list.len()).filter(|&i| {
                    let d = &sim.drones.list[i];
                    i != ci && d.team == 1 && d.started && !d.corpse && d.enabled
                        && walker::dxz(d.pos, cp) < 14.0 && !cleared.contains(&i)
                }).collect()
            };
            if !guards.is_empty() {
                for g in guards {
                    let (gpos, los_clear) = {
                        let sim = w.scene.sim.as_ref().unwrap();
                        let d = &sim.drones.list[g];
                        let bp = sim.bond.pos;
                        let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
                        (d.pos, sim.world.segment(eye, [d.pos[0], d.pos[1] + 0.3, d.pos[2]]).is_none())
                    };
                    let bpos = w.scene.sim.as_ref().unwrap().bond.pos;
                    if walker::dxz(gpos, bpos) > 9.0 || !los_clear {
                        w.walk_nav_to(gpos, &format!("approach guard {g}"), 3.0);
                    }
                    if w.scene.sim.as_ref().unwrap().gun.clip + w.scene.sim.as_ref().unwrap().gun.spare < 8 { w.resupply("ammo"); }
                    let killed = w.shoot(g, &format!("guard {g}")) || { w.resupply("ammo retry"); w.shoot(g, &format!("guard {g}")) };
                    let sim = w.scene.sim.as_ref().unwrap();
                    if killed || sim.drones.list[g].corpse || sim.drones.list[g].health <= 0.0 { cleared.insert(g); }
                }
            } else if walker::dxz(cp, w.scene.sim.as_ref().unwrap().bond.pos) > 2.0
                && (w.scene.sim.as_ref().unwrap().world.segment(
                    [w.scene.sim.as_ref().unwrap().bond.pos[0], w.scene.sim.as_ref().unwrap().bond.pos[1] + 1.0, w.scene.sim.as_ref().unwrap().bond.pos[2]],
                    [cp[0], cp[1] + 1.0, cp[2]]).is_some()
                    || cstate == state::ALLY_LEAD_WAIT) {
                // ... or the lead is simply PARKED waiting (AllyLeadWait): the real player walks to
                // him — the resume band is route-relative, so standing outside the route's fence
                // strip (db 2.6, the scan's far-node −9.8) never un-parks him; closing through the
                // nav does.
                // The straight line to the lead is BLOCKED (a wall corner / a closed shoji between
                // them): the direct steer wedges against it and holds the route gap open. Close
                // through the nav route — the walker's stuck handler opens the route's doors under
                // Bond's feet [the walkreal-a guard-approach rule].
                w.walk_nav_to(cp, "the line to the lead is blocked — close through the nav", 2.0);
            } else if (cp[1] - w.scene.sim.as_ref().unwrap().bond.pos[1]).abs() > 1.5 {
                // A floor gap (the lead parked at a stair head, Bond below): the direct steer is
                // blind to the vertical — close through the nav route (the stairs) [the walker's
                // stall-recovery close, as walkreal-a's].
                w.walk_nav_to(cp, "close the floor gap to the lead", 2.0);
            } else if cstate == state::PLAYER_IN_WAY {
                let back = [cp[0] + (cp[0] - plate[0]) * 0.5, cp[1], cp[2] + (cp[2] - plate[2]) * 0.5];
                w.walk_nav_to(back, "step out of the lead's way", 3.0);
            } else {
                // A closed door on the route: the escort works his own door nodes (DSTATE_OpenDoor
                // 0x81), but the courtyard slide door is BOND's to open — the game's other
                // activator [M Player_Activate 0x1a5170 case 0x1c → Door_Activate(obj, player, 1)].
                // The fix_mission patience: press only while the door is CLOSED/CLOSING (& 0x12), a
                // press on an OPENING door re-closes it [M 0x135be0's state-4/8 arm]; stand behind
                // the contact on his line to the door.
                let door_i = if cstate == state::OPEN_DOOR {
                    // He is working his own door node [M 0x1681f8] — pressing over him mashes the
                    // door (the fix_mission deadlock: an activation every tick sets the tick-busy
                    // marker and re-closes what he opened). Wait for his work instead.
                    None
                } else {
                    // The BLOCKING door sits between Bond and the contact — pick it by the
                    // midpoint of the pair, not by the contact alone (a closed door behind the
                    // contact's back would win the wrong way).
                    let mid = [(cp[0] + w.scene.sim.as_ref().unwrap().bond.pos[0]) * 0.5,
                               cp[1],
                               (cp[2] + w.scene.sim.as_ref().unwrap().bond.pos[2]) * 0.5];
                    w.scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate()
                        .filter(|(_, dr)| dr.state & 0x12 != 0 && walker::dxz(dr.pos, mid) < 8.0)
                        .min_by(|(_, a), (_, b)| walker::dxz(a.pos, mid).total_cmp(&walker::dxz(b.pos, mid)))
                        .map(|(i, _)| i)
                };
                if let Some(door_i) = door_i {
                    let (dpos, dl0) = {
                        let sim = w.scene.sim.as_ref().unwrap();
                        (sim.drones.doors.list[door_i].pos, sim.drones.doors.list[door_i].state)
                    };
                    eprintln!("[escort] round {round}: the contact parks at a closed door {door_i} (state {dl0:#x}) at ({:.1},{:.1}) — Bond E-opens it", dpos[0], dpos[2]);
                    let (dx, dz) = (dpos[0] - cp[0], dpos[2] - cp[2]);
                    let l = (dx * dx + dz * dz).sqrt().max(1e-4);
                    for _ in 0..1500u32 {
                        let sim = w.scene.sim.as_ref().unwrap();
                        let st = sim.drones.doors.list[door_i].state;
                        if st & 0x12 == 0 { break; } // opening/open — stop pressing
                        let bp = sim.bond.pos;
                        let stand = [dpos[0] - dx / l * 1.6, dpos[1], dpos[2] - dz / l * 1.6];
                        let d = walker::dxz(bp, stand);
                        let desired = (stand[0] - bp[0]).atan2(stand[2] - bp[2]);
                        let err = walker::wrap_pi(desired - sim.bond.yaw);
                        drop(sim);
                        if d > 1.8 {
                            w.tick(Input { turn: err.clamp(-0.07, 0.07), forward: 0.55, ..Default::default() }, false);
                        } else {
                            w.tick(Input { turn: err.clamp(-0.07, 0.07), ..Default::default() }, true);
                        }
                    }
                } else {
                    let (dx, dz) = (plate[0] - cp[0], plate[2] - cp[2]);
                    let l = (dx * dx + dz * dz).sqrt().max(1e-3);
                    let ahead = [cp[0] + dx / l * 4.0, cp[1], cp[2] + dz / l * 4.0];
                    w.walk_nav_to(ahead, "walk past the wedged lead", 3.0);
                }
            }
        }
        panic!("the escort never reached the plate in 12 rounds (last at {last_round:?})");
    };
    assert!(checked > 0, "the walking-speed leg must have evaluated the game's distance bands");
    assert!(walker::dxz(settled, plate) < 6.0, "the contact must walk his route to the LoadPointActivator plate (at {settled:?})");
    eprintln!("[escort] the contact settled at ({:.1},{:.1},{:.1}) after {} distance checks",
        settled[0], settled[1], settled[2], checked);

    // ── the exit's condition: BOTH bodies on the plate fire the transition ─────────────────────
    // Bond walks the last leg onto the plate (the stairwell descent anchor walkreal-a uses).
    w.walk_nav_to([-30.8, 6.4, 13.1], "the stairwell head", 1.2);
    w.walk_nav_to(plate, "the plate", 1.2);
    for _ in 0..120 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    let on_plate = sim.bond.pos[0] < -32.0 && sim.bond.pos[2] > 12.0;
    assert!(on_plate, "Bond must walk onto the LoadPointActivator plate (at {:?})", sim.bond.pos);
    assert!(sim.drones.channels.on(0x10), "the AND plate must fire with Bond AND the contact inside [M Trigger_Hit 0x1b1fc8]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0002), "the exit fires to 07000002 under the game's condition");
    eprintln!("[escort] level_exit 07000002 at tick {}", w.total_ticks);
    for f in &w.report.falls { eprintln!("[escort] fall: {f}"); }
}
