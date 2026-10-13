//! fix-wire2 — the 07000005 cable crossing through the REAL input path, and the 07000014 fusebox
//! dev scenario. The user's playtest of 2026-10-12: "pressing e didnt do anything so I lept at it
//! over the ledge and it kind of grabbed it. I then couldnt really move forward so I kept jumping
//! 2 more times to the other side."
//!
//! The game's law (the decompile):
//! - E does NOT grab wires — `Player_Activate` 0x1a5170's dispatch has no class-0x2C arm [M].
//! - The jump grab: `Player_HandleJump` 0x1916b0:118-147 — with the third-icon context 3
//!   (`BLData+0x95F`, written by `ThirdIcon_Update` 0x196e80 from the kind-0xFA icons [D probe:
//!   three type-3 icons under the cable at (−23.58, 32.24, 2.65) / (−23.56, 32.24, 0.21) /
//!   (−23.80, 32.74, 12.33)]) the press grabs the NEAREST wire within 1000 units.
//! - The hand-over-hand move axis is action 1 = the strafe axis [M `Input_Actionf(…,1,1)` 0x1977c0;
//!   `psiInput_MapInputs` 0x10cd80: field +0x18 = the left stick X]. W (forward, action 2) does
//!   nothing on the wire in the game too.
//! - An idle (mode 6) with move input skips the `AnimScriptIsStopped` gate [M 0x197838-0x197860] —
//!   the move starts on the press, not after the idle cycle.

use nf_game::player::{Input, SS_WALK, SS_WIRE};
use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The user's sequence at the ledge: standing at the wire icons (south ledge), E does nothing
/// (game-faithful), the JUMP grabs the nearest cable (the icon-context path), and Bond hangs.
#[test]
fn the_jump_at_the_cable_grabs_it_and_e_does_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let w = sim.wires.first().expect("07000005 has one wire (0x3E)").clone();
    // Bond at the icon1 spot (−23.58, 32.24, 2.65 [D probe]), on the south ledge floor (~31.0),
    // OUTSIDE the wire's world sphere (the walking collision grab must not pre-fire — pre-checked).
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [-23.58, 34.0, 2.65], 0.0);
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.substate, SS_WALK, "walking the icons does not auto-grab");
    let (ca, cb) = sim.bond.capsule_seg();
    assert!(!w.sphere_hits(ca, cb), "the standing capsule is outside the world sphere (the pre-fix walking scan baseline)");
    // The icons set the context: +0x95F == 3 [M ThirdIcon_Update 0x196e80; D the type-3 icons here].
    assert_eq!(sim.bond.icon_ctx, 3, "the wire icons put +0x95F at 3 at this spot (got {})", sim.bond.icon_ctx);
    // Sister (game-faithful): E does NOTHING — Player_Activate 0x1a5170 has no class-0x2C arm.
    sim.step_bond(Input::default(), false, false, true);
    assert_eq!(sim.bond.substate, SS_WALK, "E never grabs a wire [M 0x1a5170:95-160]");
    // The jump grab [M 0x1916b0:118-147]: the press takes the NEAREST wire (no capsule test).
    sim.step_bond(Input { jump: true, ..Default::default() }, false, false, false);
    assert_eq!(sim.bond.substate, SS_WIRE, "the jump at the cable (icon context 3) must grab it");
    assert_eq!(sim.bond.cam_mode, 0xB, "the grab sets the wire camera mode [M CollWire 0x1975f8]");
    assert!((sim.bond.pos[1] - (w.centre[1] - 1.0207)).abs() < 0.09,
        "hang height: closest.y − 1.0207 [M CollWire 0x19744c], got {:?}", sim.bond.pos);
    assert!((sim.bond.pos[0] - w.centre[0]).abs() < 0.1, "snapped onto the wire's x line: {:?}", sim.bond.pos);
    // The grab pose plays out; Bond still hangs (the control leg).
    for _ in 0..140 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.substate, SS_WIRE, "still on the wire after the grab pose");
}

