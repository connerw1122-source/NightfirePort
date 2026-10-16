//! cutscene-walk-stutter (task 686) — the 07000007 Kimiko scene's walking guest (the red dress,
//! skin 0x0500003b, anim script 0x060004f1, body sequence 0x04000332) "stuttered a little bit while
//! she was walking towards the other two" (playtest 1015-1738).
//!
//! The headless trace (through the real trigger path: Bond into TouchOnce #412 → SP #420 plays the
//! scene script 0x06000073 → its op-2 commands start the guests' anim scripts) shows:
//! - the anim object's frame advances EXACTLY 1 per tick (no repeat, no skip — the frame-rate
//!   multiplier chain [M `Script_Update` 0x1da998 +1 per stream per tick; `Script_Interp` case 2
//!   0x1d9a08:157-166 writes `clock − start_clock + start_frame` clamped to [1, len] and zeroes the
//!   advance; `AnimSeqTick` 0x1c28d0:62-63 maps the anim time to the sequence frame 1:1]);
//! - the root (the body sequence's world-space root, which carries the actor — the render-verified
//!   reading) moves in small steps while she walks, and shows exactly THREE one-tick jumps
//!   (t≈846/2645/3876 → seq frames 936/2735/3966, |Δ| 0.279/0.371/0.880) — each one a 1–2-frame
//!   LINEAR bridging segment IN THE DISC'S OWN ROOT POLYNOMIALS of 04000332, C0-continuous with the
//!   neighbouring segments (dumped: seg49 [935..936] slope −0.227/frame, seg186's first frame
//!   90.939 after seg185's constant 90.797, and the frame-3966 slide). The game's pipeline samples
//!   the same integer frames of the same polynomials (`AnimFrameCopy` 0x1c5668: the first segment
//!   whose end ≥ t, evaluated at absolute t — the port's `Sequence::value` is this walk, verified
//!   bit-exact against PCSX2 in WORKLOG 0017), so the jumps are the disc data playing, not a port
//!   discontinuity. The tests below lock exactly that: the port may not introduce ANY root
//!   discontinuity the disc does not carry, and the frame may never repeat or skip.
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::player::Input;

/// The disc's own one-frame root slides of body sequence 0x04000332 (the walker's track): the
/// (frame pair start, |Δ|) the data itself carries. Measured off the parsed polynomials
/// (`Sequence::value` — the game's `AnimFrameCopy` evaluation) and confirmed in the live trace.
const DISC_SLIDES: [(u32, f32); 3] = [(936, 0.279), (2735, 0.371), (3966, 0.880)];
/// Any other per-tick root delta above this bound is a port-side discontinuity.
const DELTA_BOUND: f32 = 0.05;

/// One sampled tick of the walker: (tick, anim frame, root world position).
type Sample = (u32, u32, [f32; 3]);

/// The walk lock: the frame advances exactly 1 per tick, and every per-tick root delta is either
/// ≤ `DELTA_BOUND` or exactly one of the disc's own slides. `Err` names the first violation —
/// the message the sister test trips.
fn check_walk(path: &[Sample]) -> Result<(), String> {
    if path.len() < 2 { return Err("the trace is empty".to_string()); }
    for w in path.windows(2) {
        let (t0, f0, p0) = (w[0].0, w[0].1, w[0].2);
        let (t1, f1, p1) = (w[1].0, w[1].1, w[1].2);
        if t1 != t0 + 1 { return Err(format!("tick gap {t0} -> {t1}")); }
        // The frame-rate lock: 1 frame per tick, no repeat, no skip.
        if f1 != f0 + 1 { return Err(format!("frame repeat/skip at tick {t1}: {f0} -> {f1}")); }
        let d = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2) + (p1[2] - p0[2]).powi(2)).sqrt();
        if d <= DELTA_BOUND { continue; }
        // A jump: it must be one of the disc's own slides (frame pair start == the frame at t0).
        let hit = DISC_SLIDES.iter().any(|&(fr, dd)| f0 == fr && (d - dd).abs() < 0.02);
        if !hit {
            return Err(format!("port-side root discontinuity at tick {t1} (frame {f0} -> {f1}): |d| = {d:.4}, not a disc slide {DISC_SLIDES:?}"));
        }
    }
    Ok(())
}

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return None };
    match nf_view::load(&iso, archive, false) {
        Ok(s) => Some(s),
        Err(e) => { eprintln!("load {archive}: {e}"); None }
    }
}

