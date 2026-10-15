//! fix-turn-stutter: the render camera bracket. The world jitters while turning (the view model —
//! pinned to the draw camera by fix-interp-ghosting's reattach — stays smooth, which unmasked
//! this) because the render bracket `cam_prev`/`cam_cur` was updated once per FRAME from the
//! frame's last tick: a frame that runs 2+ ticks (any droop below ~55 fps — the reported "frame
//! lag") left the bracket spanning every tick the frame ran, while the blend alpha
//! (`anim_time.fract()`) only spans one tick. The drawn camera then crawled through the wide
//! bracket and jumped at the boundary — hold-jump stutter while turning, on every level, exactly
//! the playtest report. The fix (`update_cam_bracket`, extracted from the loop the way
//! `ticks_for_frame` was for the fps-determinism test) keeps the bracket the LAST TWO ticks'
//! cameras — one tick apart — so `lerp_cam`/`lerp_yaw` at `anim_time.fract()` draws exactly one
//! tick behind the sim clock at every frame rate. These tests drive the loop's own arithmetic
//! (`ticks_for_frame` + `update_cam_bracket` + `lerp_yaw`) over a real level's sim with a steady
//! turn. Opt-in on the real level: NF_ISO.

use nf_view::{lerp_yaw, ticks_for_frame, update_cam_bracket, Camera};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The loop's per-tick camera capture (the `RedrawRequested` body's capture, verbatim inputs):
/// Bond's eye and view. `GAME_FOV` is the loop's private π/3.
fn tick_cam(sim: &nf_view::scene::Sim) -> Camera {
    let h = sim.bond.head();
    Camera {
        pos: glam::Vec3::new(h[0], h[1], h[2]),
        yaw: sim.bond.yaw,
        pitch: sim.bond.pitch * std::f32::consts::FRAC_PI_2,
        fov: std::f32::consts::FRAC_PI_3 / sim.zoom,
    }
}

/// One headless frame loop at `fps`, turning at 0.02 rad/tick (a normal mouse pace). Per frame:
/// `ticks_for_frame` schedules the ticks, each tick steps the sim and records the per-tick
/// camera, then `update_cam_bracket` — the real loop code — updates the render bracket, and the
/// draw's `lerp_yaw` renders it. Returns (rendered yaw per frame, the correct one-tick-behind yaw
/// from the full tick history, the yaw the OLD once-per-frame rule would render — the bracket
/// from the previous frame's camera to this frame's last).
fn run_turning(scene: &mut nf_view::scene::Scene, fps: u32, seconds: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let dt = 1.0 / fps as f32;
    let frames = (seconds * fps as f32).round() as usize;
    let mut anim_time = 0.0f32;
    let (mut cam_prev, mut cam_cur) = (None, None);
    let (mut old_prev, mut old_cur) = (None, None);
    let mut hist: Vec<f32> = Vec::new();
    let (mut rendered, mut correct, mut old_rule) = (Vec::new(), Vec::new(), Vec::new());
    let input = nf_game::player::Input { turn: 0.02, ..Default::default() };
    for _ in 0..frames {
        let n = ticks_for_frame(&mut anim_time, dt, 4);
        let (mut last, mut second_last, mut ran) = (None, None, 0u32);
        for _ in 0..n {
            scene.sim.as_mut().unwrap().step_bond(input, false, false, false);
            second_last = last;
            last = Some(tick_cam(scene.sim.as_ref().unwrap()));
            hist.push(scene.sim.as_ref().unwrap().bond.yaw);
            ran += 1;
        }
        update_cam_bracket(&mut cam_prev, &mut cam_cur, ran, second_last, last);
        if ran > 0 { old_prev = old_cur; old_cur = last; }
        let alpha = anim_time.fract();
        if let (Some(a), Some(b)) = (cam_prev, cam_cur) {
            rendered.push(lerp_yaw(a.yaw, b.yaw, alpha));
            let k = hist.len() - 1;
            correct.push(lerp_yaw(hist[k - 1], hist[k], alpha));
        }
        if let (Some(a), Some(b)) = (old_prev, old_cur) {
            old_rule.push(lerp_yaw(a.yaw, b.yaw, alpha));
        }
    }
    (rendered, correct, old_rule)
}

