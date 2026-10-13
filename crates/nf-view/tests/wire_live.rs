//! fix-wire: the 07000005 / 07000002 cable crossings through the real input path.
//! Source of truth: docs/atlas/wire-grapple.md; the decompile (Player_CollWire 0x197130,
//! Player_Wire 0x197658, Player_Zipline 0x197f50) and the placements probe in this file.
//! Run with: `NF_ISO=… cargo test -p nf-view --test wire_live -- --nocapture`

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

/// The user's play, end to end [D the user report: "at the cable left of the castle gate, pressing E
/// plays the falling-to-death animation instead of the hand-over-hand crossing"]: Bond walks into
/// 07000005's cable's grab zone (the wire placement −23.4855, 34.4676, 6.2629 [D placements probe],
/// yaw −1.5708, sphere radius 4.039 [L ee_start_05.bin 016dc430]) and crosses it hand over hand to the
/// far end, alive. The grab here is the COLLISION path [M Player_DealWithObjHit 0x194458 — the
/// walking capsule against the wire's world sphere]; fix-wire2 corrected the record: E (the use key)
/// NEVER grabs wires — `Player_Activate` 0x1a5170 has no class-0x2C arm — the other real grab is the
/// jump grab (see tests/fix_wire2_live.rs).
///
/// Sister (the no-input control leg): without the strafe input Bond stays at his end of the wire —
/// the movement below comes from the input, not from the grab itself.
#[test]
fn walking_into_the_07000005_cable_grabs_it_and_crosses_it_hand_over_hand() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let w = sim.wires.first().expect("07000005 has one wire (0x3E)").clone();
    assert_eq!((w.wire_type, w.param0, w.channel), (0, 2, 0), "the live instance bytes [L ee_start_05 016dc510]");
    // The wire runs along Z: A = the +Z end, B = the −Z end (yaw −1.5708 → spherical(yaw+π/2) = (0,0,r)).
    let (a_t, b_t) = w.travel_segment();
    // Bond at the B end of the grab span, standing 1.1 under the cable — inside the world sphere.
    sim.bond.pos = [w.centre[0], w.centre[1] - 1.1, b_t[2] + 0.2];
    sim.bond.yaw = 0.0;
    let health0 = sim.drones.health.health;
    // The walking collision grab [M Player_DealWithObjHit 0x194458] — E is NOT involved (no arm [M
    // 0x1a5170]); the E press here is inert input proving the grab does not need it.
    sim.step_bond(Default::default(), false, false, true);
    assert_eq!(sim.bond.substate, nf_game::player::SS_WIRE, "the collision grab attaches Bond (substate 6)");
    assert_eq!(sim.bond.cam_mode, 0xB, "the grab sets the wire camera mode (SetCamMode 0xB)");
    assert!((sim.bond.pos[1] - (w.centre[1] - 1.0207)).abs() < 0.09,
        "hang height: closest.y − 1.0207 [M CollWire 0x19744c], got {:?}", sim.bond.pos);
    // The grab pose plays out (~101 frames), then Bond hangs idle. No input.
    for _ in 0..140 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_WIRE, "still on the wire after the grab pose");
    let z_idle = sim.bond.pos[2];
    assert!(z_idle < a_t[2] - 1.0, "the idle Bond stays at his end (z {z_idle} vs the A end {})", a_t[2]);
    assert!((sim.bond.pos[0] - w.centre[0]).abs() < 0.2 && (sim.bond.pos[1] - (w.centre[1] - 1.0207)).abs() < 0.15,
        "on the wire line: {:?}", sim.bond.pos);
    // The music event 0xD [M Player_Wire 0x1976b4 posts it every tick] and the creeping flag.
    assert!(sim.creeping, "on the wire the music event 0xD posts (the substate drives it)");
    // The control leg is over: cross with the strafe axis (action 1 [M 0x1977c0]); negative = toward A.
    // The stop band: the decompile zeroes the input within 0.5² of the travelled-to end [M 0x1977e8],
    // and the last move cycle's root motion lands anywhere in [0, 0.71] short of it.
    let cross = nf_game::player::Input { strafe: -1.0, ..Default::default() };
    let mut reached = false;
    for t in 0..900 {
        sim.step_bond(cross, false, false, false);
        assert_eq!(sim.bond.substate, nf_game::player::SS_WIRE, "no fall or death check while attached (tick {t}: {:?})", sim.bond.pos);
        // Mid-cycle the anim's root motion swings Bond (the b5 track's root lifts +0.15 and shifts
        // −0.27 at the reach apex [D root tracks]); the per-add clamp re-snaps him onto the line.
        assert!((sim.bond.pos[0] - w.centre[0]).abs() < 0.5 && sim.bond.pos[1] < w.centre[1] && sim.bond.pos[1] > w.centre[1] - 1.3,
            "the anim envelope holds (tick {t}: {:?})", sim.bond.pos);
        if sim.bond.pos[2] >= a_t[2] - 0.8 { reached = true; break; }
    }
    assert!(reached, "Bond must travel the wire to the far end (A at z {}, got {:?})", a_t[2], sim.bond.pos);
    let crossed = sim.bond.pos[2] - z_idle;
    assert!(crossed > 3.0, "the crossing moved Bond {:.2} along the wire", crossed);
    assert_eq!(sim.drones.health.health, health0, "no fall damage while attached");
    // At rest (the input gone) the move cycle finishes and the next idle add re-snaps him onto the
    // wire line (the b5 tail ≈ 30 frames + the idle's own 140-frame wait before its add). The idle
    // sway leaves him within ~±0.16 of the line mid-cycle (the 5ec/5ed root-x tracks [D]).
    for _ in 0..220 { sim.step_bond(Default::default(), false, false, false); }
    assert!((sim.bond.pos[0] - w.centre[0]).abs() < 0.25 && (sim.bond.pos[1] - (w.centre[1] - 1.0207)).abs() < 0.05,
        "the clamp holds him on the line at rest: {:?}", sim.bond.pos);
    // Drop at the far end with the action press [M Player_Wire 0x197758: Input_Action(7,4)].
    sim.step_bond(nf_game::player::Input { jump: true, ..Default::default() }, false, false, false);
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the action press dismounts");
    assert_eq!(sim.bond.regrab, 0x1e, "the 30-frame regrab lockout [M 0x1978b0]");
    assert!(!sim.creeping, "off the wire the music event stops");
    // Alive after the dismount, with reachable ground below (the cable's A end anchors into the cliff
    // rock — the drop leaves him against the wall face; the walk physics takes over from there).
    assert!(sim.drones.health.health > 0.0, "Bond survives the crossing");
    assert!(sim.world.floor_below(sim.bond.pos, 8.0).is_some(), "the far end has ground within 8 (got {:?})", sim.bond.pos);
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.health.health > 0.0, "alive after the dismount ({:?})", sim.bond.pos);
}

