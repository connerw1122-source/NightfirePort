//! fix-gadgets2 bug 3: the grapple "doesn't work at all" from the user's play spot.
//!
//! The user's screenshot spot on 07000005: `--bond -18.28,38.19,-71.83,-4.717,0.151` — the gadget is
//! selected, labelled "Grapple", aimed at the point on the wall, and firing does nothing.
//!
//! What the decompile says [M]:
//! - The fire polarity: `Player_SSGrapple` 0x192860:56-66 — in the hang, `Input_Action(9,1)` HELD
//!   commits the velocity and STAYS; the RELEASE moves to phase 4 and detaches. Phase 1 (the reel)
//!   reads no fire input at all (:79-121). The pre-fix port did the opposite (the reel jumped to the
//!   hang on fire; the hang released on fire), so a human click — press, hold a few ticks, let go —
//!   latched and detached within ~3 ticks: invisible, "doesn't work at all".
//! - The reach (fix-qclaw correction): the claw bolt dies at the claw's OWN `weapon_data+0x1c`
//!   [M Bullet_update 0x12e7b0:158] — **50.0 for 0x50, 100.0 for 0x51 [L ee_start_05.bin]**, not the
//!   5.0/10.0 the old port read off the STUNNER records 0x4A/0x4C (the hardcoded table's shift).
//!   The user's spot is 15.4 units from the nearest point — inside the claw's real range, so the
//!   held fire latches there too.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The user's spot: (-18.28, 38.19, -71.83), yaw -4.717, pitch 0.151.
const SPOT: [f32; 3] = [-18.28, 38.19, -71.83];
const YAW: f32 = -4.717;
const PITCH: f32 = 0.151;

/// From the user's spot the nearest point is 15.4 units out [D the parsed placement]: INSIDE the
/// claw's real 50.0 bolt range [L weapon_data[0x50]+0x1c; the old 5.0 was the Stunner's], so the
/// held fire LATCHES from the user's own report spot.
#[test]
fn the_users_spot_latches_within_the_claws_real_range() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    assert!(!sim.drones.grapple_points.is_empty(), "07000005 carries grapple points [D #281/#282]");
    // Draw the claw like --give 0x50 (owned at the level start [L]; the re-equip may decline).
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x50).max(1);
    let _ = sim.drones.inventory.equip_weapon(&wd, 0x50, rounds, true);
    sim.gun.switch_request = 0x50;
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 0x50, "the claw must draw; weapon {:#x}", sim.gun.weapon);
    sim.bond.pos = SPOT;
    sim.bond.yaw = YAW;
    sim.bond.pitch = PITCH;
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.yaw = YAW;
    sim.bond.pitch = PITCH;
    // The held click latches: the spot is within the claw's real bolt range.
    let mut latched = false;
    for _ in 0..60 { sim.step_bond(Input::default(), true, false, false); if sim.bond.grapple.is_some() { latched = true; break; } }
    assert!(latched, "the held fire from the user's spot must latch (the nearest point is 15.4 out, the claw's range 50.0)");
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.substate, nf_game::player::SS_GRAPPLE, "the grapple substate runs");
}

/// Close-up (within the claw's real 50.0 bolt range): the held fire latches, the reel runs while
/// held (no fire test in phase 1 [M 0x192860:79-121]), Bond hangs and STAYS while the fire is held,
/// and the RELEASE detaches — the game's polarity, the user's fix.
#[test]
fn the_q_claw_holds_while_held_and_releases_on_the_button_up() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    // 4.05 units from point #282 (13.7525, 32.2919, 57.3034 [L ee_start_05.bin obj+0x30]).
    let spot = [16.5f32, 29.6, 58.6];
    sim.bond.pos = spot;
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x50).max(1);
    let _ = sim.drones.inventory.equip_weapon(&wd, 0x50, rounds, true);
    sim.gun.switch_request = 0x50;
    let mut equipped = false;
    for _ in 0..120 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.gun.weapon == 0x50 { equipped = true; break; }
    }
    assert!(equipped, "the claw (0x50) must draw; weapon {:#x}", sim.gun.weapon);
    sim.bond.pos = spot;
    let target = [13.752469f32, 32.291878, 57.303413];
    // Aim through the real look input.
    for _ in 0..200 {
        let (turn, look) = aim_inputs(&scene, target);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    // HOLD the fire through latch + reel + hang: it must never detach while held.
    let mut hung = false;
    for _ in 0..1200 {
        let (turn, look) = aim_inputs(&scene, target);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.bond.grapple.as_ref().map(|g| g.phase) == Some(2) {
            let d = dist(sim.bond.pos, target);
            if d < 2.5 { hung = true; break; }
        }
        assert!(sim.bond.substate == nf_game::player::SS_GRAPPLE || sim.bond.substate == nf_game::player::SS_WALK,
            "the grapple never leaves the substate while the fire is held");
        if sim.bond.substate == nf_game::player::SS_WALK && sim.bond.grapple.is_none() {
            panic!("the grapple detached while the fire was HELD (the pre-fix polarity)");
        }
    }
    assert!(hung, "the held fire must reel Bond to the point and hang him");
    // Release the button: the detach [M 0x192860:64-66].
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the release returns Bond to the walk substate");
    assert!(sim.bond.grapple.is_none(), "the grapple state clears on the release");
}

fn aim_inputs(scene: &nf_view::scene::Scene, target: [f32; 3]) -> (f32, f32) {
    let sim = scene.sim.as_ref().unwrap();
    let bp = sim.bond.pos;
    let dx = target[0] - bp[0];
    let dz = target[2] - bp[2];
    let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
    let desired = dx.atan2(dz);
    let err = desired - sim.bond.yaw;
    let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
    (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
}

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
fn dist(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
