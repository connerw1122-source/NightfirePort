//! fix-driver-air: the 07000006 truck driver's start sit (skin 0x05000035, dtype 13 → dtypenew 0x17,
//! start script 0600011d — the 880-frame sit-in-truck one-shot [D nf-extract script-info]), through
//! the real paths (`nf_view::load` + `Sim::step_bond`, `Input` the window builds). Playtest 1014-1725
//! (06 start): "he's sitting in the air, why didnt you test this."
//!
//! The game's own numbers (the ground truth this file pins):
//! - [L 05-truck.trace 07000006 segment (frames 37174..37878), drone 13, state 0x32 the whole
//!   window]: x holds 66.118 (span 0.024), z creeps 28.872 → 28.877, and **y settles 39.305 → 39.19 →
//!   39.2** — the crate top (`Object_Crates_Seat`, collision hi y 38.9987 [D the level's collision])
//!   **+ 0.2** — the `DroneAnim_PreTransCallback` 0x13b490:64-66 clamp floor (the hip =
//!   clamp(animObj+0x5C − 0.02, 0.2); DefaultInit 0x14b300:48 starts +0xA0 at 1.0). The settle shape
//!   is the game's SOFT settle: `Drone_CollisionHandler` 0x140968 tail — on the surface
//!   (`Drone_FeetOnPoint` 0x140660's correction = pos.y − +0xA0 − hit.y < 0.2) the velocity zeroes and
//!   `obj.y -= correction × 0.125` per tick (gated correction > −0.1) — the trace's exponential decay
//!   matches ×0.125 exactly.
//! - The pre-fix port floated him a standing hip above the seat: `hip()` was hard 1.0, so pos.y =
//!   crate 39.0 + 1.0 = 40.0 while the sit pose hangs from there — the pelvis at 40.0 vs the game's
//!   39.2 (the palette path draws the pelvis at `pos.y − 1.0 + drone_height(pose root)` ≈ pos.y in
//!   both models [M docs/formats/animation.md "the object position is the hip"]).
//!
//! The fix: `TruckDriverInit` (0x160bd0 case 1) sets the sitting hip 0.2 while the start script
//! plays (the state the game sits in — [L] state 0x32 for the whole trace window), restored to the
//! standing 1.0 on the exits (ANIM_DONE → 0x34, ALARM → 0x33); `Drone_CollisionHandler` runs the
//! game's soft settle toward `floor + hip` instead of the snap.
//!
//! Run: `NF_ISO=… cargo test -p nf-view --test fix_driver_air_live -- --nocapture --test-threads=1`

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// The driver sits ON his crate seat at the game's height: pos.y = crate top + 0.2 (the clamp),
/// reached through the game's soft settle, and the rendered pelvis rides at the game's 39.2.
/// Sisters: he holds x/z through the traced window and steps into TruckDriverIdle after the script.
#[test]
fn the_sitting_driver_rests_on_his_seat_at_the_game_height() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let di = sim.drones.list.iter().position(|dr| dr.skin == 0x0500_0035).expect("the 07000006 driver (skin 0x05000035)");
    // The sit: state 0x32 for the whole script [L 05-truck's 07000006 drone 13; M 0x160bd0 case 1
    // script branch holds the state].
    tick_idle(sim, 2);
    assert_eq!(sim.drones.list[di].state, 0x32, "the driver holds TruckDriverInit while the sit script plays");

    // The soft settle lands him at crate + 0.2: the game's own y 39.305 → 39.19 → 39.2 over ~8 ticks
    // [L 05-truck.trace frames 37178-37207]; the port's settle converges by ~tick 30.
    tick_idle(sim, 60);
    let dr = &sim.drones.list[di];
    assert!((dr.pos[1] - 39.2).abs() < 0.05,
        "the sitting driver rests at the game's 39.2 = the crate top 38.9987 + the 0.2 clamp [L 05-truck.trace] — got {}", dr.pos[1]);
    // ...through the game's settle, not a teleport: the ×0.125 rate [M 0x140968 tail] — the sister
    // test below pins the rate on the standing driver.
    // The rendered pelvis rides at the game's height: the palette's hip-level bone ≈ 39.2 (the game
    // draws the root bone AT the object position [M docs/formats/animation.md]; the port's draw maps
    // the pose root — 39.2 + drone_height(pose root) − 1.0 ≈ 39.2).
    {
        let d = sim.data();
        let pal = sim.drones.list[di].palette(&d).expect("the sit palette");
        let lo = pal.iter().map(|m| m[3][1]).fold(f32::MAX, f32::min);
        let hi = pal.iter().map(|m| m[3][1]).fold(f32::MIN, f32::max);
        assert!(lo < 39.6 && hi < 40.8, "the sit renders at the game's height (the pre-fix float put the span at 39.2..40.75 with pos.y 40.0) — span {lo:.3}..{hi:.3}");
    }

    // Sisters: he holds his seat through the traced window (the game's x span 0.024 over 700 frames
    // [L]; the port's consumed-root drift is the facial3 known gap, bounded here), and the script's
    // end steps him into TruckDriverIdle at the standing hip.
    tick_idle(sim, 640); // tick 700
    let dr = &sim.drones.list[di];
    assert!((dr.pos[0] - 66.118).abs() < 0.1, "the sit holds x (the game's 66.118 [L]) — got {}", dr.pos[0]);
    tick_idle(sim, 220); // tick 920, the 880-frame script done
    let dr = &sim.drones.list[di];
    assert_eq!(dr.state, 0x34, "the driver steps into TruckDriverIdle when the sit script ends [M 0x160bd0 case 5]");
}

