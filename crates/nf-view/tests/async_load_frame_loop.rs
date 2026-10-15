//! The frame loop during a level load (async-level-load): headless, no ISO.
//!
//! The window thread's pending-load branch is `App::poll_async_load`: while the worker builds
//! the scene, every frame redraws the loading screen with `loading::dots_lit(job.elapsed_ticks())`
//! and ends; the frame the result arrives, the scene swaps in. A real `App` needs the GPU, so the
//! loop here drives the exact pieces that branch calls — `Job::poll` + `Job::elapsed_ticks` +
//! `dots_lit` — at the redraw cadence, against a fake slow load. The game's own load runs the same
//! way on one thread: the main thread blocks in `ResetMap_Load` 0x1d6360 state 3 while
//! `VBlankInt` 0x124088 steps the six dots every 11 vblanks, ping-pong 0→6→0.
//!
//! The old model blocked the window thread inside the load, so the dots never moved — the user's
//! symptom this task fixes.

use nf_view::async_load::Job;
use nf_view::loading::dots_lit;

#[test]
fn the_frame_loop_keeps_drawing_dots_while_a_load_runs() {
    // A fake slow load: ~1 s ≈ 60 vblanks at 60 Hz — one full 0→6→0 sweep of the game's cadence.
    let mut job: Job<u32> = Job::spawn(|| { std::thread::sleep(std::time::Duration::from_millis(1000)); 1 });
    let mut frames = 0u32;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        match job.poll() {
            None => {
                // The pending branch's draw: the screen redraws with the dots advanced.
                seen.insert(dots_lit(job.elapsed_ticks()));
                frames += 1;
                std::thread::sleep(std::time::Duration::from_millis(16)); // the redraw cadence
            }
            Some(Some(v)) => { assert_eq!(v, 1, "the fake load's value must arrive intact"); break; }
            Some(None) => panic!("the fake load must not look panicked"),
        }
    }
    // Sister assertions (each can fail if the plumbing breaks):
    assert!(frames >= 20, "1 s of load at ~16 ms frames must keep the loop ticking; got {frames} pending frames");
    assert!(seen.iter().any(|&d| d > 0), "the dot count must advance past 0 during the load; saw {seen:?}");
    assert!(seen.len() >= 3, "the sweep must pass through several dot counts over 1 s; saw {seen:?}");
}

#[test]
fn a_fast_load_swaps_in_as_soon_as_it_is_ready() {
    // The near-instant case (a cached or tiny archive): the loop takes the result the first
    // poll that finds it, and never drops or re-orders it. The worker is a real thread, so the
    // exact poll that lands is racy — the contract is arrival + a drained tail, not poll #1.
    let mut job: Job<&'static str> = Job::spawn(|| "scene");
    let mut got = None;
    for _ in 0..400 {
        if let Some(v) = job.poll() { got = Some(v); break; }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(got, Some(Some("scene")), "a finished worker's value must arrive through the loop");
    // Sister: an empty poll after the take stays None (the drained job).
    assert_eq!(job.poll(), None, "a drained job must keep the loop's next polls pending-free");
}
