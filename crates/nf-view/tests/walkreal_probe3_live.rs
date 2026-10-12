//! walkreal probe 3: VERIFY the spawn-room escape route of 07000002 with the real input path only.
//!
//! Route from the probe-2 evidence (crates/nf-view/tests/walkreal_probe2_live.rs):
//!   * The level's only kind-0x24 start is (−27.17, 1.64, 29.80) gate 0 [D map 51 #0] — the bunker
//!     room IS the game's spawn, cold or warm (pick_start has no other candidate; 07000001's exit
//!     leaves ch 0x10 on but there is no gate-0x10 start here).
//!   * The room's floor is −0.05; a ~1.2-wide south corridor runs along x ≈ −27.2 (walls at x −27.8
//!     and −26.6; the capsule centreline push is ≤ 0.07) from the room pocket (z 29.5..34.5) down to
//!     the ladder alcove at z 41..42.5 [probe2_corridor_push, probe2_room_bfs (lenient path, 23 nodes)].
//!   * The alcove holds ladder[1] (placement #114, kind 0x22): centre (−26.87, 2.54, 42.23), r 2.57,
//!     yaw −π, bottom 0.97, top 5.96. Grab cells on the bunker floor: x −27.67..−26.17,
//!     z 40.93..42.68 (root 1.05 inside the window −0.53..5.61; reach 1.55).
//!   * No doors, no hatches, no other trigger volumes inside the room [probe2_room_objects] — the
//!     ladder is the only way out.
//!   * The top dismount lunge (+0.6 local forward [M Player_Climb 0x196310]) carries Bond from the
//!     ladder column (z 41.73) to z 42.35, over the garden band (floor 4.82–4.91, root ≈ 5.95) that
//!     starts at z ≈ 42.4 — the game's main floor here ("the playtest's garden", docs/atlas/ladder.md).
//!
//! This test drives Bond with `Sim::step_bond(Input{..}, …)` ONLY (the walker harness) along that
//! route and asserts he climbs the ladder and lands on the main floor (y > 4.5), then that the
//! TouchOnce #341 plate (out 0x15) fires — the game's own "main floor reached" signal.

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The climb trace: walk into the ladder, then hold forward and print the trajectory around the
/// top dismount — where does the lunge land him?
#[test]
fn probe3_climb_trace() {
    use nf_game::player::Input;
    let mut w = Walker::new(nf_view::load(&iso(), "07000002.bin", false).expect("load 07000002"));
    w.wait(30);
    // Straight down the corridor to the alcove (verified walkable by the first test).
    w.walk_to(Wp::new(-27.2, 1.05, 36.0), "corridor");
    w.walk_to(Wp::new(-27.2, 1.05, 40.5), "corridor south");
    // Walk INTO the ladder until the grab.
    let mut grabbed = false;
    for t in 0..600 {
        let sim = w.scene.sim.as_ref().unwrap();
        let desired = (-26.87 - sim.bond.pos[0]).atan2(42.23 - sim.bond.pos[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let input = Input { turn: if err.abs() > 0.2 { err.clamp(-0.08, 0.08) } else { 0.0 }, forward: 1.0, ..Default::default() };
        w.tick(input, false);
        if w.scene.sim.as_ref().unwrap().bond.substate == nf_game::player::SS_CLIMB { grabbed = true; eprintln!("[trace] grabbed at +{t}"); break; }
    }
    assert!(grabbed, "no grab");
    // Climb with forward only, no turn (the game eases the yaw itself), tracing.
    for t in 0..400u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        let climbing = sim.bond.substate == nf_game::player::SS_CLIMB;
        if t % 5 == 0 || !climbing {
            eprintln!("[trace] +{t}: pos ({:.3},{:.3},{:.3}) yaw {:.3} sub {} grounded {} regrab {} vel {:?}", p[0], p[1], p[2], sim.bond.yaw, sim.bond.substate, sim.bond.grounded, sim.bond.regrab, sim.bond.vel);
        }
        if !climbing && sim.bond.grounded {
            eprintln!("[trace] LANDED grounded at ({:.3},{:.3},{:.3}) after +{t}", p[0], p[1], p[2]);
            let sim = w.scene.sim.as_ref().unwrap();
            eprintln!("[trace] floor_below(pos, 8) = {:?}", sim.world.floor_below(sim.bond.pos, 8.0));
            break;
        }
        w.tick(Input { forward: 1.0, ..Default::default() }, false);
    }
}


#[test]
fn walk_spawn_room_to_main_floor() {
    use nf_game::player::Input;
    let mut w = Walker::new(nf_view::load(&iso(), "07000002.bin", false).expect("load 07000002"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        eprintln!("[probe3] spawn = ({:.3},{:.3},{:.3}) grounded {}", p[0], p[1], p[2], sim.bond.grounded);
        assert!((p[0] - -27.17).abs() < 0.1 && (p[2] - 29.80).abs() < 0.1, "the level's only 0x24 start [D], got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
        // The ladder under test, from the sim's own list.
        let l = &sim.ladders[1];
        eprintln!("[probe3] ladder[1] centre ({:.2},{:.2},{:.2}) r {:.2} yaw {:.3} top {:.2} bottom {:.2}",
            l.centre[0], l.centre[1], l.centre[2], l.radius, l.yaw, l.top(), l.bottom());
    }
    w.wait(30);

    // 1. South through the room pocket (z 29.5..34.5), then the tight corridor x ≈ −27.2
    //    (walls at −27.8/−26.6) down to the ladder alcove at z 41..42.5.
    w.walk_to(Wp::new(-27.2, 1.05, 31.5), "pocket south");
    w.walk_to(Wp::new(-27.2, 1.05, 33.5), "pocket exit");
    w.walk_to(Wp::new(-27.2, 1.05, 36.0), "corridor");
    w.walk_to(Wp::new(-27.2, 1.05, 39.5), "corridor south");
    w.walk_to(Wp::new(-27.2, 1.05, 41.0), "alcove mouth");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[probe3] at the alcove mouth: pos ({:.2},{:.2},{:.2}) d(ladder) {:.2}",
            sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2],
            ((sim.bond.pos[0] - -26.87).powi(2) + (sim.bond.pos[2] - 42.23).powi(2)).sqrt());
    }

    // 2. The ladder grab with real input: walk INTO the ladder centre (the grab is not input-driven —
    //    walking into the axis column with |ladder.yaw − yaw| > π/2 and the root in the window attaches).
    //    No turn during the climb: the game eases the yaw itself; a steering turn here would spin the
    //    top lunge (the dismount lunge rides the facing).
    let mut grabbed = false;
    for t in 0..600 {
        let sim = w.scene.sim.as_ref().unwrap();
        let desired = (-26.87 - sim.bond.pos[0]).atan2(42.23 - sim.bond.pos[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let input = Input { turn: if err.abs() > 0.2 { err.clamp(-0.08, 0.08) } else { 0.0 }, forward: 1.0, ..Default::default() };
        w.tick(input, false);
        if w.scene.sim.as_ref().unwrap().bond.substate == nf_game::player::SS_CLIMB { grabbed = true; eprintln!("[probe3] ladder grabbed at +{t}"); break; }
    }
    assert!(grabbed, "the ladder grab never fired");
    let mut climbed = false;
    for t in 0..400 {
        let sim = w.scene.sim.as_ref().unwrap();
        let climbing = sim.bond.substate == nf_game::player::SS_CLIMB;
        if !climbing && sim.bond.grounded && sim.bond.pos[1] > 4.5 {
            let p = sim.bond.pos;
            eprintln!("[probe3] top dismount landed at ({:.3},{:.3},{:.3}) after +{t} climb ticks", p[0], p[1], p[2]);
            climbed = true;
            break;
        }
        w.tick(Input { forward: 1.0, ..Default::default() }, false);
    }
    assert!(climbed, "the climb never reached the top dismount onto the garden");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        eprintln!("[probe3] after the climb: pos ({:.3},{:.3},{:.3}) grounded {} floor_below {:?} substate {}",
            p[0], p[1], p[2], sim.bond.grounded, sim.world.floor_below(p, 8.0), sim.bond.substate);
        assert!(p[1] > 4.5, "Bond must reach the main floor (y > 4.5) after the climb, got {}", p[1]);
    }

    // 3. Prove he is on the level's garden/main floor: walk east along the garden band INTO the
    //    guard-enable TouchOnce #341 (out 0x15, mask 8 player; bbox −25.7..−24.3, 4.7..6.1,
    //    40.8..44.4 [D]) and let the volume fire ch 0x15.
    w.walk_to(Wp::new(-26.0, 5.9, 43.4).r(1.2), "garden east");
    w.walk_to(Wp::new(-24.6, 5.9, 42.8).r(0.8), "into the #341 plate");
    w.wait(30);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        eprintln!("[probe3] on the main floor: pos ({:.3},{:.3},{:.3}) floor_below {:?} ch 0x15 = {}",
            p[0], p[1], p[2], sim.world.floor_below(p, 8.0), sim.drones.channels.on(0x15));
        assert!(p[1] > 4.5, "still on the main floor");
        assert!(sim.drones.channels.on(0x15), "TouchOnce #341 (out 0x15) must fire on the main floor");
    }
    eprintln!("[probe3] ESCAPE VERIFIED in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for (label, ticks, pos) in &w.report.legs { eprintln!("[probe3] leg '{label}' {ticks} ticks -> ({:.2},{:.2},{:.2})", pos[0], pos[1], pos[2]); }
    for s in &w.report.stuck_events { eprintln!("[probe3] stuck: {s}"); }
}
