//! walkreal-a: 07000004 "Enemies Vanish" (Castle D) walked with the player's own movement — no
//! teleports after the spawn. The route (docs/levels/07000004.md):
//!   the start (1.87, 5.84, 41.07) → the intro SP #3 + the story scene SP #2 settle by ~tick 1600
//!   (the AI re-enabled, ch 0x53 written; the story places Bond beside the target) → the assassination:
//!   the target (drone skin 0x0500001e, stationary) dies → ch 0x54 [M DroneFunc_SetDeathChannel
//!   0x13e660] → the OUT trigger #46 copies it to ch 0x64 (the mission-complete channel; its
//!   consumer Mission_Update 0x182c60 is the un-ported mission layer — the walk asserts the raise
//!   and the copy, the level has no 0xE8 exit [D]).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_the_assassination() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.level, 0x0700_0004);
        assert!(sim.drones.triggers.iter().all(|t| t.kind != walker::trigger_kind::LOAD_LEVEL),
            "07000004 has no 0xE8 exit [D placement probe]");
        let p = sim.bond.pos;
        assert!((p[0] - 1.87).abs() < 2.0 && (p[2] - 41.07).abs() < 2.0, "start #0 [D], got {p:?}");
    }
    // The intro + story scenes: the walker skips them through the game's own skip action; wait for
    // the settle (the AI re-enabled, ch 0x53 written) [D walk_07000004_live's probe].
    let mut ready = false;
    for _ in 0..6000u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { ready = true; break; }
    }
    assert!(ready, "the intro + story scenes must settle (the AI re-enabled, ch 0x53 written)");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        eprintln!("[04] settled at tick {} bond ({:.2},{:.2},{:.2}) ch 0x53 on", w.total_ticks, p[0], p[1], p[2]);
        assert!(!sim.drones.channels.on(0x54), "the objective channel starts off");
    }
    // The target: stationary, the story scene places Bond right next to him — walk the last steps
    // and shoot him through the real fire path.
    let target_i = w.scene.sim.as_ref().unwrap().drones.list.iter()
        .position(|d| d.skin == 0x0500_001e).expect("the target drone exists");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let tp = sim.drones.list[target_i].pos;
        eprintln!("[04] the target at ({:.1},{:.1},{:.1}), bond {:.1} away", tp[0], tp[1], tp[2], walker::dxz(tp, sim.bond.pos));
    }
    let killed = w.shoot(target_i, "the target");
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(killed || sim.drones.list[target_i].corpse || sim.drones.list[target_i].health <= 0.0,
        "the target must die (the walk's own shots)");
    // The death channel + the OUT copy settle over the death anim.
    let mut raised = false;
    let mut completed = false;
    for _ in 0..900u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x54) { raised = true; }
        if sim.drones.channels.on(0x64) { completed = true; break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(raised, "the target's death must raise ch 0x54 [M DroneFunc_SetDeathChannel 0x13e660]");
    assert!(completed, "the OUT trigger #46 must copy ch 0x54 to the mission-complete ch 0x64");
    eprintln!("[04] walked the assassination in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[04] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[04] stuck: {s}"); }
}