fn max_err(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max)
}

/// The named contract: at 30 fps — two ticks per frame, the loaded-frame case — the drawn camera
/// tracks the one-tick-behind sim camera exactly. The OLD once-per-frame bracket spans both of
/// the frame's ticks and deviates by about a tick's turn every frame; that contrast is the
/// fail-first evidence (this assertion failed against the pre-fix extraction).
#[test]
fn loaded_frames_render_one_tick_behind_exactly() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (rendered, correct, old_rule) = run_turning(&mut scene, 30, 6.0);
    // The fix's first-frame bonus: a 2-tick frame closes the bracket from its own per-tick
    // captures, so even frame 1 interpolates (at 60 fps the first 1-tick frame still has only
    // one camera — 239 there).
    assert_eq!(rendered.len(), 180);
    let err = max_err(&rendered, &correct);
    assert!(err < 1e-4, "the drawn camera must sit one tick behind the sim clock at 30 fps; max error {err:.5} rad");
    let old_err = max_err(&old_rule, &correct);
    assert!(old_err > 0.005, "contrast: the old once-per-frame bracket must deviate on loaded frames (got {old_err:.5} rad — the test would not have caught the bug)");
}

/// Sister (the full-rate invariant): at 60 fps every frame runs exactly one tick, the bracket is
/// the previous frame's camera and the new one — byte-identical to the old rule. The fix must not
/// move anything at a solid 60.
#[test]
fn full_rate_frames_keep_the_exact_bracket() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (rendered, correct, old_rule) = run_turning(&mut scene, 60, 4.0);
    assert_eq!(rendered.len(), 239, "the first frame has one camera and cannot interpolate");
    let err = max_err(&rendered, &correct);
    assert!(err < 1e-4, "60 fps must render one tick behind exactly; max error {err:.5} rad");
    let old_err = max_err(&rendered, &old_rule);
    assert_eq!(old_err, 0.0, "at one tick per frame the new bracket IS the old bracket");
}

/// Sister (no ISO): the bracket arithmetic itself — 2+ ticks close the bracket from the per-tick
/// captures; one tick keeps the previous frame's camera; nothing ran leaves the pair alone.
#[test]
fn bracket_arithmetic() {
    let cam = |yaw| Some(Camera { pos: glam::Vec3::ZERO, yaw, pitch: 0.0, fov: 1.0 });
    let (mut prev, mut cur) = (cam(10.0), cam(11.0));
    // A 2-tick frame whose per-tick captures are yaws 12 and 13: the bracket is the LAST TWO.
    update_cam_bracket(&mut prev, &mut cur, 2, cam(12.0), cam(13.0));
    assert_eq!(prev.unwrap().yaw, 12.0);
    assert_eq!(cur.unwrap().yaw, 13.0);
    // A 1-tick frame: the previous frame's camera and the new one (the old one-tick-behind pair).
    update_cam_bracket(&mut prev, &mut cur, 1, cam(99.0), cam(14.0));
    assert_eq!(prev.unwrap().yaw, 13.0);
    assert_eq!(cur.unwrap().yaw, 14.0);
    // Nothing ran (paused, the dev menu, a replay exhausted): the pair stays.
    update_cam_bracket(&mut prev, &mut cur, 0, cam(98.0), cam(97.0));
    assert_eq!(prev.unwrap().yaw, 13.0);
    assert_eq!(cur.unwrap().yaw, 14.0);
    // The first ticks of a session: None propagates until two cameras exist.
    let (mut prev, mut cur) = (None, None);
    update_cam_bracket(&mut prev, &mut cur, 1, None, cam(1.0));
    assert!(prev.is_none() && cur.unwrap().yaw == 1.0);
}
