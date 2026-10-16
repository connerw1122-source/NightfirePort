//! nis-event9-cnis: NIS event 9 (cam2mode) through the real input paths.
//!
//! Evidence base:
//! - The game routes event 9 via `Player_Cam2Mode(0, param0)` [M `Script_EventHandler` 0x1d94c0
//!   case 9]. Mode 3 → `Player_SetCamMode(BL, 2)` [M `Player_Cam2Mode` 0x195fc8:7-9] —
//!   `Player_PositionCamera` 0x18f5d8 case 2 → `camera_track_XZ_pos_behind(1.0, 10.0, obj+0x30,
//!   obj+0x50, viewer)` + `camera_track_Y_pos(1.0, 0.0, obj, viewer)` + `Camera_PosYRot`/
//!   `PosXRot(1.0, obj+0x30, viewer)` + `viewer+0x108 = 1.0`, `viewer+4 = 0` [M 0x18f5d8:145-156,
//!   0x1bc260/0x1bc1f0/0x1bc0c0/0x1bc148]. Anything else → `BL+0x8a8 = 0` + `SetCamMode(0)` +
//!   `Player_ChangeSubState(player, 0)` [M 0x195fc8:11-16].
//! - The disc's single event-9 firing [D, the nis_event9_census pin]: 07000001's script
//!   0x0600005b, stream 4, param 1 — the RESET arm. No disc script ever fires the orbit-enter
//!   mode 3, so the orbit's parameters are pinned from the decompile addresses above ([M]-only;
//!   marked in the asserts).
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The disc's one event-9 firing hands the camera back through the real script path: the test
/// puts the camera in the orbit state (mode 2 — what an earlier cut-scene's enter arm leaves [M
/// 0x195fc8:8]), starts 0600005b the game's way (`spawn_once`), and its stream-4 event 9 with
/// param 1 [D] must run `Player_Cam2Mode`'s exit arm — `SetCamMode(0)` + `ChangeSubState(0)` [M
/// 0x195fc8:11-16]. On the old tree kind 9 fell to the unhandled `Request::Event` arm and the
/// camera stayed stuck in mode 2.
#[test]
fn the_discs_event9_firing_hands_the_camera_back_through_the_real_script() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 01");
    let sim = scene.sim.as_mut().unwrap();

    // The script's shape straight from the archive [D]: the single event-9 command, stream 4,
    // param 1 (the firing the script-census pin counts as kind 9 × 1).
    let nis = sim.scripts.lib.get(&0x0600_005b).expect("0600005b ships in 01's archive")
        .streams.iter().enumerate()
        .flat_map(|(si, s)| s.decode().into_iter().map(move |c| (si, c)))
        .filter_map(|(si, c)| match c {
            nf_formats::nis::Cmd::Event { kind: 9, params } => Some((si, params.clone())),
            _ => None,
        }).collect::<Vec<_>>();
    assert_eq!(nis, vec![(4, vec![1])], "the disc's single event-9 firing [D]");

    // The orbit state, then the real script: the exit arm must clear it.
    sim.bond.cam_mode = 2;
    let ident = [[1.0f32, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    sim.scripts.spawn_once(sim.scripts.players.len(), 0x0600_005b, ident);
    for _ in 0..600 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.bond.cam_mode == 0 { break; }
    }
    assert_eq!(sim.bond.cam_mode, 0, "the script's event 9 runs the exit arm's SetCamMode(0) [M 0x195fc8:13]");
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the exit arm's ChangeSubState(0) [M 0x195fc8:14]");
}

/// The orbit-enter arm (mode 3) drives `Player_PositionCamera` case 2's camera — the parameters
/// are decompile-pinned ([M 0x18f5d8 case 2:145-156]; no disc script fires mode 3 [D census]).
/// Driven through the real hand-off drain (`player_events` — the same route the script's event
/// rides).
#[test]
fn event9_mode3_drives_the_behind_bond_orbit() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 01");
    let sim = scene.sim.as_mut().unwrap();
    // A distinctive pose: off-origin, rotated — so the orbit math is exercised, not identity.
    sim.bond.pos = [12.0, 3.5, -7.0];
    sim.bond.yaw = 1.1;

    sim.drones.player_events.push(nf_game::nis::Request::Cam2Mode { mode: 3 });
    sim.step_bond(Default::default(), false, false, false);
    assert_eq!(sim.bond.cam_mode, 2, "mode 3 → Player_SetCamMode(BL, 2) [M 0x195fc8:8]");

    // `camera_track_XZ_pos_behind(1.0, 10.0, obj+0x30, obj+0x50, viewer)` [M 0x1bc260]: the
    // camera sits 10 units behind Bond on the XZ plane — pos − 10·(sin yaw, cos yaw) — and
    // `camera_track_Y_pos(1.0, 0.0, obj, viewer)` [M 0x1bc1f0] keeps Bond's own Y anchor. The
    // expectation reads the pose the tick settled on (the physics may have drifted it).
    let pos = sim.bond.pos;
    let yaw = sim.bond.yaw;
    let want_cam = [pos[0] - 10.0 * yaw.sin(), pos[1], pos[2] - 10.0 * yaw.cos()];
    let Some((cam, look_yaw, look_pitch, fov)) = sim.cam2_camera() else {
        panic!("the orbit camera must be Some while cam_mode == 2");
    };
    assert!((cam[0] - want_cam[0]).abs() < 1e-4 && (cam[1] - want_cam[1]).abs() < 1e-4 && (cam[2] - want_cam[2]).abs() < 1e-4,
        "the camera sits 10 units behind Bond on the XZ plane at Bond's Y [M 0x1bc260/0x1bc1f0]");
    let want_yaw = (pos[0] - cam[0]).atan2(pos[2] - cam[2]);
    assert!((look_yaw - want_yaw).abs() < 1e-4, "the look snaps to Bond (atan2(dx, dz)) [M 0x1bc0c0]");
    assert!(look_pitch.abs() < 1e-4, "same-anchor Y ⇒ pitch 0 [M 0x1bc148]");
    assert_eq!(fov, 1.0, "viewer+0x108 = 1.0 — the unzoomed FOV factor [M 0x18f5d8:154]");

    // The exit arm returns the eye camera [M 0x195fc8:11-16].
    sim.drones.player_events.push(nf_game::nis::Request::Cam2Mode { mode: 0 });
    sim.step_bond(Default::default(), false, false, false);
    assert_eq!(sim.bond.cam_mode, 0, "the exit arm's SetCamMode(0)");
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the exit arm's ChangeSubState(0)");
    assert!(sim.cam2_camera().is_none(), "the orbit camera is gone once cam_mode leaves 2");
}
