//! fix-swim-vent5: the third-icon scope. The 07000005 castle-gate door (the 0x1D LOAD_LEVEL
//! trigger at (30.0, 38.84, 38.2)) must not hand Bond to the second vent's creepwall: the game
//! reads `BLData+0x95F` = 0xFF (no icon) at the door and at the level start [L PCSX2 2026-10-13,
//! held 40 frames each], so `Player_Activate`'s creepwall arm — gated on the context ALONE
//! [M 0x1a5170:66 `if (BL+0x95f == 6)`] — never fires there, even though the vent wall sits
//! ~24 units away (squared 579 < the scan's 1000). The context comes from the icons' own straddle
//! volume [M Controls_StraddleTest 0x134778:30-38 + Control_BuildWorldSph 0x133070; the write
//! ThirdIcon_Update 0x196e80:36-52], ported as the icon's placement sphere with the live-measured
//! radius (see scene.rs). Through the REAL viewer path (`nf_view::load` + `Sim::step_bond`).

use nf_game::player::{Input, SS_WALK};
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

/// [L PCSX2 2026-10-13] the game's +0x95F is 0xFF at the gate door and the level start, and the
/// icons' own types at the vent crawl spot (6) and the south-ledge wire spot (3).
#[test]
fn the_icon_context_is_none_at_the_gate_door_and_the_level_start() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The gate door: the second vent's ctx-6 icons are ~24.1 away (icon[3] (24.20, 31.93, 60.01)).
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [30.0, 38.84, 37.70], 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 0xff,
        "the gate door must read no icon [L PCSX2 2026-10-13] (got {})", sim.bond.icon_ctx);
    // Sister: the level start (104.95, 25.12, 108.83) reads none too (~94 from any ctx-6 icon).
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [104.95, 25.12, 108.83], 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 0xff,
        "the level start must read no icon [L PCSX2 2026-10-13] (got {})", sim.bond.icon_ctx);
    // Sister (positive): the vent crawl spot still reads the crawl icon 6.
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    let w = sim.creepwalls[0].clone();
    let (a, b) = w.endpoints();
    let out = [b[0] - (a[0] - b[0]) / w.half_len.max(1e-3), w.pos[1], b[2] - (a[2] - b[2]) / w.half_len.max(1e-3)];
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, out, 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 6, "the vent entry must read the crawl icon (got {})", sim.bond.icon_ctx);
}

/// The user's symptom: E at the gate door crawled the second vent's wall (substate 7 on a creepwall
/// ~31 units away, squared 971 < 1000) instead of opening the door. The context gate alone decides
/// [M Player_Activate 0x1a5170:66]; the scan's proximity never creeps.
#[test]
fn the_use_key_at_the_gate_door_does_not_creep() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    wait_for_control(&mut sim);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [30.0, 38.84, 37.70], 0.0);
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
    // The scan range is genuinely met here: the second vent's wall is within the squared-1000
    // scan (the pre-fix proximity fallback crept on it) — only the missing context keeps E off it.
    let d2 = sim.creepwalls.iter()
        .map(|w| (w.pos[0] - sim.bond.pos[0]).powi(2) + (w.pos[1] - sim.bond.pos[1]).powi(2) + (w.pos[2] - sim.bond.pos[2]).powi(2))
        .fold(f32::INFINITY, f32::min);
    assert!(d2 < 1000.0, "the test spot must sit inside the creep scan (nearest wall squared {d2})");
    sim.step_bond(Input::default(), false, false, true);
    assert_eq!(sim.bond.substate, SS_WALK,
        "E at the gate door must not crawl [M 0x1a5170:66; L the 0xFF pin] (substate {})", sim.bond.substate);
    assert_eq!(sim.bond.icon_ctx, 0xff, "the door context stays none across the use tick");
}
