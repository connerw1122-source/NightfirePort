//! walkreal-a: 07000002 "The Exchange" (Castle B) walked with the player's own movement — no
//! teleports after the spawn. The route (docs/levels/07000002.md):
//!   spawn #0 (−27.17, 1.64, 29.80, gate 0) → climb the south ladder (−26.9, 2.5, 42.2, top 6.0)
//!   → the bunker: the three TouchOnce plates #385/386/387 (outs 0x7E/0x7F/0x80, lock 0x81) →
//!   AND #388 → ch 0x66 → TouchOnce #389 (out 0x81) → the Load Point #206 (E, key gate ch 0x4C).
//! ch 0x4C's writer is NOT on the disc (the un-ported mission/dialogue layer [D the level scan]) —
//! the walk proves the route to the exit and the refusal, then hands the channel the way the
//! mission layer would and loads 07000003.

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

#[test]
fn walk_the_bunker_route_to_the_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000002.bin", false).expect("load 07000002"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - -27.17).abs() < 2.0 && (p[2] - 29.8).abs() < 2.0, "spawn #0 [D], got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
    }
    w.wait(60);

    // 1. The dock ladder at (−15.3, 3.0, 35.0) (top 5.2): the start sits at y 1.05, the main
    //    floor at ~6.3 [L dump]. The other ladder (−26.9, 42.2) sits behind the dock's north wall.
    // 1. The secret-bunker room (the level's only 0x24 start [D; probe2]): the tight corridor at
    //    x −27.2 (walls −27.8/−26.6) runs south to the alcove, then ladder #114 (centre
    //    (−26.87, 2.54, 42.23), top 5.96) climbs to the garden — the main floor [L probe2/3].
    for (name, at) in [
        ("bunker room south", [-27.2f32, 1.05, 31.5]),
        ("bunker corridor", [-27.2, 1.05, 36.0]),
        ("corridor south", [-27.2, 1.05, 39.5]),
        ("alcove mouth", [-27.2, 1.05, 41.0]),
    ] {
        w.walk_to(Wp { at, act: Act::None, arrive: 1.0 }, name);
    }
    w.walk_to(Wp { at: [-26.87, 2.54, 42.23], act: Act::Ladder { top_y: 6.0 }, arrive: 0.9 }, "ladder #114");
    // The garden band: TouchOnce #341 (out 0x15) fires as the main floor is reached.
    w.walk_to(Wp { at: [-26.0, 5.9, 43.4], act: Act::None, arrive: 1.4 }, "the garden");
    w.walk_to(Wp { at: [-24.6, 5.9, 42.8], act: Act::None, arrive: 1.4 }, "the garden east");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[02] ch 0x15 (the garden touch) on: {}", sim.drones.channels.on(0x15));
    }

    // 2. Cross the level to the bunker: the garden → the south hall → the west wing → the north
    //    corridor (doors [2]/[1]/[0] open under Bond's feet) → the bunker. Anchors from the AI
    //    paths (BotPath1a/4/5/9 [L path dump]); the nav graph fills between them.
    for (name, at) in [
        ("garden path", [-22.2f32, 5.9, 34.1]),
        ("south hall", [-23.3, 5.9, 27.6]),
        ("south hall west", [-31.2, 5.9, 22.1]),
        ("west wing south", [-37.4, 5.9, 8.0]),
        ("west wing mid", [-52.1, 5.9, -1.5]),
        ("corridor east door", [-27.8, 5.9, -20.3]),
        ("corridor mid door", [-46.2, 5.9, -20.3]),
        ("bunker door", [-46.1, 5.9, -23.2]),
        ("bunker corridor", [-55.1, 5.9, -23.1]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }

    // 4. The bunker plates, in the game's AND order (any order works — the AND just watches).
    let plate = |sim: &nf_view::scene::Sim, out: u8| walker::trigger_centre(sim, walker::trigger_kind::TOUCH_ONCE, out).expect("the bunker plate");
    let sim = w.scene.sim.as_ref().unwrap();
    let p7e = plate(sim, 0x7e);
    let p7f = plate(sim, 0x7f);
    let p80 = plate(sim, 0x80);
    let p389 = walker::trigger_centre(sim, walker::trigger_kind::TOUCH_ONCE, 0x81).expect("the bunker-entry touch");
    drop(sim);
    // 0x7E last: its corner sits behind the deepest furniture; the 0x7F/0x80 side approaches it
    // over open floor [L bunker walk].
    for (name, at) in [("bunker plate 0x7F", p7f), ("bunker plate 0x80", p80), ("bunker plate 0x7E", p7e)] {
        // The pads are small (~1.2) — stand ON them (0x80 sits on a lower ledge).
        w.walk_nav_to(at, name, 0.4);
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[02] plates: 0x7e {} 0x7f {} 0x80 {} → AND 0x66 {} (bond {:?})",
            sim.drones.channels.on(0x7e), sim.drones.channels.on(0x7f), sim.drones.channels.on(0x80),
            sim.drones.channels.on(0x66), sim.bond.pos);
        for t in sim.drones.triggers.iter().filter(|t| [0x7eu8, 0x7f, 0x80].contains(&t.out)) {
            eprintln!("[02] plate {:#x} lo {:?} hi {:?}", t.out, t.lo, t.hi);
        }
        assert!(sim.drones.channels.on(0x66), "the three bunker plates fire AND ch 0x66 [D #388]");
    }
    // 3. The bunker-entry touch #389 (out 0x81, lock 0x66 — fires only once the AND is on).
    w.walk_nav_to(p389, "the bunker touch 0x81", 1.0);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x81), "ch 0x81 fires behind the AND [D #389 lock 0x66]");
    }

    // 6. The exit: E on the Load Point #206 without ch 0x4C refuses [M Trigger_Activate 0x1b2318:76-90].
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL).map(|t| t.centre()).expect("the Load Point");
    drop(sim);
    w.walk_nav_to(exit, "the Load Point", 1.6);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E without the key");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x4c), "ch 0x4C has no writer on this level [D]");
        assert_eq!(sim.drones.level_exit, None, "the key gate refuses the exit without ch 0x4C [M 0x1b2318:76-90]");
    }
    // The un-ported mission layer owns ch 0x4C (the AND(0x48, 0x4B) of the chat/bunker objectives).
    // Hand it the way that layer would, then E: the level loads 07000003.
    let sim = w.scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x4c, 1, sim.drones.frame);
    drop(sim);
    w.walk_to(Wp { at: exit, act: Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E with the key");
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0003), "the exit loads 07000003 through the real reload path");
    eprintln!("[02] walked to the exit in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[02] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[02] stuck: {s}"); }
}