/// The sister level: 07000002 has a climbable wire too (placement at −61.3055, 8.0824, −30.8348,
/// yaw −1.1167, "Wire" bounds centre (−4.1239, 0, 0) radius 11.5188 [D placements probe]). The same
/// grab → cross → dismount chain must work there (the world sphere is offset from the translation, so
/// this also pins `Wire::create`'s centre rotation).
#[test]
fn the_07000002_wire_crosses_too() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let w = sim.wires.first().expect("07000002 has one wire (0x3E)").clone();
    assert_eq!(w.wire_type, 0, "hand-over-hand (param 1 = 0)");
    // The world sphere: translation + the bounds centre rotated by the yaw (offset (−4.1239, 0, 0)).
    let (s, c) = w.yaw.sin_cos();
    let want_c = [
        -61.3055 + (-4.1239) * c,
        8.0824 + 0.0,
        -30.8348 - (-4.1239) * s,
    ];
    for k in 0..3 { assert!((w.centre[k] - want_c[k]).abs() < 1e-3, "centre[{k}] {} vs {}", w.centre[k], want_c[k]); }
    let (a_t, b_t) = w.travel_segment();
    // Bond under the wire's midpoint, 1.1 below — inside the world sphere. (The wire's ends sit in
    // trigger volumes on this level — a touch trigger at the B end starts a cut-scene that disables
    // Bond — so the test grabs mid-wire, clear of them.)
    sim.bond.pos = [w.centre[0], w.centre[1] - 1.1, w.centre[2]];
    sim.bond.yaw = 0.0;
    sim.step_bond(Default::default(), false, false, true);
    assert_eq!(sim.bond.substate, nf_game::player::SS_WIRE, "the grab works on 07000002's wire");
    for _ in 0..140 { sim.step_bond(Default::default(), false, false, false); }
    let idle = (sim.bond.pos[0], sim.bond.pos[2]);
    // Cross toward A (negative strafe; the grab yaw ≈ the mode-8 target, so no turn needed).
    let cross = nf_game::player::Input { strafe: -1.0, ..Default::default() };
    let mut reached = false;
    for t in 0..2400 {
        sim.step_bond(cross, false, false, false);
        assert_eq!(sim.bond.substate, nf_game::player::SS_WIRE, "attached the whole way (tick {t}: {:?})", sim.bond.pos);
        if sim.bond.pos[2] >= a_t[2] - 0.8 && sim.bond.pos[0] >= a_t[0] - 0.8 { reached = true; break; }
    }
    assert!(reached, "crossed the wire to A ({:?} want ≥ ({:.2},{:.2}))", sim.bond.pos, a_t[0], a_t[2]);
    let moved = ((sim.bond.pos[0] - idle.0).powi(2) + (sim.bond.pos[2] - idle.1).powi(2)).sqrt();
    assert!(moved > 8.0, "traveled {:.2} along the wire (the half-span is {:.2})", moved, w.radius - 0.5);
    assert_eq!(sim.drones.health.health, 100.0, "alive the whole way");
    sim.step_bond(nf_game::player::Input { jump: true, ..Default::default() }, false, false, false);
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the dismount works on 07000002 too");
    assert!(sim.drones.health.health > 0.0);
}

