//! walkreal-c: 07000015 (the missile base silo) walked with the player's own movement — no
//! `bond.pos` writes after the spawn. The route (docs/levels/07000015.md; the walkreal_probe_c
//! dump): the spawn plate #484 fires ch 0x4b under Bond; the west grid lanes at y≈41.4 (doors
//! [0]/[48]/[36] unlocked) → the north lane through doors [38]/[39]/[46]/[47] → the west lane
//! doors [32]/[29] → the south lane doors [42]/[51]/[49] (all unlocked — the keypad-locked doors
//! [37] 0xf7 / [35] 0xc8 are bypassed by the north lane) → the USE exit #445 at (0.0, 41.3, −24.8)
//! ([M Player_Activate 0x1a5170 case 0x1d → Trigger_Activate 0x1b2318]; `Trigger_Update` case 10
//! never self-fires with gate 0 [M 0x1b1a78:162-166]).
//!
//! Enemies are DISABLED for the walk only in the walkreal sense: damage is suppressed
//! (top_up_health, the walker's allowance — the garrison's pain pipeline still runs). All 20
//! guards stay live and armed. Opt-in via NF_ISO (the slim ISO is enough).

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

// GAP (walkreal-c, updated): the route is PROVEN from the spawn to door[47]'s north lane — the
// silo crossing (the z≈−5 lane threads the silo hole), the x34.6 spine's west plank around the
// door[45] keypad slot (0xc8 has no writer on the level — the east plank dead-ends), doors
// [0]/[48]/[36]/[38]/[39]/[46]/[47] all open (E or the straddle walk-in), and the rider reaches
// (28.1, 41.5, 68.7). The stop: the top-band door[32]'s doorway (in the west wall at z 70.1) —
// the approach corner hops Bond onto furniture (y 42.2, airborne) and the straddle radius (1.4)
// is never entered. Next: a patient threshold walk or the doorway's true facing.
#[test]
#[ignore = "the walk's reached blocker — see docs/reports/walkreal-c.md; un-ignore when the blocker lands"]
fn walk_spawn_to_the_omega_sector_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000015.bin", false).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 93.0 && p[0] < 96.0 && p[1] > 40.0 && p[1] < 43.0 && p[2] > -2.0 && p[2] < 2.0,
            "the cold start spawns on the silo gantry (#1 [D]), got {p:?}");
    }
    w.wait_enabled(3000, "the silo arrival");
    w.wait(120);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4b), "the spawn plate #484 fires ch 0x4b [D]");
        assert_eq!(sim.drones.mission.states[21][0], 3, "'Find the exit to Omega Sector' announces [L record 15]");
    }

    // The route: the spawn gantry west, then the NAV GRAPH's own route to the exit hall (the
    // z≈0 lane crosses the silo hole at x 66-74 — the walk's first run fell in; the guards' AI
    // paths are the game's own answer for the gantry maze), then the USE exit.
    w.walk_to(Wp::new(88.0, 41.4, -0.5).r(1.2), "the gantry west");
    assert!(w.follow_nav_route([34.6, 41.4, 6.0], "the silo crossing"), "the nav route crosses the gantry");
    w.walk_to(Wp::new(34.6, 41.4, 6.0).r(1.2), "the north turn");

    // The north lane through the grid, then the west lane south, then the exit hall. The x34.6
    // spine SPLITS at z 30: the east plank (x38-44) dead-ends at door[45]'s keypad slot (ch_lock
    // 0xc8 — no writer on the level [D scans; the trigger dump has no AND] — permanent), and the
    // first runs fell into the lower deck (y −3.9) at (33.7, 38.8). The WEST plank (x 24-34,
    // z 28-42, the floor grid) carries the route around the slot to door[46].
    let route: &[(&str, Wp)] = &[
        ("door[38] north", Wp::new(34.6, 41.4, 16.0).r(1.2)),
        ("door[39] north", Wp::new(34.6, 41.4, 28.0).r(1.2)),
        ("the west plank", Wp::new(29.0, 41.4, 31.0).r(1.2)),
        ("the plank north", Wp::new(28.0, 41.4, 35.0).r(1.2)),
        ("the plank north 2", Wp::new(28.5, 41.4, 39.0).r(1.2)),
        ("the spine north", Wp::new(30.5, 41.4, 42.0).r(1.2)),
        ("door[46] north", Wp::new(34.6, 41.4, 46.5).r(1.2)),
        ("door[46] through", Wp::new(34.6, 41.4, 52.0).r(1.2)),
        ("door[47] north", Wp::new(34.6, 41.4, 65.0).r(1.2)),
        ("door[32] east side", Wp::new(27.6, 41.4, 70.1).r(0.7)),  // the doorway is in the west wall at z 70.1
        ("door[32] threshold", Wp::new(26.0, 41.4, 70.1).r(0.7)),  // INTO the straddle door's plane
        ("door[32] through", Wp::new(23.0, 41.4, 70.3).r(0.9)),
        ("door[32] west-top", Wp::new(20.0, 41.4, 70.0).r(1.2)),
        ("door[29] west-top", Wp::new(4.0, 41.4, 70.0).r(1.2)),
        ("the south turn", Wp::new(-1.0, 41.4, 62.0).r(1.2)),
        ("door[42] south", Wp::new(-1.0, 41.4, 20.0).r(1.2)),
        ("door[51] south", Wp::new(-1.0, 41.4, 4.0).r(1.2)),
        ("the exit hall", Wp::new(-1.0, 41.4, -8.0).r(1.2)),
        ("the exit approach", Wp::new(-0.6, 41.4, -21.0).r(1.4)),
    ];
    w.walk_route(route);

    // The USE exit #445: face the plane, pulse E (the use sphere: head + look, radius 1 [M 0x1a5170]).
    let exit_at = [0.0f32, 41.3, -24.8];
    w.walk_to(Wp::at(exit_at[0], exit_at[1], exit_at[2] + 1.2, Act::Use { use_ticks: 300 }).r(1.0), "the USE exit #445");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.level_exit, Some(0x0700_0016),
            "E at the #445 plane loads 07000016 [D placement #445 param 0]");
        assert!(!sim.drones.channels.on(0x42), "nothing writes ch 0x42 on this level [D scans] — the gated gate #343's channel stays cold");
        assert_eq!(sim.drones.mission.states[21][0], 3,
            "the objective stays announced on 07000015 (its completing channel 0x8e is 07000016's spawn plate [D])");
    }
}