/// The hand-over-hand move: the strafe axis starts the move DURING the idle (the mode-6 gate), W
/// (forward) does nothing on the wire (the axis is action 1 [M]), and the crossing carries Bond to
/// the far end alive.
#[test]
fn the_hand_over_hand_move_crosses_on_the_strafe_axis() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let w = sim.wires.first().expect("wire").clone();
    let (a_t, _) = w.travel_segment();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [-23.58, 34.0, 2.65], 0.0);
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    sim.step_bond(Input { jump: true, ..Default::default() }, false, false, false);
    assert_eq!(sim.bond.substate, SS_WIRE, "the jump grab attaches");
    for _ in 0..140 { sim.step_bond(Input::default(), false, false, false); } // the grab pose out
    let z_idle = sim.bond.pos[2];
    // The idle anim (5ec) has just been (re)added. Its track length bounds the pre-fix wait: the
    // old gate held the move until the idle STOPPED; the decompile's mode-6 path has no gate [M].
    let idle_frames = sim.wire_anims.tracks.get(&0x0600_05ec).map(|t| t.frames).unwrap_or(0);
    assert!(idle_frames > 30, "the idle track must be long enough to bound the pre-fix wait (got {idle_frames})");
    // The strafe press DURING the idle: the move must start on the press (movement within 25 ticks).
    let cross = Input { strafe: -1.0, ..Default::default() };
    let mut moved_at = None;
    for t in 0..25u32 {
        sim.step_bond(cross, false, false, false);
        if (sim.bond.pos[2] - z_idle).abs() > 0.1 { moved_at = Some(t); break; }
    }
    assert!(moved_at.is_some(), "the strafe press during the idle must start the move within 25 ticks (idle track {idle_frames})");
    // Let the current move cycle finish (the anim's root motion runs to its end — game-faithful):
    // park when Bond reaches the idle (mode 6 = the game's idle byte [M +0x94D = 6]).
    for _ in 0..400u32 {
        sim.step_bond(cross, false, false, false);
        if sim.bond.mode == 6 { break; }
    }
    // Sister: W (forward, action 2) does nothing on the wire — the move axis is the strafe [M].
    // Idle re-adds sway ±0.0542 [M the per-add shift], so the band sits just above the sway.
    let z_w = sim.bond.pos[2];
    for _ in 0..120 { sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false); }
    assert!((sim.bond.pos[2] - z_w).abs() < 0.08, "forward input never moves Bond along the wire [M 0x1977c0 reads action 1]");
    // Keep crossing to the far end: alive the whole way, ground at the north end.
    let mut reached = false;
    for _ in 0..900u32 {
        sim.step_bond(cross, false, false, false);
        assert_eq!(sim.bond.substate, SS_WIRE, "no fall or death check while attached");
        if sim.bond.pos[2] >= a_t[2] - 0.8 { reached = true; break; }
    }
    assert!(reached, "Bond must travel the wire to the far end (A z {}, got {:?})", a_t[2], sim.bond.pos);
    assert!(sim.drones.health.health > 0.0, "alive after the crossing");
}

/// The 07000014 fusebox dev scenario: Bond starts CROUCHED inside the crawl vent (the user's ask),
/// with the Laser out for the sabotage.
#[test]
fn the_fusebox_scenario_starts_crouched_inside_the_vent() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load 07000014");
    assert!(dev::scenario("fusebox-07000014").is_some(), "the scenario is registered");
    let (_, f) = dev::scenario("fusebox-07000014").unwrap();
    f(&mut scene);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.level, 0x0700_0014);
        let p = sim.bond.pos;
        assert!((p[0] - 41.0).abs() < 0.2 && (p[2] - 62.0).abs() < 0.2, "Bond inside the vent's north run, got {p:?}");
        assert!(sim.bond.crouched, "the scenario starts Bond CROUCHED (the user's ask)");
        assert!((sim.bond.cc - nf_game::player::CROUCH_ROOT_HEIGHT).abs() < 1e-4, "the crouch root height");
        // Inside the crawl tunnel: the ceiling within ~1.5 of the floor [D probe_vent2: headroom
        // 1.46 at (41.0, 2.40, 62.0)].
        let ceil_t = sim.world.segment([p[0], p[1] + 0.1, p[2]], [p[0], p[1] + 2.5, p[2]]);
        let headroom = ceil_t.map(|t| 0.1 + 2.4 * t).unwrap_or(99.0);
        assert!(headroom < 1.7, "inside the vent (crawl headroom), got {headroom:.2}");
        assert_eq!(sim.gun.weapon, 0x4e, "the Laser (the cutting weapon [L weapon_data]) is selected");
    }
    // Sixty ticks in: still crouched, still in the tunnel (no auto-stand, no fall through).
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.bond.crouched, "still crouched after 60 ticks");
    assert!((sim.bond.pos[0] - 41.0).abs() < 0.5 && (sim.bond.pos[2] - 62.0).abs() < 1.0,
        "still inside the vent tunnel, got {:?}", sim.bond.pos);
}
