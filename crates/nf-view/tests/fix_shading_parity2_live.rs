//! Headless NF_ISO tests for `View_SetupRenderModes2` 0x1e7908 (fix-shading-parity2): character
//! skins with BOTH skinned main meshes and rigid extra parts (runtime skin+0x2c/0x2d, disc +0x10/
//! +0x11 — `AnimProcessSkinData` 0x1bd500:93-96) pick their two lights from the CAMERA's sphere
//! [viewer+0xc0, radius 0x40000000 = 2.0] instead of the object's (obj+0x80). The caller gate is
//! `AnimObjectDraw` 0x1c4490:118-122 (`skin+0x2c != 0 && skin+0x2d != 0` → the `2` function);
//! `View_SetupRenderModes` 0x1e78b0 (the other arm) keeps obj+0x80, and every draw of such a skin —
//! its rigid attachments included — rides that one upload (they draw inside the same
//! `AnimObjectDraw` call).
//!
//! The port threaded `Material::cam_pick` from `skin_cam_pick(skin)` at the character call sites;
//! `gpu::light_pick_sphere` returns (eye, 2.0) for those draws. Before the fix every character draw
//! picked from its own sphere — on a close-up (the 07000007 Kimiko script camera slides to arm's
//! length) the game uploads the lights nearest the CAMERA, the port the lights nearest the
//! CHARACTER, so the two uploaded pairs disagree exactly when the camera closes in.
//!
//! Opt-in via `NF_ISO` (the numeric test runs everywhere).

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Step the sim `n` ticks with no input (the viewer's per-frame `step_bond` — the real input path).
fn step(scene: &scene::Scene, n: usize) {
    let sim = scene.sim.as_ref().unwrap() as *const scene::Sim as *mut scene::Sim;
    for _ in 0..n {
        unsafe { (*sim).step_bond(Default::default(), false, false, false) };
    }
}

/// The port's gate (scene::skin_cam_pick is private; the same predicate on the pub fields).
fn skin_hits_setup_render_modes2(skin: &nf_formats::character::Skin) -> bool {
    !skin.main.is_empty() && !skin.extra.is_empty()
}

/// Two lights, one at the character, one at the camera — the pick spheres decide which pair uploads.
fn two_lights() -> nf_game::light::ResolvedLights {
    let light = |pos: [f32; 3]| nf_game::light::ResolvedLight {
        pos: [pos[0], pos[1], pos[2], 1.0],
        track_pos: [pos[0], pos[1], pos[2], 1.0],
        rgb: [255.0, 255.0, 255.0],
        intensity: 1.0,
        inv_r2: 0.01,
        kind: 3,
        active: true,
        track: None,
    };
    nf_game::light::ResolvedLights { lights: vec![light([10.0, 1.0, 10.0]), light([20.0, 1.0, 20.0])] }
}

/// The first test: the camera-sphere pick itself. Numeric — runs without the disc.
#[test]
fn the_setup_render_modes2_skin_picks_lights_from_the_camera() {
    let lights = two_lights();
    let eye = [20.0, 1.0, 20.0];
    let char_centre = [10.0, 1.0, 10.0];
    // The object sphere (obj+0x80, `View_SetupRenderModes` 0x1e78b0): the character's own sphere.
    let obj_sphere = (char_centre, 3.0);
    // The camera sphere (`View_SetupRenderModes2` 0x1e7908): viewer+0xc0, radius 0x40000000 = 2.0.
    let cam_sphere = (eye, 2.0);
    let object_pick = lights.pick2([obj_sphere.0[0], obj_sphere.0[1], obj_sphere.0[2], obj_sphere.1], 0x8a0);
    let camera_pick = lights.pick2([cam_sphere.0[0], cam_sphere.0[1], cam_sphere.0[2], cam_sphere.1], 0x8a0);
    // The pre-fix disagreement: the two spheres upload DIFFERENT pairs (lamp A leads the object
    // pick, lamp B the camera pick) — the user-visible delta on a close-up.
    assert_ne!(object_pick.0, camera_pick.0, "the two spheres must upload different light positions");
    assert_eq!(object_pick.0[0], [10.0, 1.0, 10.0, 1.0], "the object sphere picks the lamp at the character");
    assert_eq!(camera_pick.0[0], [20.0, 1.0, 20.0, 1.0], "the camera sphere picks the lamp at the camera");
    // The port's decision function: a cam_pick draw returns EXACTLY the game's camera sphere
    // (viewer+0xc0, r 2.0) — with a cel sphere available (the arm must not consult it) and without.
    let cel_spheres = [([9.0, 1.0, 9.0], 5.0)];
    assert_eq!(nf_view::gpu::light_pick_sphere(true, Some(0), 7, char_centre, 3.0, eye, &cel_spheres), cam_sphere,
        "a SetupRenderModes2 skin draws pick [eye, 2.0], ignoring any cel sphere");
    assert_eq!(nf_view::gpu::light_pick_sphere(true, None, u32::MAX, char_centre, 3.0, eye, &[]), cam_sphere,
        "the camera pick needs no cel sphere");
    // Sisters: the other arms keep the old spheres (the pre-fix behaviour, now the guarded arm).
    assert_eq!(nf_view::gpu::light_pick_sphere(false, Some(0), 7, char_centre, 3.0, eye, &cel_spheres), ([9.0, 1.0, 9.0], 5.0),
        "a room-class cel glist keeps the cel sphere");
    assert_eq!(nf_view::gpu::light_pick_sphere(false, None, u32::MAX, char_centre, 3.0, eye, &cel_spheres), (char_centre, 3.0),
        "an object draw keeps obj+0x80 (the material centre/radius)");
}

/// The gate on a live level: 07000005's view-model skins (a sleeve mesh plus rigid gun parts) and
/// any third-person skins with main+extra flag cam_pick; world and effect draws never do.
#[test]
fn the_camera_pick_flags_the_right_draws_on_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    step(&scene, 5);
    // The skins that hit `View_SetupRenderModes2`: characters whose skin has main AND extra.
    let hitting: Vec<usize> = (0..scene.characters.len())
        .filter(|&ci| skin_hits_setup_render_modes2(&scene.characters[ci].skin))
        .collect();
    assert!(!hitting.is_empty(), "no 07000005 skin hits View_SetupRenderModes2 — the gate would be dead code");
    eprintln!("07000005 characters hitting View_SetupRenderModes2: {} of {} (view-model sleeve+parts skins included)", hitting.len(), scene.characters.len());
    // The flag reached the materials: at least one Character-layer draw of a hitting skin says
    // cam_pick (every draw of such a skin rides the same AnimObjectDraw upload).
    let char_true = scene.materials.iter().filter(|m| m.layer == scene::Layer::Character && m.cam_pick).count();
    assert!(char_true > 0, "no Character-layer material carries cam_pick");
    // Sisters: world and effect draws keep the object path (obj+0x80 / cel+0x80).
    assert!(scene.materials.iter().any(|m| m.layer == scene::Layer::World && !m.cam_pick), "world draws must keep the object sphere");
    assert!(scene.materials.iter().any(|m| m.layer == scene::Layer::Effect && !m.cam_pick), "effect draws must keep the object sphere");
    // No non-character draw ever picks from the camera (the game's gate lives in AnimObjectDraw).
    assert!(!scene.materials.iter().any(|m| m.layer != scene::Layer::Character && m.cam_pick),
        "a non-Character draw carries cam_pick — the gate leaked off the anim-object path");
}

/// The census: which skins hit `View_SetupRenderModes2` — parsed straight off the disc. Prints the
/// list per level (the report's "which skins" section) and asserts the gate is live on both the
/// playtest level and the cut-scene level.
#[test]
fn the_skin_census_of_view_setup_render_modes2() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for archive in ["07000005.bin", "07000007.bin"] {
        let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open iso");
        let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
        let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
        let level = nf_formats::level::Level::parse(&data).expect("parse level");
        let mut hitting = Vec::new();
        for (hash, skin) in &level.skins {
            if skin_hits_setup_render_modes2(skin) { hitting.push(*hash); }
        }
        hitting.sort_unstable();
        eprintln!("{archive}: {} skins, {} hit View_SetupRenderModes2: {:08X?}", level.skins.len(), hitting.len(), hitting);
        assert!(!hitting.is_empty(), "{archive}: no skin hits View_SetupRenderModes2");
        // Every hitting skin's extra parts are real rigid parts (bone, hash) — the game's +0x2d list.
        for h in &hitting {
            let skin = &level.skins[h];
            assert!(!skin.extra.iter().all(|(_, part)| part.is_none()), "{archive} skin {h:08X}: extra carries no hashes");
        }
    }
}
