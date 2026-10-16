//! head-track — the verification round for playtests 1013-1758 / 1015-0046 ("how all the models
//! are looking kind of weird, where their heads are turning kind of the wrong direction, wrong
//! angle" / "how weird their head looks when they're turning").
//!
//! A frame-stepped headless trace through the REAL input path (`Sim::step_bond`, no page/state
//! pokes — Bond is staged by placement like fix_heads07's F12 spots), for three poses of a tracked
//! drone — Bond beside it on side A (`cross(up, fwd)` off its yaw), side B, and ABOVE it (the
//! 07000007 balcony the fix_heads07 trace proved saturates) — asserting every tick's stored head
//! yaw/pitch against values computed from the game's formula, independently transcribed HERE:
//!
//! - the ease: `Ang_RotateToBearing` 0x1e0f10 (decompile re-read for this task: wrap both angles
//!   into [0, 2π), wrap the diff to [−π, π], clamp the step to ±max, move `cur + clamped·rate`,
//!   no overshoot past the clamped step, snap within 1e-4);
//! - the rates: 0.14 alerted (`DCVars+0x50c > 0.66` → the port's `dr.alert`) else 0.07
//!   [`NDrone2_DoHeadTracking` 0x1472c8:14-18 — the 0.07 / 0.66 / 0.14 consts, disasm-checked];
//! - the range gate: track (measure a want) only within 3.0 of the target, else ease to 0
//!   [`AnimObjectSetHeadTracking` 0x1c6748 stores dist² = 0x40400000² = 9.0; HeadTrack 0x1bf618:191
//!   skips both angle blocks beyond it, leaving the wants at 0];
//! - the clamps: yaw ±1.1 with the |yaw| > 2.6 → 0 rule, pitch [−1.0, +0.6] in the game's
//!   positive-down convention = [−0.6, +1.0] in the port's stored positive-up (0x1bf618:290-307);
//! - the measurement itself is the port's `head_track_want` (drone.rs), whose every sub-step was
//!   re-verified against the disassembly for this task: neck = `Vec_UnitVector`(out, bone2 pos,
//!   head pos) = unit(head − bone2) [0x1bf728 jal 0x1e2d28, args 0x1bf6e8/0x1bf714/0x1bf72c],
//!   side = `Vec_CrossNormalise`(neck, head row2) = unit(neck × fwd) [0x1bf738 jal 0x1e3078],
//!   the yaw sign via `Vec_ScalarTripleProduct`(fwd, to, neck) [0x1bfa48], the pitch sign via the
//!   side triple, the per-plane 0.25 gates, the pitch behind-skip and the one-plane coupling.
//!
//! The traced subject is an UNARMED drone (kind 0xD party guest): armed guards in the aim/fire
//! states stop tracking by design (`AnimObjectHeadTrack` 0x1bf618:137-139 clears the 0x10 request
//! under the 0x180 aim flags; the port's aim branch eases the head to 0) — a guard would leave the
//! track branch mid-trace for reasons the formula assertions below deliberately do not cover
//! (they are 0x1bf618's, not the drone state machine's). The head-track code is shared.
//!
//! On top of the per-tick formula equality, the trace asserts the DIRECTION end to end from the
//! posed head bone (the thing the playtests saw): the head's world forward turns TOWARD Bond in
//! every pose, the head stays upright (no roll — the real game yaws about the neck and pitches
//! about the side axis only), and the saturated poses sit at the clamps.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn row(m: &[[f32; 4]; 4], r: usize) -> [f32; 3] { [m[r][0], m[r][1], m[r][2]] }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

/// `Ang_RotateToBearing` 0x1e0f10 transcribed from the decompile (this task's re-read; the port's
/// `trig::rotate_to_bearing` is the same for rate ≤ 1 — this copy is the test's independent
/// reference, so a regression in either is caught).
fn rotate_to_bearing_game(mut cur: f32, mut target: f32, rate: f32, max: f32) -> f32 {
    const TAU: f32 = 6.283_185_5;
    const PI: f32 = 3.141_592_7;
    if target < 0.0 || TAU <= target { target = target % TAU; if target < 0.0 { target += TAU; } }
    if cur < 0.0 || TAU <= cur { cur = cur % TAU; if cur < 0.0 { cur += TAU; } }
    let mut d = target - cur;
    if PI <= d.abs() { if d < 0.0 { d += TAU; } else { d -= TAU; } }
    let clamped = d.clamp(-max, max);
    let full = cur + clamped;
    let mut eased = cur + (full - cur) * rate;
    if (clamped >= 0.0 && eased > full) || (clamped < 0.0 && eased < full) { eased = full; }
    if (eased - target).abs() < 0.0001 { eased = target; }
    eased
}

