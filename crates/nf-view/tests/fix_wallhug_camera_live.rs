//! fix-wallhug-prompt: the wall shimmy's THIRD-PERSON camera, through the REAL input path.
//!
//! The user (playtest 1014-1725): "near the wall edging area I kept seeing a prompt and when I
//! entered a prompt it was in first person. should be 3rd person."
//!
//! The game: entering the crawl (`Player_CollCreepWall` 0x196758:114-127) holsters the weapon
//! (`Player_WeaponNone`), sets cam mode 0xB, and every creep tick `Player_PositionCamera`
//! 0x18f5d8 case 0xb runs `camera_tracking` 0x1ba5e0 — the third-person tracking camera: Bond is
//! drawn (`View_SetDrawInAllViews`), the view model is not (`View_SetDrawInNoViews(BLData+0x7e8)`),
//! and the viewer sits `closest − Cartesian(4.0, wall.yaw + π, −2.7486268)` off the wall's
//! axis line with both ends' Y +1.0, looking at the tracked point. This file asserts the ported
//! `Sim::creep_camera()` (scene.rs) — on the pre-fix tree it is a COMPILE failure (the method does
//! not exist; documented in the report), while the runtime geometry pins below.

use nf_game::player::{Input, SS_CREEP};
use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    scene.sim.take().expect("sim")
}

fn wait_for_control(sim: &mut Sim) {
    let idle = Input::default();
    for _ in 0..12000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
    for _ in 0..60 { sim.step_bond(idle, false, false, false); }
    assert!(!sim.disabled, "the intro NIS never released Bond");
}

/// Entering the shimmy through the real use-key path (the ctx-6 icon arms at the vent mouth, E
/// crawls) puts the view in the game's third-person tracking camera: off Bond (not his head), up
/// near the game's spherical offset, looking DOWN at him — and it leaves with the crawl.
#[test]
fn the_shimmy_camera_is_third_person() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    // The vent mouth (the fix_swim_vent5 out-point of creepwalls[0]) — ctx 6 arms there.
    let w = sim.creepwalls[0].clone();
    let (a, b) = w.endpoints();
    let mouth = [b[0] - (a[0] - b[0]) / w.half_len.max(1e-3), w.pos[1], b[2] - (a[2] - b[2]) / w.half_len.max(1e-3)];
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, mouth, 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 6, "the mouth must read the crawl icon (got {})", sim.bond.icon_ctx);
    // On foot the window keeps Bond's eye — no tracking camera [M the cam mode is 0 on foot].
    assert!(sim.creep_camera().is_none(), "on foot there is no tracking camera");
    // E: the crawl starts [M Player_Activate 0x1a5170:66-97 → Player_CollCreepWall 0x196758].
    sim.step_bond(Input::default(), false, false, true);
    assert_eq!(sim.bond.substate, SS_CREEP, "E at the mouth must crawl (substate {})", sim.bond.substate);
    // A few creep ticks, then the camera: the game's tracking camera, NOT Bond's head.
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.substate, SS_CREEP, "the crawl holds");
    let (cam, yaw, pitch, _fov) = sim.creep_camera().expect("the creep runs the tracking camera");
    let head = sim.bond.head();
    let off = ((cam[0] - head[0]).powi(2) + (cam[1] - head[1]).powi(2) + (cam[2] - head[2]).powi(2)).sqrt();
    assert!(off > 1.0, "the creep camera must sit OFF Bond's eye [M 0x18f5d8 case 0xb: View_SetDrawInAllViews, the viewer at closest − Cartesian(4.0, …)] — offset {off:.2}");
    assert!(cam[1] > head[1] + 1.0,
        "the tracking camera rides ABOVE (closest +1.0 line, r 4.0, pitch 2.7486 → 4·|cos| ≈ 3.70 up [M 0x1ba5e0:106-125]) — cam y {:.2} vs head {:.2}", cam[1], head[1]);
    assert!(pitch < -0.5, "the camera looks DOWN at the crawled point [M 0x1ba5e0:135-155] — pitch {pitch:.2}");
    let _ = yaw;
}
