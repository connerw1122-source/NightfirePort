//! G-043 120 fps: the render rate must not change the simulation. The winit loop's tick scheduler
//! (`nf_view::ticks_for_frame`) is the exact arithmetic the `RedrawRequested` handler runs (extracted
//! verbatim from the loop); these tests drive it at 60 Hz and 144 Hz frame times over the same
//! wall-clock span against a real level's sim and assert identical sim states — the "no gameplay
//! change at any frame rate" contract. The interpolation sister (`lerp_yaw`) pins the camera half.
//! Opt-in on the real level (07000006, G-043's subject): NF_ISO.

use nf_view::ticks_for_frame;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// One headless render loop: `fps` frames per simulated second, each frame scheduling its ticks the
/// way the real loop does and stepping the sim once per scheduled tick. Returns the per-frame tick
/// counts plus the sim state at the end (frame counter, Bond's head position, health).
fn run_loop(scene: &mut nf_view::scene::Scene, fps: u32, seconds: f32, max_ticks: u32) -> (Vec<u32>, u32, [f32; 3], f32, u32) {
    let dt = 1.0 / fps as f32;
    let frames = (seconds * fps as f32).round() as u32;
    let mut anim_time = 0.0f32;
    let mut per_frame = Vec::new();
    for _ in 0..frames {
        let n = ticks_for_frame(&mut anim_time, dt, max_ticks);
        for _ in 0..n { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        per_frame.push(n);
    }
    let sim = scene.sim.as_ref().unwrap();
    let head = sim.bond.head();
    let (health, frame) = (sim.drones.health.health, sim.drones.frame);
    (per_frame, frame, head, health, anim_time as u32)
}

/// The named contract: a 60 Hz render loop and a 144 Hz render loop over the same wall-clock span
/// produce identical sim states. 4.005 s (not a whole tick multiple) keeps both accumulators clear
/// of the tick boundary by ~0.3 tick — f32 drift over 576 frames is ~1e-4, three orders smaller.
#[test]
fn render_loop_rate_does_not_change_the_simulation() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut a = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let mut b = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let (fa, frame_a, head_a, health_a, anim_a) = run_loop(&mut a, 60, 4.005, 4);
    let (fb, frame_b, head_b, health_b, anim_b) = run_loop(&mut b, 144, 4.005, 4);
    assert_eq!(frame_a, frame_b, "the same wall time must run the same tick total at 60 and 144 Hz");
    assert_eq!(frame_a, 240, "4.005 s of wall time is 240 whole ticks at any frame rate");
    assert_eq!(head_a, head_b, "Bond must stand in the same place");
    assert_eq!(health_a, health_b, "health must match");
    assert_eq!(fa.iter().sum::<u32>(), fb.iter().sum::<u32>());
    assert_eq!(anim_a, anim_b);
    // The scheduler's shape: at 60 Hz every frame runs exactly 1 tick; at 144 Hz ticks arrive
    // one at a time across ~2.4 frames (0.4167 per frame) — never a burst, never a skip > 1.
    assert!(fa.iter().all(|&n| n == 1), "60 Hz frames run exactly one tick each: {fa:?}");
    assert!(fb.iter().all(|&n| n <= 1), "144 Hz frames must not burst ticks: {fb:?}");
    assert!(fb.iter().filter(|&&n| n == 0).count() > 100, "most 144 Hz frames run 0 ticks (the pose cache's no-tick frames)");
}

/// The catch-up cap (fix-perf-level, `NF_MAXTICKS` = 4): a 5 fps loop asks for 12 ticks per frame
/// and is dropped to 4 — under load the game slows down (like the original), it never bursts.
/// The dropped ticks are gone (total < wall-time ticks), and the same loop is deterministic.
#[test]
fn slow_frames_drop_ticks_never_burst() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut a = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let (fa, frame_a, _, _, _) = run_loop(&mut a, 5, 4.0, 4);
    assert_eq!(fa.len(), 20, "5 fps for 4 s is 20 frames");
    assert!(fa.iter().all(|&n| n == 4), "every 5 fps frame is capped at 4 ticks (12 requested): {fa:?}");
    assert_eq!(frame_a, 80, "capped ticks are dropped, not deferred: 20 x 4");
}

/// Sister (no ISO): the scheduler's boundary arithmetic — one tick per 1/60 s frame, none inside a
/// tick, and the cap applies before the count is handed to the sim.
#[test]
fn scheduler_boundary_arithmetic() {
    let mut t = 0.0f32;
    assert_eq!(ticks_for_frame(&mut t, 1.0 / 60.0, 4), 1);
    assert_eq!(ticks_for_frame(&mut t, 1.0 / 120.0, 4), 0, "half a tick: no whole tick yet");
    assert_eq!(ticks_for_frame(&mut t, 1.0 / 120.0, 4), 1, "the second half-frame crosses the boundary");
    let mut t = 0.0f32;
    assert_eq!(ticks_for_frame(&mut t, 0.2, 4), 4, "a 5 fps frame asks 12, is capped at 4");
    assert_eq!(t as u32, 12, "the clock still advanced by dt·60 (12); only the run was capped");
    let mut t = 10.0f32;
    assert_eq!(ticks_for_frame(&mut t, -1.0, 4), 0, "a negative dt (should not happen) runs nothing");
}

/// Sister (no ISO): the interpolation's yaw half takes the short way round the ±π wrap — a Bond
/// turning from 3.1 to −3.1 rad must not sweep the long way through 0 in an interpolated frame.
#[test]
fn lerp_yaw_takes_the_short_way_round() {
    let mid = nf_view::lerp_yaw(3.1, -3.1, 0.5);
    assert!((mid - std::f32::consts::PI).abs() < 0.01 || (mid + std::f32::consts::PI).abs() < 0.01,
        "halfway from 3.1 to -3.1 is ±π, not 0: got {mid}");
    assert_eq!(nf_view::lerp_yaw(0.1, -0.1, 0.5), 0.0);
    assert!((nf_view::lerp_yaw(0.0, std::f32::consts::PI - 0.1, 1.0) - (std::f32::consts::PI - 0.1)).abs() < 1e-6);
    let end = nf_view::lerp_yaw(0.0, std::f32::consts::PI + 0.1, 1.0);
    let diff = (end - (std::f32::consts::PI + 0.1) + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    assert!(diff.abs() < 1e-6, "t = 1.0 lands on b's direction even across the wrap: got {end}");
}
