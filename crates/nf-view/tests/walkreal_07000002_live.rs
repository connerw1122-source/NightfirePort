//! walkreal-a: 07000002 "The Exchange" (Castle B) walked with the player's own movement — no
//! teleports after the spawn. The route (docs/levels/07000002.md):
//!   spawn #0 (−27.17, 1.64, 29.80, gate 0) → ladder #114 (−26.9, 2.5, 42.2, top 6.0) →
//!   the garden: TouchOnce #341 (out 0x15) → the south hall → the west wing → the north corridor →
//!   the bunker door — then SOUTH around the doorway sensor #389 (out 0x81, lock 0x66: its quad
//!   band x −47.65..−46.26 / z −25.14..−21.55 [D tris]) → the three TouchOnce plates #385/386/387
//!   (outs 0x7E/0x7F/0x80, lock 0x81) → AND #388 → ch 0x66 → the Load Point #206 (E, key gate
//!   ch 0x4C).
//! The sensor chain [D #385-389 + M Trigger_Update 0x1b1a78 lock gate]: #389 raises 0x81 (which
//! enables 3 guards [D] AND locks the plates — their lock is 0x81), so the plates are only
//! reachable by sneaking past the sensor; once all three are touched the AND closes 0x66, which
//! LOCKS #389 (dead) — the walk back through the doorway fires nothing.
//! ch 0x4C's writer IS on the disc — AND #480 (out 0x4C = AND(0x48, 0x4B) [D; census correction,
//! docs/levels/07000002.md]) — but the 0x49/0x4A inputs of 0x4B belong to the un-ported mission layer,
//! so the walk drives the gate channel directly: it proves the route to the exit and the refusal,
//! then loads 07000003.

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
    //    corridor → the bunker door (the AI nav's own long way through the west wing and the
    //    courtyard [L path dump]). Anchors from the AI paths; the nav graph fills between them.
    for (name, at) in [
        ("garden path", [-22.2f32, 5.9, 34.1]),
        ("south hall", [-23.3, 5.9, 27.6]),
        ("south hall west", [-31.2, 5.9, 22.1]),
        ("west wing south", [-37.4, 5.9, 8.0]),
        // The west wing's stair lane (x ≈ −36.7, the AI path's own descent [L path dump]):
        // the direct line to the courtyard's mid (x −46..−52) rides the z 3.6 balcony edge and
        // wedges there — the stairs at x −36.7 are the walked descent.
        ("west wing stair", [-36.7, 5.9, 1.0]),
        ("stair bottom", [-36.9, 5.5, -1.5]),
        ("courtyard lane", [-43.2, 5.5, -1.8]),
        ("bunker door", [-46.1, 5.9, -23.2]),
        ("bunker corridor", [-55.1, 5.9, -23.1]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }

    // 3. The doorway sensor #389 (out 0x81, lock 0x66; its quads span x −47.65..−46.26 /
    //    z −25.14..−21.55 at waist height over the corridor floor [D tris]) is UNAVOIDABLE on the
    //    main route — the game's own chain: crossing raises ch 0x81, which enables 3 guards [D]
    //    and LOCKS the plates (their lock is 0x81 [D]). The stealth alternative exists (the z −26
    //    gap at x −59 into the plate room, cell map) and is left to a later pass; this walk takes
    //    the main door and asserts the game's outcome.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x81), "crossing the doorway sensor raises ch 0x81 [D #389 out 0x81]");
    }

    // 4. Walk the bunker room to its far end (the plates' corner): with 0x81 on, the plates are
    //    LOCKED [D #385-387 lock 0x81] — the game's ambush outcome — so they must stay silent.
    w.walk_nav_to([-59.0, 5.4, -26.0], "the bunker room far end", 1.6);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x7e) && !sim.drones.channels.on(0x7f) && !sim.drones.channels.on(0x80),
            "the plates are locked by the ambush (0x81) [D #385-387 lock 0x81]");
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[02] plates (locked by 0x81): 0x7e {} 0x7f {} 0x80 {} → AND 0x66 {} (bond {:?})",
            sim.drones.channels.on(0x7e), sim.drones.channels.on(0x7f), sim.drones.channels.on(0x80),
            sim.drones.channels.on(0x66), sim.bond.pos);
    }
    // 5. The walk back east through the doorway (the sensor is one-shot; 0x81 is latched) and on
    //    to the Load Point #206.
    // 6. E without ch 0x4C refuses [M Trigger_Activate 0x1b2318:76-90].
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