/// `Player_Zipline` 0x197f50 (substate 0xF, wire type 2) on a synthetic sloped wire: Bond grabs the
/// HIGH end, slides down accelerating (the fall-speed accumulator +0.15/tick [M 0x198054]), arrives at
/// the lower end (within 1.0 [M 0x198038]) and dismounts alive (mode 9 → the walk substate, regrab 60
/// [M 0x198078-0x19808c]). Sister: an UPHOLD grab (near the lower end) is refused [M 0x197260: a
/// zipline refuses grabs within half the segment of the lower end].
#[test]
fn zipline_rides_down_and_dismounts_at_the_lower_end() {
    let world = nf_game::world::CollisionWorld::from_triangles(Vec::new());
    // A sloped wire: yaw −π/2 → the segment runs along Z; pitch 0.3 tilts it.
    let wire = nf_game::wire::Wire {
        centre: [0.0, 10.0, 0.0], radius: 8.0, yaw: -std::f32::consts::FRAC_PI_2, pitch: 0.3,
        wire_type: 2, param0: 0, param2: 0, channel: 0,
    };
    let (a, b) = wire.zipline_segment();
    let high = if a[1] > b[1] { a } else { b };
    let low = if a[1] > b[1] { b } else { a };
    let wires = vec![wire.clone()];
    let ctx = nf_game::player::LadderCtx { wires: &wires, wire_anims: None, ..Default::default() };
    // Bond just above the high end, inside the sphere, outside the lower-end refusal zone.
    let mut bond = nf_game::player::Bond::new([high[0], high[1] - 1.1, high[2]], 0.0);
    bond.grab_wire(0, &ctx);
    assert_eq!(bond.substate, nf_game::player::SS_ZIPLINE, "a zipline grab enters substate 0xF [M CollWire 0x1971a8]");
    let mut ticks = 0;
    while bond.substate == nf_game::player::SS_ZIPLINE {
        bond.tick_ctx(&world, &ctx, &Default::default());
        ticks += 1;
        assert!(ticks < 3000, "the ride must end (pos {:?})", bond.pos);
    }
    assert_eq!(bond.substate, nf_game::player::SS_WALK);
    assert_eq!(bond.regrab, 60, "the zipline dismount locks the regrab for a second [M 0x198080]");
    // He slid DOWNHILL: the y dropped from the high end's hang to the low end's.
    assert!(bond.pos[1] < low[1], "arrived at the lower end ({:?} vs low y {})", bond.pos, low[1]);
    // Sister: grabbing near the LOWER end is refused.
    let mut bond2 = nf_game::player::Bond::new([low[0], low[1] + 0.2, low[2]], 0.0);
    bond2.grab_wire(0, &ctx);
    assert_eq!(bond2.substate, nf_game::player::SS_WALK, "the lower-end refusal zone holds");
}

#[test]
fn probe_wire_placements() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for name in ["07000005.bin", "07000002.bin"] {
        let data = read_archive(&iso, name);
        let level = Level::parse(&data).expect("parse");
        let n = level.maps.iter().map(|m| m.placements.iter().filter(|p| p.flags & 0xff == 0x3e).count()).sum::<usize>();
        eprintln!("{name}: {n} wire (0x3E) placements");
        for (mi, m) in level.maps.iter().enumerate() {
            for p in m.placements.iter().filter(|p| p.flags & 0xff == 0x3e) {
                eprintln!("  map {mi} at ({:.4},{:.4},{:.4}) rot ({:.4},{:.4},{:.4})",
                    p.translation[0], p.translation[1], p.translation[2], p.rotation[0], p.rotation[1], p.rotation[2]);
            }
        }
        assert!(n > 0, "{name} carries a wire placement");
    }
}