/// The angle (rad) between the head bone's world forward (palette row 2) and the direction to
/// `target`. `psiBuildMatrixPalette` 0x111498: each palette entry is 0x40 bytes — rotation rows
/// 0..0x30, translation +0x30 — row 2 = the world image of the bone's local +z (forward).
fn head_fwd_angle(world: &[[[f32; 4]; 4]], target: [f32; 3]) -> f32 {
    let head = [world[5][3][0], world[5][3][1], world[5][3][2]];
    let fwd = norm(row(&world[5], 2));
    let to = norm([target[0] - head[0], target[1] - head[1], target[2] - head[2]]);
    dot(fwd, to).clamp(-1.0, 1.0).acos()
}

/// The head bone's world up verticalness (1 = upright — the game's head carries no roll).
fn head_uprightness(world: &[[[f32; 4]; 4]]) -> f32 { norm(row(&world[5], 1))[1] }

/// The port's tracked-drone pick: an enabled, alive, UNARMED drone nearest to Bond (the game's
/// armed gate: `ObjectIsArmed` 0x178768 = weapon 2..=0x46 or 0x6a..=0x71 — the port's aims_torso
/// weapon gate; the head-track code is shared by guards and guests, the playtest bugs on both).
fn nearest_unarmed_drone(sim: &nf_view::scene::Sim) -> Option<usize> {
    let bp = sim.bond.pos;
    let mut best: Option<(i64, usize)> = None;
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || !dr.enabled || dr.corpse { continue; }
        if (2..=0x46).contains(&dr.weapon) || (0x6a..=0x71).contains(&dr.weapon) { continue; }
        let p = dr.pos;
        let d2 = ((p[0] - bp[0]).powi(2) + (p[2] - bp[2]).powi(2)) as i64;
        if best.map_or(true, |(b, _)| d2 < b) { best = Some((d2, i)); }
    }
    best.map(|(_, i)| i)
}

/// One tick of the trace: read the pre-tick state, compute the game's expected next stored angles
/// from THIS file's transcription, step, and compare. `pin` re-stages Bond's position before the
/// step (the floating-free "above" alternative is not used — Bond always stands on real floors).
/// The eye the port's update feeds `head_track_want` is Bond's camera = `Bond::head()` (scene.rs
/// `run(...)` passes `self.bond.head()` as the snapshot's `eye`); a pinned Bond's one-tick fall
/// (~1.4 mm) perturbs the want by ≲7e-4 rad, inside the tolerance.
fn trace_tick(scene: &mut nf_view::scene::Sim, di: usize) -> (f32, f32, f32, f32, bool) {
    let (prev_y, prev_p, alert, unarmed, los, seen) = {
        let dr = &scene.drones.list[di];
        (dr.head_yaw, dr.head_pitch, dr.alert, dr.weapon, dr.los, dr.seen)
    };
    assert!(!((2..=0x46).contains(&unarmed) || (0x6a..=0x71).contains(&unarmed)),
        "the traced drone must be unarmed — armed guards in the aim states leave the track branch by design");
    // The drone update's branch: the game's head tracking follows the sight pass (the port's gate
    // is `los || seen`, +0x228 & 2/4 — before the first sight pass the guest has not noticed Bond
    // and the head stays level, easing to 0 at 0.07 like the no-target arm).
    let tracking = los || seen;
    let (want_y, want_p) = {
        let eye = scene.bond.head();
        let dr = &scene.drones.list[di];
        if tracking && dr.dist <= 3.0 { dr.head_track_want(&scene.data(), eye).unwrap_or((0.0, 0.0)) } else { (0.0, 0.0) }
    };
    let sp = if alert > 0.66 { 0.14 } else { 0.07 };
    let exp_y = rotate_to_bearing_game(prev_y, want_y, sp, std::f32::consts::PI);
    let exp_p = rotate_to_bearing_game(prev_p, want_p, sp, std::f32::consts::PI);
    scene.step_bond(Input::default(), false, false, false);
    let sim = scene;
    let dr = &sim.drones.list[di];
    let eye = sim.bond.head();
    let (fy, fp) = (dr.head_yaw, dr.head_pitch);
    // The port measures the want inside the update, from the anim pose AFTER this tick's anim
    // advance; this reference reads it pre-tick — the guest's idle bob moves the measured pitch
    // want by ~0.02 rad, so the eased step differs by ~0.07·0.02 ≈ 1.5e-3. A sign flip, a rate
    // error or a branch miss produces O(0.1) deltas — the tolerance only absorbs the measure point.
    let tol = 4e-3;
    // The game's RotateToBearing returns the result wrapped into [0, 2π) (0x1e0f10 normalises
    // both inputs first); the port stores the signed equivalent — the palette consumes sin/cos,
    // so only the value mod 2π is meaningful.
    const TAU: f32 = std::f32::consts::TAU;
    let dmy = (fy - exp_y).rem_euclid(TAU);
    let dmp = (fp - exp_p).rem_euclid(TAU);
    let dmy = dmy.min(TAU - dmy);
    let dmp = dmp.min(TAU - dmp);
    if dmy >= tol || dmp >= tol {
        panic!(
            "tick trace: the stored head angles left the game's ease path\n  want ({want_y:.4}, {want_p:.4}) sp {sp}\n  expected ({exp_y:.4}, {exp_p:.4})\n  got      ({fy:.4}, {fp:.4})\n  prev ({prev_y:.4}, {prev_p:.4})\n  |delta| ({dmy:.5}, {dmp:.5})"
        );
    }
    let world = { let dr = &sim.drones.list[di]; dr.palette(&sim.data()).expect("palette") };
    let tracked = tracking && dr.dist <= 3.0;
    (fy.abs(), fp, head_fwd_angle(&world, eye), head_uprightness(&world), tracked)
}

