//! Input record/replay determinism tests (B-5).
//!
//! These tests are opt-in via `NF_ISO` (the disc image path), like the other `_live` tests. They synthesise a short
//! recording — a fixed input sequence — by building a `Replay` from a freshly loaded scene's starting state, then
//! replay it TWICE through the real `Sim` (no GPU, no window) and assert the two runs produce IDENTICAL Bond
//! position, weapon, ammo and RNG state at EVERY tick. If they diverge, the test prints the first divergent tick and
//! the differing fields so the cause (frame-rate-dependent input, wall-clock time, HashMap iteration order, …) can be
//! found.
//!
//! A second test exercises the file-format round-trip: serialise the recording to bytes, re-parse it, and replay —
//! asserting the re-parsed replay produces the same states as the in-memory one.
//!
//! Replay files committed under `tests/replays/` hold input only (never game bytes); they are safe to commit and
//! share.

use nf_view::replay::{Replay, TickInput, TickState};

/// The level used for the determinism tests (07000005 — the same one the live AI tests use).
const LEVEL: &str = "07000005.bin";

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A short, fixed input sequence: 60 ticks. Ticks 0..30 hold W (forward) with a small turn; tick 30 presses the
/// trigger (and holds it through 40); ticks 40..60 hold S (backward). This exercises movement, aim/fire and the
/// weapon's clip/ammo + the shared RNG (guard vision rolls happen each tick).
fn sample_ticks() -> Vec<TickInput> {
    let mut t = Vec::with_capacity(60);
    for i in 0..60u32 {
        let mut tk = TickInput::default();
        if i < 30 { tk.forward = 1.0; tk.turn = 0.01; }
        else if i < 40 { tk.trigger = true; }
        else { tk.forward = -1.0; }
        // A reload on tick 45 (one-shot) — exercises the gun's reload path.
        if i == 45 { tk.reload = true; }
        t.push(tk);
    }
    t
}

/// Compare two state traces; on the first difference, print a detailed diff and return false.
fn traces_match(a: &[TickState], b: &[TickState]) -> bool {
    if a.len() != b.len() {
        eprintln!("trace length differs: {} vs {}", a.len(), b.len());
        return false;
    }
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            eprintln!("DIVERGENCE at tick {i}:");
            eprintln!("  run A: pos={:?} yaw={} weapon={:#x} clip={} spare={} rng=({:#x},{:#x}) frame={}",
                x.bond_pos, x.bond_yaw, x.weapon, x.clip, x.spare, x.rng_x, x.rng_y, x.frame);
            eprintln!("  run B: pos={:?} yaw={} weapon={:#x} clip={} spare={} rng=({:#x},{:#x}) frame={}",
                y.bond_pos, y.bond_yaw, y.weapon, y.clip, y.spare, y.rng_x, y.rng_y, y.frame);
            return false;
        }
    }
    true
}

#[test]
fn replay_is_deterministic_two_runs_identical() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let ticks = sample_ticks();
    // Run 1: load, build the recording's header from the starting state, replay.
    let mut scene_a = nf_view::load(&iso, LEVEL, false).expect("load scene A");
    let replay = Replay::from_scene_and_ticks(&scene_a, LEVEL, None, false, ticks.clone())
        .expect("build replay");
    let states_a = nf_view::replay::replay_headless(&mut scene_a, &replay).expect("replay A");
    // Run 2: a FRESH load (so the starting state is reproduced from the disc, not cloned), same recording, replay.
    let mut scene_b = nf_view::load(&iso, LEVEL, false).expect("load scene B");
    let states_b = nf_view::replay::replay_headless(&mut scene_b, &replay).expect("replay B");
    assert!(states_a.len() == states_b.len(), "tick counts differ: {} vs {}", states_a.len(), states_b.len());
    assert!(traces_match(&states_a, &states_b),
        "replay diverged between two runs — the Sim is not deterministic for the same input sequence");
    // Sanity: the replay actually advanced the sim (Bond moved / the RNG stepped). If every tick were identical to
    // the start, the test would pass trivially while proving nothing.
    let first = &states_a[0]; let last = &states_a[states_a.len() - 1];
    assert!(first.frame == 0, "trace[0] should be the pre-tick start (frame 0), got {}", first.frame);
    assert!(last.frame == 60, "trace[60] should be after 60 ticks (frame 60), got {}", last.frame);
    assert!(last.rng_x != first.rng_x || last.rng_y != first.rng_y,
        "RNG did not advance over 60 ticks — the replay did not actually run the sim");
}

#[test]
fn replay_file_format_round_trips() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let ticks = sample_ticks();
    let mut scene = nf_view::load(&iso, LEVEL, false).expect("load scene");
    let replay = Replay::from_scene_and_ticks(&scene, LEVEL, None, false, ticks.clone())
        .expect("build replay");
    // Serialise → bytes → re-parse, and check the header + ticks survive intact.
    let bytes = replay.to_bytes();
    let re_parsed = Replay::from_bytes(&bytes).expect("re-parse");
    assert_eq!(re_parsed.header.level, replay.header.level);
    assert_eq!(re_parsed.header.rng_x, replay.header.rng_x);
    assert_eq!(re_parsed.header.rng_y, replay.header.rng_y);
    assert_eq!(re_parsed.header.bond_pos, replay.header.bond_pos);
    assert_eq!(re_parsed.header.start_weapon, replay.header.start_weapon);
    assert_eq!(re_parsed.ticks.len(), replay.ticks.len());
    assert_eq!(re_parsed.ticks, replay.ticks, "tick inputs changed across serialise/parse");
    // The re-parsed recording must replay to the SAME states as the original.
    let states_orig = nf_view::replay::replay_headless(&mut scene, &replay).expect("replay orig");
    let mut scene2 = nf_view::load(&iso, LEVEL, false).expect("load scene 2");
    let states_reparsed = nf_view::replay::replay_headless(&mut scene2, &re_parsed).expect("replay re-parsed");
    assert!(traces_match(&states_orig, &states_reparsed),
        "re-parsed replay produced different states than the original — the format is lossy");
}

#[test]
fn replay_commit_sample_file() {
    // Generates a committed sample replay under tests/replays/ (inputs only — no game bytes) so the format has a
    // real, inspectable artefact. This test writes the file when NF_ISO is set; CI without the disc skips it. The
    // file is .gitignored from game bytes but the replay format is inputs-only, so committing it is fine.
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping sample generation"); return };
    let ticks = sample_ticks();
    let scene = nf_view::load(&iso, LEVEL, false).expect("load scene");
    let replay = Replay::from_scene_and_ticks(&scene, LEVEL, None, false, ticks).expect("build");
    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/replays/sample-07000005-60tick.nfr");
    replay.save(out).expect("save sample replay");
    eprintln!("wrote sample replay: {out} ({} ticks, {} bytes)", replay.tick_count(), replay.to_bytes().len());
}
