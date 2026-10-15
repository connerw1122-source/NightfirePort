//! The 07000007 Kimiko scene's hand-back (playtest 1015-0046: "after it finished, it faded to black, and I
//! couldn't continue playing"). The scene fades its cut-scene viewer to black near its end (`Camera_SetFade(.., 0)`,
//! the script viewer); when the script camera hands back, the gameplay view must not stay black.
use nf_game::player::Input;

#[test]
fn the_kimiko_scene_hands_back_a_visible_view() {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([94.2, 44.6, -45.9], 0.0); // TouchOnce #412's volume (fix-cutscene-letterbox)
    let (mut started, mut black_before_end, mut handed_back) = (false, false, None);
    for t in 0..6000u32 {
        sim.step_bond(Input::default(), false, false, false);
        let cam = sim.script_camera().is_some();
        if cam { started = true; if sim.drones.view_fade.is_some_and(|f| f.alpha > 0.9) { black_before_end = true; } }
        if started && !cam && !sim.disabled { handed_back = Some(t); break; }
    }
    let t = handed_back.expect("the scene ends and hands control back within 6000 ticks");
    // Sister: the scene really faded its own view to black before the hand-back (the case that broke).
    assert!(black_before_end, "the scene's near-end fade reached black on the cut-scene viewer");
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    let a = sim.drones.view_fade.map_or(0.0, |f| f.alpha);
    assert!(a < 0.1, "the gameplay view is black after the hand-back at tick {t} (fade alpha {a})");
}
