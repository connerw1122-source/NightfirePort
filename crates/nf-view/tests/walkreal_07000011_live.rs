//! walkreal-b: 07000011 (HT Tower2 A) walked with the player's own movement — no teleports
//! except the documented stairwell-descent stand-in. Route (docs/levels/07000011.md): the spawn
//! #146 (−0.40, 46.97, 25.83 [D]; the TouchOnce #282 spawn tile → ch 0x7E) → east across the top
//! office floor to the stairwell door (13.8, 46.8, 7.1 [D]) → E-opened — the doorway's clear band
//! is its SOUTH half (the leaf's hinge sits on the north edge z 7.1; the swung leaf blocks
//! z 7.0-7.2 [D probe]) → into the stairwell's top landing (15.1, 6.4) → BLOCKER: the stairwell
//! DESCENT — the flights pass UNDER the top landing (the 0x11 plate's flight spans y 35.7-40.1
//! at x 14.6-16.6, z 4.4-6.4, directly below the 46.7 landing [D probe]); the surface stacks show
//! ~5 m landing steps (36.2 → 31.1 → 22.9 → 15.0 on the west line) whose connections the point
//! probes cannot resolve into a walked chain (the flight graph needs the level mesh) → the
//! documented stand-in to the stairwell bottom → the Load Point #158 (14.58, 16.41, −3.94 [D]) →
//! E → level_exit 0x07000012 ([M Trigger_Activate 0x1b2318 — the E arm falls through with the
//! gate off]). Objective "Gain access to lower office floor" (ch 0x48) has no writer on this
//! level — it completes one level later on 07000012 [D §2; the design].

#[path = "common/walker.rs"]
mod walker;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_spawn_to_the_lower_floor_load_point() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000011.bin", false, None).expect("load"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] + 0.40).abs() < 1.5 && (p[1] - 46.97).abs() < 2.0 && (p[2] - 25.83).abs() < 1.5,
            "the spawn #146 [D], got {p:?}");
    }
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x7e),
        "the spawn tile TouchOnce #282 fires ch 0x7E [D]");

    // Leg 1: east across the top floor to the stairwell door (the nav web [D paths 0/1] has no
    // cross-floor route — the direct chain), then through the doorway's south band — walked.
    w.stop_on_exit = false;
    w.follow_nav_route([12.7, 47.4, 6.4], "across the top floor to the stairwell door");
    w.walk_to(Wp::new(16.0, 47.4, 6.4).r(0.9), "through the doorway (the south band)");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[011] post-leg1: exit {:?} ch100 {} ch62 {} ch61 {} ch60 {} ch63 {} pos {:?}",
            sim.drones.level_exit, sim.drones.channels.on(100), sim.drones.channels.on(0x62),
            sim.drones.channels.on(0x61), sim.drones.channels.on(0x60), sim.drones.channels.on(0x63), sim.bond.pos);
        assert!(!sim.drones.channels.on(0x62), "no death on the walked top floor");
    }
    // BLOCKER (pinned): the stairwell DESCENT (see the header).
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [14.8, 17.4, -3.5];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    eprintln!("[011] the stairwell-descent stand-in (the flight graph needs the level mesh)");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let y = sim.bond.pos[1];
        assert!(y < 19.0, "Bond stands at the stairwell bottom (~16.4 [D]), got {y}");
    }
    // Leg 3: the Load Point #158 and the E press ([M 0x1a5170 case 0x1d → 0x1b2318]).
    w.walk_to(Wp::new(14.6, 17.0, -2.8).r(0.9), "the Load Point");
    let mut loaded = false;
    for _ in 0..60 {
        w.tick(nf_game::player::Input::default(), true);
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { loaded = true; break; }
    }
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0012),
        "E at the Load Point #158 loads 07000012 [M 0x1b2318]");
    assert!(loaded, "the exit fired");
    eprintln!("[011] walked out in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
}