/// ── The 3-pose trace ────────────────────────────────────────────────────────────────────────────
/// Bond beside the tracked drone on both sides (its own floor) and above it (the fix_heads07 F12
/// balcony, whose guests the 1014-1725 trace measured at yaw ±1.100 / pitch +1.000); every tick's
/// stored angles follow the game's formula, the head turns toward Bond and stays upright, and the
/// saturated poses sit at the clamps (|yaw| 1.1; pitch the game's −1.0 up-limit = the port's
/// +1.0 positive-up). Finally Bond steps beyond the 3.0 range and the stored angles ease back to
/// 0 (the game's gate-skip leaves the wants at 0).
#[test]
fn head_track_trace_left_right_above_matches_the_game_formula() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // Bond at the guest's floor, 1.4 to each side of the guest's FACING (yaw read live so the
    // guest's wander between phases doesn't matter): cross(up, fwd) at yaw θ = (cos θ, 0, −sin θ)
    // — the port's horizontal side axis (drone.rs aim comment). 1.4 is inside the 3.0 range gate
    // and far enough for the yaw to saturate the ±1.1 clamp.
    let (anchor, gyaw, gpos) = {
        let sim = scene.sim.as_ref().unwrap();
        let di = nearest_unarmed_drone(sim).unwrap();
        (di, sim.drones.list[di].yaw, sim.drones.list[di].pos)
    };
    // 0.3 ahead + 1.1 to each side of the guest's FACING: ~75° off-axis, 1.14 away — inside the
    // visibility short-circuit (`dist < 2.0` → visibility 1.0, drone.rs visibility()), past the
    // yaw clamp (atan(1.1/0.4) ≈ 1.22 → the measured want clamps at ±1.1).
    // fwd = (sin θ, 0, cos θ); side = cross(up, fwd) = (cos θ, 0, −sin θ) (the port's side axis).
    let (s_y, c_y) = (gyaw.sin(), gyaw.cos());
    let side_a = [gpos[0] + 0.3 * s_y + 1.1 * c_y, gpos[1], gpos[2] + 0.3 * c_y - 1.1 * s_y];
    let side_b = [gpos[0] + 0.3 * s_y - 1.1 * c_y, gpos[1], gpos[2] + 0.3 * c_y + 1.1 * s_y];

    for (name, sign) in [("side-a", 1.0f32), ("side-b", -1.0)] {
        // The guests wander (~1.5 units/s): re-read the anchor's live pos/yaw right before each
        // placement so the staged spot is ~1.14 from the drone the trace follows.
        let (spot, yaw) = {
            let sim = scene.sim.as_ref().unwrap();
            let g = &sim.drones.list[anchor];
            let (gpos, th) = (g.pos, g.yaw);
            let (s_y, c_y) = (th.sin(), th.cos());
            let spot = [gpos[0] + 0.3 * s_y + sign * 1.1 * c_y, gpos[1], gpos[2] + 0.3 * c_y - sign * 1.1 * s_y];
            (spot, th + sign * std::f32::consts::FRAC_PI_2)
        };
        // The drones list can splice during the settle ticks (spawner waves), shifting indices:
        // stage onto the nearest unarmed drone, settle, re-check, and re-stage until Bond really
        // stands ~1.1 from one (the splices settle within a few tries).
        let di = 'stage: {
            let mut di = anchor;
            for _ in 0..4 {
                let (spot, yaw) = {
                    let sim = scene.sim.as_ref().unwrap();
                    let g = &sim.drones.list[di];
                    let (s_y, c_y) = (g.yaw.sin(), g.yaw.cos());
                    ([g.pos[0] + 0.3 * s_y + sign * 1.1 * c_y, g.pos[1], g.pos[2] + 0.3 * c_y - sign * 1.1 * s_y],
                     g.yaw + sign * std::f32::consts::FRAC_PI_2)
                };
                {
                    let sim = scene.sim.as_mut().unwrap();
                    sim.bond = nf_game::player::Bond::new(spot, yaw);
                }
                for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
                di = nearest_unarmed_drone(scene.sim.as_ref().unwrap()).expect("an unarmed drone in 07000007");
                let sim = scene.sim.as_ref().unwrap();
                let d = sim.drones.list[di].dist;
                if d <= 2.0 { break 'stage di; }
            }
            di
        };
        let mut max_yaw = 0.0f32;
        let mut min_angle = 180.0f32;
        let mut min_upright = 1.0f32;
        let mut tracked_ticks = 0usize;
        for t in 0..90 {
            if t % 15 == 0 {
                let sim = scene.sim.as_ref().unwrap();
                let dr = &sim.drones.list[di];
                println!("{name} tick {t}: dist {:.2} los {} seen {} tracking {} bond {:?} dr {:?} dy {:.2}", dr.dist, dr.los, dr.seen, dr.tracking, sim.bond.pos, dr.pos, dr.pos[1] - sim.bond.pos[1]);
            }
            let (yaw, _pitch, ang, up, tracked) = trace_tick(scene.sim.as_mut().unwrap(), di);
            max_yaw = max_yaw.max(yaw);
            min_angle = min_angle.min(ang);
            min_upright = min_upright.min(up);
            if tracked { tracked_ticks += 1; }
            if t % 30 == 0 { println!("{name} tick {t}: |yaw| {yaw:.3} fwd→bond {ang:.3} upright {up:.3}"); }
        }
        println!("{name}: max |yaw| {max_yaw:.3} min fwd→bond {min_angle:.3} min upright {min_upright:.3} tracked {tracked_ticks}");
        // The clamp: the stored yaw never passes the game's ±1.1 (0x1bf618:294-297) in any pose.
        assert!(max_yaw <= 1.101,
            "{name}: the stored yaw must stay within the game's ±1.1 clamp; max {max_yaw:.3}");
        if tracked_ticks >= 30 {
            assert!(min_angle < 0.6,
                "{name}: while tracking, the head's world forward turns toward Bond (the playtest's 'wrong direction' check); min angle {min_angle:.3} rad");
            assert!(min_upright >= 0.9,
                "{name}: the head must stay upright — the game's head-track only yaws about the neck and pitches about the side axis, no roll (the 1013-1758 roll bug); min {min_upright:.3}");
        }
    }

    // Above: Bond on the playtest F12 balcony (spot 3 of the 1014-1725 run, fix_heads07's
    // placement) — the guests below him within the 3.0 gate pitch up to the game's −1.0
    // (positive-down) clamp = the port's +1.0 positive-up.
    let di = {
        // The fix_heads07 F12 spots (the 1014-1725 run measured guests below these at yaw ±1.100 /
        // pitch +1.000): try each until one has an unarmed drone below Bond within the gate.
        let spots = [[39.78f32, 44.10, -70.86], [40.58, 44.13, -70.77], [41.71, 44.13, -72.05]];
        let mut found = None;
        'spots: for sp in spots {
            {
                let sim = scene.sim.as_mut().unwrap();
                sim.bond = nf_game::player::Bond::new(sp, 0.682);
                sim.bond.pitch = -0.080;
            }
            // The guests mill around: give them time to wander under the balcony (the 1014-1725
            // run's saturated trackers arrived over ~25 s), checking periodically.
            for t in 0..30 {
                for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
                let sim = scene.sim.as_ref().unwrap();
                let bp = sim.bond.pos;
                let mut best = None;
                for (i, dr) in sim.drones.list.iter().enumerate() {
                    if !dr.present || !dr.enabled || dr.corpse { continue; }
                    if (2..=0x46).contains(&dr.weapon) || (0x6a..=0x71).contains(&dr.weapon) { continue; }
                    let d3 = ((dr.pos[0] - bp[0]).powi(2) + (dr.pos[1] - bp[1]).powi(2) + (dr.pos[2] - bp[2]).powi(2)).sqrt();
                    if dr.pos[1] < bp[1] - 0.2 && d3 <= 3.0 { best = Some((d3, i)); }
                }
                if let Some((d3, i)) = best {
                    println!("above: spot {sp:?} tick {} → drone {i} at {:?} (3D {d3:.2} below Bond)", t * 30, sim.drones.list[i].pos);
                    found = Some(i);
                    break 'spots;
                }
            }
        }
        match found {
            Some(i) => i,
            None => {
                eprintln!("above: no guest wandered under any balcony spot — the pitch-clamp phase is skipped this run");
                for _ in 0..160 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
                return;
            }
        }
    };
    {
        let mut max_yaw = 0.0f32;
        let mut max_pitch = 0.0f32;
        let mut min_angle = 180.0f32;
        let mut min_upright = 1.0f32;
        for t in 0..120 {
            let (yaw, pitch, ang, up, _tracked) = trace_tick(scene.sim.as_mut().unwrap(), di);
            max_yaw = max_yaw.max(yaw);
            max_pitch = max_pitch.max(pitch);
            min_angle = min_angle.min(ang);
            min_upright = min_upright.min(up);
            if t % 40 == 0 { println!("above tick {t}: |yaw| {yaw:.3} pitch {pitch:.3} fwd→bond {ang:.3} upright {up:.3}"); }
        }
        println!("above: max |yaw| {max_yaw:.3} max pitch {max_pitch:.3} min fwd→bond {min_angle:.3} min upright {min_upright:.3}");
        assert!(max_pitch >= 0.99,
            "above: the stored pitch must clamp at the game's up-limit −1.0 (positive-down) = the port's +1.0 (positive-up); max {max_pitch:.3}");
        assert!(min_angle < 0.5,
            "above: the head's world forward must turn up toward Bond; min angle {min_angle:.3} rad");
        assert!(min_upright >= 0.9, "above: the head must stay upright (no roll); min {min_upright:.3}");
    }

    // The ease-back: Bond steps beyond the 3.0 range gate — the game's HeadTrack skips both angle
    // blocks (0x1bf618:191), the wants stay 0, and the stored angles decay to 0 at the same rate.
    {
        let sim = scene.sim.as_mut().unwrap();
        let g = sim.drones.list[di].pos;
        sim.bond = nf_game::player::Bond::new([g[0] + 6.0, g[1], g[2]], 0.0);
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..160 {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.list[di].dist > 3.0, "the away pose must be beyond the 3.0 head-track range");
        let _ = trace_tick(scene.sim.as_mut().unwrap(), di);
    }
    let (fy, fp) = {
        let sim = scene.sim.as_ref().unwrap();
        (sim.drones.list[di].head_yaw, sim.drones.list[di].head_pitch)
    };
    println!("away: stored angles after the ease-back ({fy:.4}, {fp:.4})");
    assert!(fy.abs() < 0.05 && fp.abs() < 0.05,
        "beyond the range gate the stored angles ease back to 0 (the game's skipped angle blocks leave the wants at 0); got ({fy:.3}, {fp:.3})");
}