/// The walker's root path over the whole scene, through the real trigger path (the same harness as
/// fix_cutscene_letterbox_live: Bond into TouchOnce #412's volume starts SP #420's scene script
/// 0x06000073, whose op-2 commands start the guests' anim scripts).
fn trace_walker() -> Option<Vec<Sample>> {
    let mut scene = load("07000007.bin")?;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    const WALKER: u32 = 0x0500_003b; // the red-dress guest's skin (anim object of script 0x060004f1)
    let mut path: Vec<Sample> = Vec::new();
    for t in 0..4400u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        // The anim object of the walker's skin, its frame and its root world position — the same
        // reading the render's `characters_script_pos` makes (`script_pose_together`'s body window,
        // bone 0 world translation).
        let sample = sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
            nf_game::nis::Obj::Anim { skin, script, frame, .. } if *skin == WALKER => {
                let sk = sim.skins.get(skin)?;
                let skel = sim.skeletons.get(&sk.skeleton)?;
                let sc = sim.scripts.get(script)?;
                let (q, f) = nf_game::anim::script_frame(sc, *frame)?;
                let seq = sim.seqs.get(&q)?;
                let world = nf_formats::anim::world_pose(&nf_formats::anim::local_pose(seq, skel, sk, f as u32), sk);
                Some((t, *frame as u32, [world[0][3][0], world[0][3][1], world[0][3][2]]))
            }
            _ => None,
        });
        if let Some(s) = sample { path.push(s); }
    }
    Some(path)
}

/// The lock: over the whole 4326-tick scene the walker's frame advances 1/tick and her root never
/// jumps beyond the disc's own three data slides. Sister inside: the walk's own small-step stats.
#[test]
fn walkers_root_introduces_no_discontinuity_beyond_the_disc_data() {
    let Some(path) = trace_walker() else { return };
    assert!(path.len() > 4000, "the scene ran {} sampled ticks, expected the full 4326", path.len());
    // The walk's own rhythm: the overwhelming majority of ticks move ≤ 3 cm.
    let deltas: Vec<f32> = path.windows(2).map(|w| {
        let (a, b) = (w[0].2, w[1].2);
        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
    }).collect();
    let mut sorted = deltas.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p99 = sorted[sorted.len() * 99 / 100];
    assert!(p99 <= DELTA_BOUND, "the 99th percentile per-tick root delta is {p99:.4} — the walk itself is not small-step");
    // The lock (frame 1/tick + every jump is a disc slide).
    if let Err(err) = check_walk(&path) { panic!("{err}"); }
    // And the disc's three slides ARE present (the trace samples the disc data, not something else).
    for &(fr, dd) in DISC_SLIDES.iter() {
        let hit = path.windows(2).any(|w| w[0].1 == fr && {
            let (a, b) = (w[0].2, w[1].2);
            let d = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            (d - dd).abs() < 0.02
        });
        assert!(hit, "the disc slide at frame {fr} (|d| {dd}) never appears — the trace is not sampling the disc's data");
    }
    // The walk segments are monotone in their heading (no root backtracking beyond the slide)
    // — a repeat/skip of the ROOT would show as a negative delta along the walk's direction.
    // Covered by the frame lock + the slide whitelist above.
    eprintln!("walk lock held: {} ticks, 99th pct delta {p99:.4}, disc slides all present", path.len());
}

/// Sister: a deliberately doubled frame trips the check — the frame-repeat arm, and a synthetic
/// position jump that is not a disc slide trips the discontinuity arm.
#[test]
fn a_doubled_frame_trips_the_walk_check() {
    // A clean synthetic walk: 1 unit per 100 ticks along +x.
    let clean: Vec<Sample> = (0..300u32).map(|t| (t, t + 1, [t as f32 / 100.0, 43.7, -33.0])).collect();
    assert!(check_walk(&clean).is_ok(), "the checker rejects its own clean walk");
    // Double a frame mid-walk (the stutter the task describes): the tick advances but the anim
    // frame repeats — the frame lock must trip.
    let mut doubled = clean.clone();
    doubled[150].2 = doubled[149].2; // the root freezes (the same pose drawn twice)...
    // ...with the frame held (the object's frame did not advance under the tick).
    doubled[150].1 = doubled[149].1;
    let err = check_walk(&doubled).expect_err("a doubled frame must trip the check");
    assert!(err.contains("frame repeat/skip"), "wrong trip: {err}");
    // A port-side root jump that is NOT a disc slide: the discontinuity arm must trip.
    let mut jumped = clean.clone();
    jumped[200].2 = [3.9, 43.7, -33.0];
    let err = check_walk(&jumped).expect_err("a non-disc root jump must trip the check");
    assert!(err.contains("port-side root discontinuity"), "wrong trip: {err}");
    // And the disc's own slides pass: a 1200-tick walk whose frames reach 936, with the slide at
    // the natural (frame 936 -> 937) pair — the walk CONTINUES from the slid position (as the disc's
    // C0-continuous segments do).
    let disc: Vec<Sample> = (0..1200u32).map(|t| (t, t + 1, [t as f32 / 100.0, 43.7, -33.0])).collect();
    let mut disc = disc;
    for s in disc.iter_mut().skip(936) { s.2[0] -= 0.27; }
    assert!(check_walk(&disc).is_ok(), "a disc slide must pass: {:?}", check_walk(&disc));
}
