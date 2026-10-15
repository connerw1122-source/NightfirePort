//! fix-gate-guard-truck: the 07000005 truck-ride exit's gate guard (GAPS G-037, U-048's channel 0x22).
//!
//! The level-end chain by truck, every link now evidenced on disc + decompile:
//!   1. SP #37 (placement 37, scripts [0x06000A22], mode 1) auto-plays the truck script from level load
//!      [D placement params; M SP_CreateScriptPlayer 0x1db9a8's create-time load]. Its stream 8 creates
//!      the TouchOnce ch 0x5C plate at the gatehouse porch (entity 02000480, quad x 95.98–101.00 /
//!      z 103.94–108.78 at y 27.05 [L the port's live trigger at creation t1884]); its streams 6/7 create
//!      the park plate ch 0x70 and the bed-following Touch ch 0x71; stream 9 fires event 0xD [0x73,1].
//!   2. Bond walks the cold start #26 (104.95, 25.12, 108.83 [D kind-0x24 placement]) onto that porch
//!      (the courtyard floor 23.8–23.9 steps ~0.9 up to the porch 24.8 [D world floors]) and stands on
//!      the plate: the script creates it at its tick 1884 [D the 05-truck trace's hand-off t1884] and the
//!      touch fires ch 0x5C the same tick → SP #38 (0x060008C8, out 0x5C) plays the hand-off cut-scene
//!      (event 4 PlayerDisable, event 6 the player hand-off [D U-048]) that puts Bond on the truck bed.
//!   3. THE GATE GUARD (the task's unknown, U-048): ch 0x22 is written by TouchOnce #21/#22/#23
//!      (out 0x23/0x24/0x25, mask 0x8, at (60.15, 28.42, 96.43)/(61.85, 26.53, 82.04)/(58.73, 27.13,
//!      110.70) [D]) OR-ed by mux #25 (0xEF: OR(0x23,0x24,0x25) → 0x22 [M Trigger_Update 0x1b1a78 case
//!      0xc]), and by the valley TouchOnce #305 (out 0x22 direct). NO type-7 script writes 0x22 [D the
//!      script scan]; Drone_InitComms 0x139520 writes only the alarm pair 0x97/0x98 [M]. The READERS:
//!      spawner #268 (mode 2, group 2, start = ext+0x0e = 0x22, kill 0x43, templates = gate guards pl 3
//!      (skin 0x05000030) + pl 4 (0x050002E) at their post (28.34/32.06, 37.12, 44.86) [D]) gates on
//!      switch_channels[0x22] [M DroneSpawner_Control 0x137e68 ext+0x0e], and refuses a spawn whose
//!      position is visible [M DroneSpawner_SpawnDrone 0x137cb0 → Drone_PositionVisible]; the spawned
//!      guards wait in WaitSwitch for their placement's switch channel (param 3 = 0x22 [M
//!      NDrone2_DSTATE_Global 0x15b830 → WaitSwitch 0x15b960]). The ride itself crosses plate #21's
//!      volume (the truck's path passes (61, 28.3, 96.6) inside lo [59.2, 26.0, 94.0]–hi [61.1, 30.8,
//!      98.9]) — so a from-the-start ride DOES raise ch 0x22 mid-ride (the core-loop port-map's "riding
//!      straight to the truck never spawns the guard" is disproved by this test's ch-0x22 tick), the
//!      guard spawns and stands at post, and Touch #318 (mask 0x10 = drone, at (28.36, 36.49, 43.90))
//!      raises ch 0x72 [D placement].
//!   4. Mux #316 = AND(0x70, 0x71, 0x72) → ch 0x65 → SP #320 (0x06000A23 "EndLevel", watch 0x65) fires
//!      event 0x11 [0x98,0x98,0x1D,1] at its tick 300 → ch 0x1D → placement 118 LOAD_LEVEL (gate 0x1D)
//!      → level_exit 0x07000006 [D U-048; M Trigger_Update case 10 0x1b1a78 → Trigger_Activate 0x1b2318].
//!
//! The trace's chain order [D tests/traces/05-truck.trace]: 0x72 (rel −317) → 0x70+0x65 (−300) → 0x73
//! (−273) → 0x1D (0). The port parks Bond at y 37.93 against the trace's 37.887 (the crouched ride, [D
//! fix-truck]).
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test gate_guard_truck_live -- --nocapture --test-threads=1`

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use walker::Walker;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The real game's parked Bond (player obj+0x30, trace frame 37173) [D 05-truck.trace].
const GAME_PARK: [f32; 3] = [29.885, 37.887, 51.656];

/// The full truck-ride integration from the LEVEL START, every tick through the real input path
/// (`Sim::step_bond` + `Input`; no `bond.pos` writes after the spawn): the cold start → the walk to
/// the gatehouse porch → the climb up its ~0.9 step → the stand on the script-created ch-0x5C plate →
/// the hand-off cut-scene → the crouched ride → the park → the chain → the LOAD_LEVEL of 07000006.
#[test]
fn cold_start_walks_the_gatehouse_and_the_truck_ride_ends_the_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    // The hand-off cut-scene must PLAY (the game's own beat): no skip — the level start has no
    // blocking intro on this level and the route crosses no other cut-scene plate.
    w.skip_cutscenes = false;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 104.95).abs() < 2.0 && (p[2] - 108.83).abs() < 2.0,
            "the cold start is placement #26 (104.95, 25.12, 108.83) [D], got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
    }
    w.wait(30);
    // The courtyard by the porch's south edge (floor 23.8–23.9).
    w.walk_nav_to([102.0, 25.0, 110.5], "courtyard by the gatehouse porch", 1.6);
    // Up the ~0.9 porch step with the player's own stair move (run + hop), then onto the plate.
    assert!(w.climb_stairs([99.5, 26.0, 107.0], "onto the gatehouse porch", 1.0, 600),
        "Bond must climb the gatehouse porch step");
    w.walk_to(walker::Wp::new(99.0, 26.0, 106.6).r(0.7), "onto the ch-0x5C plate");
    let stand_pos = w.scene.sim.as_ref().unwrap().bond.pos;
    eprintln!("[gate] on the plate at {:?} (world tick {})", stand_pos, w.total_ticks);

    // Stand: the truck script creates the ch-0x5C plate at its tick 1884 [D trace] and the stood
    // Bond (capsule 0.55 around pos+0.15 [M Player_Collision 0x194df0]) touches the y-27.05 quad.
    let mut saw_5c = 0u32;
    for _ in 0..2400u32 {
        w.tick(Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x5c) { saw_5c = w.total_ticks; break; }
    }
    assert!(saw_5c > 0, "standing on the porch must fire the script-created ch-0x5C plate (SP #37 stream 8, tick 1884)");
    eprintln!("[gate] ch 0x5C on at world tick {saw_5c} — the hand-off cut-scene");

    // The hand-off plays out; when Bond is re-enabled below x 95 he is on the truck bed — crouch
    // like the trace's ride [D fix-truck] (a standing capsule misses the ch-0x70 park plate).
    let mut first_on: [Option<u32>; 7] = [None; 7]; // 0x22 0x72 0x70 0x65 0x73 0x1d 0x5c
    let chs = [0x22u32, 0x72, 0x70, 0x65, 0x73, 0x1d, 0x5c];
    let mut prev_dis = true;
    let mut crouch_armed = false;
    let mut input = Input::default();
    for _ in 0..6000u32 {
        let sim = w.scene.sim.as_ref().unwrap();
        let dis = sim.disabled;
        let px = sim.bond.pos[0];
        if !dis && prev_dis && px < 95.0 { crouch_armed = true; }
        prev_dis = dis;
        input.crouch = if crouch_armed && !w.scene.sim.as_ref().unwrap().bond.crouched { crouch_armed = false; true } else { false };
        w.tick(input, false);
        let sim = w.scene.sim.as_ref().unwrap();
        for (k, ch) in chs.iter().enumerate() {
            if first_on[k].is_none() && sim.drones.channels.on(*ch) { first_on[k] = Some(w.total_ticks); }
        }
        if sim.drones.level_exit.is_some() { break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    let at = |k: usize| first_on[k];
    let (t22, t72, t70, t65, t73, t1d, t5c) = (at(0), at(1), at(2), at(3), at(4), at(5), at(6));
    eprintln!("[gate] first-on: 0x5C {t5c:?} 0x22 {t22:?} 0x72 {t72:?} 0x70 {t70:?} 0x65 {t65:?} 0x73 {t73:?} 0x1D {t1d:?}");
    eprintln!("[gate] park bond {:?} game {GAME_PARK:?}", sim.bond.pos);

    // The gate guard: the group-2 templates (pl 3 skin 0x05000030 / pl 4 skin 0x0500002E) must have
    // spawned and stood at post — ch 0x72 IS the guard-at-post channel (Touch #318, mask 0x10).
    let guard_out = sim.drones.list.iter()
        .filter(|d| d.group == 2 && d.present && (d.skin == 0x0500_0030 || d.skin == 0x0500_002e))
        .map(|d| d.placement).collect::<Vec<_>>();
    eprintln!("[gate] group-2 gate guards present: placements {guard_out:?}");
    assert!(!guard_out.is_empty(), "the gate-guard drone (spawner #268, group 2, start ch 0x22) must spawn");

    // The chain order against the trace: the hand-off (0x5C) → the guard activation (0x22, mid-ride
    // through courtyard plate #21) → the guard at post (0x72) → the park (0x70 with the mux 0x65 the
    // same tick) → the truck script's 0x73 → the EndLevel's 0x1D → the exit.
    let (t22, t72, t70, t65, t73, t1d) = (t22.expect("ch 0x22 must fire"), t72.expect("ch 0x72 must fire"),
        t70.expect("ch 0x70 must fire"), t65.expect("ch 0x65 must fire"), t73.expect("ch 0x73 must fire"),
        t1d.expect("ch 0x1D must fire"));
    assert!(t5c.unwrap() < t22, "the hand-off (0x5C) precedes the guard activation (0x22)");
    assert!(t22 < t72, "the guard activation (0x22) precedes the guard at post (0x72)");
    assert!(t72 < t70, "the guard at post (0x72) precedes the park plate (0x70)");
    assert!((t65 as i32 - t70 as i32).abs() <= 2, "the mux 0x65 fires with 0x70 (the AND completes the same tick)");
    assert!(t70 < t73, "the park (0x70) precedes the truck script's 0x73");
    assert!(t73 < t1d, "0x73 precedes the EndLevel's 0x1D (its event 0x11 at script tick 300)");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the ride must end the level into 07000006");
    // The park against the trace (the task's fix-truck bar: y within 0.05, xz within 0.5).
    let bond = sim.bond.pos;
    assert!((bond[1] - GAME_PARK[1]).abs() <= 0.05,
        "parked Bond y {:.3} must match the trace's 37.887 ±0.05 (the crouched cc)", bond[1]);
    assert!((bond[0] - GAME_PARK[0]).abs() < 0.5 && (bond[2] - GAME_PARK[2]).abs() < 0.5,
        "parked Bond xz {:?} vs the game {GAME_PARK:?}", &bond[0..1]);
}

/// Sister: Bond walks to the gatehouse porch's EDGE but never onto the script-created ch-0x5C plate
/// (he stays east of its footprint, x ≤ 101.0 [L the live trigger's bounds]) — the hand-off never
/// fires, the truck never boards him, and the level never ends, though the truck script still runs
/// (the plate appears at its tick 1884 either way). Proves the plate stand is load-bearing.
#[test]
fn without_the_plate_stand_the_handoff_never_fires() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    w.skip_cutscenes = false;
    w.wait(30);
    w.walk_nav_to([102.0, 25.0, 110.5], "courtyard by the gatehouse porch", 1.6);
    // Stand EAST of the plate's footprint for 2600 ticks (past the t1884 creation).
    let mut saw_5c = false;
    let mut plate_seen = false;
    for t in 0..2600u32 {
        w.tick(Input::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x5c) { saw_5c = true; break; }
        if t == 2000 { plate_seen = sim.drones.triggers.iter().any(|t| t.out == 0x5c); }
        if sim.drones.level_exit.is_some() { break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    eprintln!("[gate] sister: plate created {plate_seen} ch 0x5C {saw_5c} exit {:?}", sim.drones.level_exit);
    assert!(!saw_5c, "Bond east of the plate footprint must not fire ch 0x5C");
    assert_eq!(sim.drones.level_exit, None, "without the hand-off the level cannot end by truck");
}
