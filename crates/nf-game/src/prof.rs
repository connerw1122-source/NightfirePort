//! A tiny scoped timer for finding where a tick's time goes (`NF_PROFILE=1`). Off by default: one relaxed
//! atomic load per scope. Never feeds back into the simulation, so determinism is unaffected.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static ON: OnceLock<bool> = OnceLock::new();
static TOTALS: Mutex<Vec<(&'static str, u64, u64)>> = Mutex::new(Vec::new());
static TICKS: AtomicU64 = AtomicU64::new(0);
static PRINTED: AtomicBool = AtomicBool::new(false);
static CAPTURE: AtomicBool = AtomicBool::new(false);
static SAMPLES: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

pub fn enabled() -> bool { *ON.get_or_init(|| std::env::var_os("NF_PROFILE").is_some()) }

/// Test hook: turn profiling on without the env var (`ON` may already be initialised by an
/// earlier call in the process). Timing only — never feeds back into the simulation.
pub fn force_enable() { let _ = ON.set(true); }

/// While on, every scope's per-call elapsed nanoseconds are pushed to the sample log
/// (`take_samples`), so a probe can compute per-phase percentiles, not just averages. Off by
/// default; the flag only gates this extra push and never touches the simulation.
pub fn capture(on: bool) { CAPTURE.store(on, Ordering::Relaxed); }

/// The samples captured since the last call: one `(scope name, ns)` pair per scope lifetime,
/// in call order. A caller that wraps each tick can note `take_samples().len()` (or the total
/// push count) before and after a tick to slice the log per tick.
pub fn take_samples() -> Vec<(&'static str, u64)> { std::mem::take(&mut SAMPLES.lock().unwrap()) }

/// Clear the totals, the sample log and the tick counter (a probe's run boundary — without it
/// the every-600-ticks auto-print fires mid-run from whatever counted before).
pub fn reset() {
    TOTALS.lock().unwrap().clear();
    SAMPLES.lock().unwrap().clear();
    TICKS.store(0, Ordering::Relaxed);
    PRINTED.store(false, Ordering::Relaxed);
}

/// Times its own lifetime under `name` when profiling is on.
pub struct Scope { name: &'static str, start: Option<Instant> }

pub fn scope(name: &'static str) -> Scope { Scope { name, start: enabled().then(Instant::now) } }

impl Drop for Scope {
    fn drop(&mut self) {
        let Some(t) = self.start else { return };
        let ns = t.elapsed().as_nanos() as u64;
        let mut v = TOTALS.lock().unwrap();
        match v.iter_mut().find(|e| e.0 == self.name) { Some(e) => { e.1 += ns; e.2 += 1; } None => v.push((self.name, ns, 1)) }
        if CAPTURE.load(Ordering::Relaxed) { SAMPLES.lock().unwrap().push((self.name, ns)); }
    }
}

/// Call once per sim tick: every 600 ticks prints the average milliseconds per tick of each scope, then resets.
pub fn tick() {
    if !enabled() { return; }
    let n = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    if n % 600 != 0 { return; }
    let mut v = TOTALS.lock().unwrap();
    let mut rows: Vec<_> = v.iter().map(|&(k, ns, c)| (k, ns as f64 / 600.0 / 1e6, c)).collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    eprintln!("[profile] ms per tick over the last 600 ticks:");
    for (k, ms, c) in rows { eprintln!("  {k:<28} {ms:8.3}  ({c} calls)"); }
    v.clear();
    PRINTED.store(true, Ordering::Relaxed);
}