/// Sister: the soft settle is the game's rate — the SITTING driver (the 0.2 animated hip, the state
/// the trace pins [L 05-truck.trace]) lifted 0.15 mid-sit moves exactly correction × 0.125 on the
/// first tick ([M 0x140968 tail; the trace's own decay shape 39.305 → 39.289 → …]). The tail's gate
/// is the game's hip-dependent SECOND probe: `c2 = hit − (obj.pos + up·(0.4 − hip))`, settle while
/// `c2 > −0.1` ⟺ `correction < 0.5 − hip` [M 0x140968 @0x140c44-60 + 0x140e30-48; the probe
/// `NDrone2_Collision` 0x140e90] — so the STANDING driver (hip 1.0, state 0x34 after the script) is
/// NOT settled by a 0.15 lift (the gate needs correction < −0.5): the port keeps its snap there (the
/// game's standing y is owned by the animated hip, unported — the report's residual 1; the 109j
/// settle-everywhere gate was the ai_live cover regression, fix-driver-air2).
#[test]
fn the_floor_settle_runs_at_the_games_one_eighth_rate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let di = sim.drones.list.iter().position(|dr| dr.skin == 0x0500_0035).expect("the driver");
    // Mid-sit (state 0x32 holds through the 880-frame script [L 05-truck.trace drone 13]): lifted 0.15,
    // one tick of the settle moves correction × 0.125.
    tick_idle(sim, 100);
    assert_eq!(sim.drones.list[di].state, 0x32, "mid-sit");
    let steady = sim.drones.list[di].pos[1];
    sim.drones.list[di].pos[1] = steady + 0.15;
    sim.step_bond(Input::default(), false, false, false);
    let after = sim.drones.list[di].pos[1];
    assert!((after - (steady + 0.15 - 0.15 * 0.125)).abs() < 0.005,
        "the sitting driver settles correction × 0.125 per tick [M 0x140968 tail; L the trace's 39.305→39.289] — {} → {}", steady + 0.15, after);
    // Sister: the STANDING driver (the script done, hip restored to 1.0 [M 0x160bd0 case 5]) lifted 0.15
    // is NOT settled — the gate needs correction < 0.5 − 1.0 = −0.5 — and takes the port's snap.
    tick_idle(sim, 820); // tick 920: the sit done
    assert_eq!(sim.drones.list[di].state, 0x34, "the driver idles");
    let floor = {
        let g = &sim.drones.list[di];
        let from = g.pos;
        let h = sim.world.segment(from, [from[0], from[1] - 2.5, from[2]]).expect("the driver stands on floor");
        from[1] - 2.5 * h
    };
    sim.drones.list[di].pos[1] = floor + 1.0 + 0.15; // the feet 0.15 above the floor — inside the 0.2 on-surface band [M 0x140660]
    sim.step_bond(Input::default(), false, false, false);
    let after = sim.drones.list[di].pos[1];
    assert!((after - (floor + 1.0)).abs() < 0.005,
        "the standing hip is not settled by a 0.15 lift (the gate needs correction < −0.5 [M 0x140e30-48]) — the snap stands in: {floor} → {after}");
}
