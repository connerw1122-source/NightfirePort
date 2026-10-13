//! walkreal-a: 07000003 "Alpine Escape" (Castle C) walked with the player's own movement — no
//! teleports after the spawn. The route (docs/levels/07000003.md):
//!   the 0x24 start (−8.85, 6.44, −23.19; the 0x4d objective-reveal touch fires under Bond) →
//!   the mid courtyard past the 0x36 plates (lock 0x3a — the story death channel) → the east lawn
//!   (the 0x23/0x24/0x25 guard-enable plates fire as the walk crosses them) → the Load Point #451
//!   (18.11, 6.65, 16.05; E, key gate ch 0x52).
//! ch 0x52's writer is NOT on the disc (the un-ported mission/dialogue layer [D the level scan]) —
//! the walk proves the route to the exit and the refusal, then hands the channel the way the
//! mission layer would and loads 07000004.

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_the_alpine_route_to_the_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000003.bin", false).expect("load 07000003"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - -8.85).abs() < 2.0 && (p[2] - -23.19).abs() < 2.0, "the 0x24 spawn [D], got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
    }
    w.wait(60);
    {
        // The spawn's objective-reveal touch (out 0x4d at (−9.0,−23.2) [D dump]).
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[03] ch 0x4d (the spawn reveal) on: {}", sim.drones.channels.on(0x4d));
    }

    // The route: north-east across the courtyard to the Load Point. The nav graph fills between
    // the anchors; the guard-enable plates (0x21/0x23/0x24/0x25) fire under Bond's feet.
    for (name, at) in [
        ("courtyard east", [0.0f32, 5.9, -20.0]),
        ("mid courtyard", [13.0, 5.9, -18.0]),
        ("the east lawn", [22.0, 5.9, 5.0]),
        ("the exit lawn", [18.5, 5.9, 13.0]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }

    // 6. The exit: E on the Load Point #451 without ch 0x52 refuses [M Trigger_Activate 0x1b2318:76-90].
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL).map(|t| t.centre()).expect("the Load Point");
    drop(sim);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E without the key");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x52), "ch 0x52 has no writer on this level [D]");
        assert_eq!(sim.drones.level_exit, None, "the key gate refuses the exit without ch 0x52 [M 0x1b2318:76-90]");
    }
    // The un-ported mission layer owns ch 0x52. Hand it the way that layer would, then E: the
    // level loads 07000004.
    let sim = w.scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x52, 1, sim.drones.frame);
    drop(sim);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E with the key");
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0004), "the exit loads 07000004 through the real reload path");
    eprintln!("[03] walked to the exit in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[03] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[03] stuck: {s}"); }
}
