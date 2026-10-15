//! The worker-thread level load (async-level-load).
//!
//! The game loads a level on its main thread while the six LOADING dots keep animating:
//! `ResetMap_Load` 0x1d6360 state 3 draws the screen (`ResetMap_GenLoadScreen` 0x1d5de0 — skipped
//! for the front end 0x07000048), runs the whole blocking build (`LoaderLoad` type-0 loop, then
//! type-1, `Player_Start` 0x190fd0, `psiPreGameRun` 0x11e2e8 …) on the calling thread, and only
//! then calls `psiEndLoadingScreen` 0x10ba28 (which just clears `PS2LSActive`). The animation
//! comes from the vblank interrupt, not the loader: `VBlankInt` 0x124088 steps `BlobToDraw` once
//! every 11 vblanks (`BlobTime` 10 → negative → reset) and ping-pongs it 0→6→0 while
//! `PS2LSActive == 2` — set by `psiAllocateLoadingBlobs` 0x11f270 after `psiStartLoadingScreen`
//! 0x10b9f8. The port's old model blocked the window thread in the load, so the screen showed the
//! dots' first state only; [`Job`] moves the build to a worker thread and lets the window thread
//! keep redrawing the screen with `loading::dots_lit` over the load's elapsed 60 Hz vblanks.

/// A load running on a worker thread: `spawn` the build, `poll` it from the frame loop.
///
/// The frame loop calls [`Job::elapsed_ticks`] for the dot count and [`Job::poll`] once per frame;
/// the result arrives as `Some(Some(value))`, a panicked worker surfaces as `Some(None)` (the
/// window thread reports it and keeps the old scene — the sync load's hard crash becomes the
/// failed-load path instead), and `None` keeps the loop drawing. The thread is named for the
/// debugger's sake.
pub struct Job<T> {
    rx: std::sync::mpsc::Receiver<Option<T>>,
    started: std::time::Instant,
    /// Set once `poll` has delivered a terminal result (a value or a worker death): a drained
    /// channel afterwards means "done", never "died" — `try_recv` can't tell them apart.
    done: bool,
}

impl<T: Send + 'static> Job<T> {
    pub fn spawn(run: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("level-load".into()).spawn(move || {
            // The default panic hook still prints the panic; `None` is the frame loop's signal.
            let _ = tx.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).ok());
        });
        if spawned.is_err() {
            // The OS refused the thread: `tx` died with the never-run closure, and `poll`'s
            // Disconnected arm surfaces the loss the same way as a panicked worker (the sync
            // load's failure mode — a frozen window — is strictly worse).
            eprintln!("level-load: worker thread unavailable; the load is lost");
        }
        Self { rx, started: std::time::Instant::now(), done: false }
    }

    /// The test seam: the "worker" runs synchronously, `poll` still reads it off the channel —
    /// the frame loop's contract is the same either way.
    pub fn inline(run: impl FnOnce() -> T) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Some(run()));
        Self { rx, started: std::time::Instant::now(), done: false }
    }

    /// The load's elapsed time in 60 Hz vblank units — the game counts the dots in vblanks
    /// (`VBlankInt` 0x124088 steps every 11), and 11 vblanks at 60 Hz is the wall-clock cadence
    /// the window's redraws stand in for. `loading::dots_lit` is its consumer.
    pub fn elapsed_ticks(&self) -> u32 { (self.started.elapsed().as_secs_f64() * 60.0) as u32 }

    /// `None` while the worker still runs; `Some(None)` = it panicked (or never started);
    /// `Some(Some(value))` = done. After a terminal result the job is drained: every later
    /// poll is `None` (the App never re-polls a taken job, but the contract must not depend
    /// on that).
    pub fn poll(&mut self) -> Option<Option<T>> {
        if self.done { return None; }
        match self.rx.try_recv() {
            Ok(v) => { self.done = true; Some(v) }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            // The sender dropped without a queued value: the worker died (panicked and the
            // send never ran, or the thread never started). Not "pending" — the loop must not
            // wait forever on a load that can no longer produce a scene.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => { self.done = true; Some(None) }
        }
    }
}

// The worker sends the built scene across the thread boundary: Scene is plain data (Vecs,
// HashMaps, the Sim), so `Send` holds — this is the compile-time proof, re-checked on every build.
#[allow(dead_code)]
fn _assert_scene_is_send() { fn assert_send<T: Send>() {} assert_send::<crate::scene::Scene>(); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_delivers_value_and_ticks_advance() {
        let mut j = Job::spawn(|| { std::thread::sleep(std::time::Duration::from_millis(120)); 42u32 });
        let mut saw_pending = false;
        let t0 = j.elapsed_ticks();
        loop {
            match j.poll() {
                None => { saw_pending = true; std::thread::sleep(std::time::Duration::from_millis(20)); }
                Some(Some(v)) => {
                    assert_eq!(v, 42, "the worker's value must arrive through the channel");
                    break;
                }
                Some(None) => panic!("the fake load must not look panicked"),
            }
        }
        // The dot count advanced across the pending polls (the load did not stall the ticker);
        // the sister assertion pins a real gap, not just t0 == 0 luck.
        let t1 = j.elapsed_ticks();
        assert!(saw_pending, "a 120 ms fake load must show at least one pending poll");
        assert!(t1 > t0, "the elapsed vblank count must advance during the load ({t0} → {t1})");
        assert!(t1 >= 5, "120 ms ≈ 7 vblanks at 60 Hz; got {t1}");
    }

    #[test]
    fn job_surfaces_a_panic_as_none() {
        let mut j = Job::spawn(|| -> u32 { panic!("fake load blow-up") });
        let mut r = None;
        for _ in 0..200 {
            if let Some(p) = j.poll() { r = Some(p); break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(r, Some(None), "a panicked worker must surface as Some(None), never hang the loop");
        // Sister: after the panic the channel stays drained (no repeat, no stuck value).
        assert_eq!(j.poll(), None, "a drained job must keep returning None");
    }

    #[test]
    fn inline_job_polls_like_a_worker() {
        let mut j = Job::inline(|| 7u32);
        assert_eq!(j.poll(), Some(Some(7)), "the inline seam must behave like a finished worker");
        assert_eq!(j.poll(), None, "and stay drained afterwards");
    }
}
