//! fix-cutscene-letterbox — the cut-scene picture must match the game's. The main agent's PCSX2
//! compare of the 07000007 Kimiko scene (trigger 0x48, script 0x06000073) found two gaps:
//! (1) the game draws the black letterbox bars top and bottom through the whole scene
//! (`Script_CameraStart` 0x1d9140 creates the two 0x2c9468 border sprites — 512×64 at y 0 and
//! y 0x180, draw layer 1, texture 0x03000002 — and `Script_KillStream` 0x1d9860 case 4 hides them
//! by layer 0xff; no fade either way) and the port's headless `--shot` renders drew none: the
//! shot path's HUD was `overlay_for` alone, which is EMPTY while a script camera runs (the
//! gameplay-HUD gate) — the script overlay was never appended there (the window path did);
//! (2) the port's widescreen render shows more at the sides than the game's 4:3 frame (at frame
//! 400 Dominique waits in the left doorway, visible in the port, outside the game's frame): the
//! port projected every frame at the window's aspect, the game rendered the cut-scene at the
//! console's 4:3.

use nf_game::player::Input;

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return None };
    match nf_view::load(&iso, archive, false) {
        Ok(s) => Some(s),
        Err(e) => { eprintln!("load {archive}: {e}"); None }
    }
}

fn playing(s: &nf_view::scene::Scene, hash: u32) -> bool {
    s.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| p.scripts.contains(&hash) && p.state == 2 && p.info[p.cur].as_ref().is_some_and(|i| i.playing()))
}

/// Drives the real trigger path: Bond into TouchOnce #412's volume → SP #420 plays 0x06000073,
/// then `ticks` more sim steps. Returns the scene.
fn kimiko_scene(ticks: u32) -> Option<nf_view::scene::Scene> {
    let mut scene = load("07000007.bin")?;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start from trigger 412");
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    Some(scene)
}

/// The two bar quads of the script overlay frame, found by texture: (run start, 12 vertices).
fn bar_verts<'a>(frame: &'a nf_view::gpu::HudFrame, scene: &nf_view::scene::Scene) -> &'a [nf_view::gpu::HudVertex] {
    let strip = 0x0300_0002;
    let tex_pos = scene.hud_textures.iter().position(|(h, _)| *h == strip)
        .unwrap_or_else(|| panic!("the 0x03000002 strip texture (the bars' texture [L SpriteInfo 0x2c9468+0x24]) is not in 07000007's hud_textures — the bars' quads silently no-op in hud::quad"));
    let run = frame.runs.iter().find(|&&(t, _, _, c)| t == tex_pos + 1 && c == 12)
        .unwrap_or_else(|| panic!("the script overlay must carry the two bar quads (12 vertices of the strip texture); runs {:?}",
            frame.runs.iter().map(|&(t, b, f, c)| (t, b, c)).collect::<Vec<_>>()));
    &frame.v[run.2 as usize..(run.2 + run.3) as usize]
}

/// (1) The bars, through the real trigger path. While the scene's script camera runs, the drone's
/// letterbox flag is up, the script camera is live, and the frame the headless shot draws
/// (`shot_hud` — what `--shot` assembles since the fix; the window path appends the same) carries
/// the two solid black bars: full width, top touching y 0, bottom touching y 448, 64 px tall each
/// [M Script_CameraStart 0x1d9140:49-61; L SpriteInfo 0x2c9468: w 0x200 h 0x40, second at +0x26 =
/// 0x180, texture 0x03000002, colour 0x000000ff]. The bars hold through the scene (the camera
/// stream lives the whole 4326-tick scene; `Script_KillStream` only drops them when it dies).
#[test]
fn the_kimiko_scene_raises_the_letterbox_and_the_shot_carries_the_bars() {
    let Some(scene) = kimiko_scene(110) else { return }; // ~frame 200 of the viewer's run
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.letterbox, "the script camera request must raise the drone letterbox flag");
    assert!(sim.script_camera().is_some(), "the script camera must be live");
    // The gameplay overlay the shot path used to draw alone is empty while the camera runs
    // (the same gate the port's HUD blank rides) — the bars can only come from the script overlay.
    assert!(nf_view::overlay_for(&scene, 16.0 / 9.0).runs.is_empty(), "the gameplay HUD is blank during the cut-scene");
    let frame = nf_view::shot_hud(&scene, 16.0 / 9.0, None);
    let bars = bar_verts(&frame, &scene);
    let (ys, xs): (Vec<f32>, Vec<f32>) = (bars.iter().map(|v| v.p[1]).collect(), bars.iter().map(|v| v.p[0]).collect());
    assert!(ys.iter().any(|&y| y > 0.99), "the top bar must touch the screen top, ys {ys:?}");
    assert!(ys.iter().any(|&y| y < -0.99), "the bottom bar must touch the screen bottom, ys {ys:?}");
    assert!(xs.iter().any(|&x| x < -0.99) && xs.iter().any(|&x| x > 0.99), "the bars span the full width, xs {xs:?}");
    for v in bars {
        assert!(v.c[0] < 0.01 && v.c[1] < 0.01 && v.c[2] < 0.01 && v.c[3] > 0.99, "the bars are solid black, colour {:?}", v.c);
    }
    // Through the scene: the flag holds at the driver's compare frames 400 / 1000 / 1400.
    let mut scene = scene;
    for t in (110..1400).step_by(1) {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if [400, 1000, 1400].contains(&(t + 1)) {
            assert!(scene.sim.as_ref().unwrap().drones.letterbox, "the letterbox must hold at frame {}", t + 1);
            assert!(scene.sim.as_ref().unwrap().script_camera().is_some(), "the script camera must still run at frame {}", t + 1);
        }
    }
}

/// Sister: before the trigger, no bars — the letterbox flag starts down and the frame the shot
/// draws carries no script bars while no script camera runs.
#[test]
fn no_letterbox_bars_before_the_scene_starts() {
    let Some(scene) = load("07000007.bin") else { return };
    let strip = 0x0300_0002;
    let Some(tex_pos) = scene.hud_textures.iter().position(|(h, _)| *h == strip) else {
        eprintln!("strip texture not loaded in this level — nothing to assert"); return;
    };
    assert!(!scene.sim.as_ref().unwrap().drones.letterbox, "the letterbox flag starts down");
    let frame = nf_view::shot_hud(&scene, 16.0 / 9.0, None);
    assert!(!frame.runs.iter().any(|&(t, _, _, _)| t == tex_pos + 1), "no script bars before a script camera runs");
}

/// (2) The extent. `cutscene_frame` frames the cut-scene at the game's own 4:3: a 16:9 window
/// pillar-boxes the game picture into the central 1280×720→960×720 band (the shot target's band
/// is [160, 0, 960, 720]); a 4:3 window keeps the full frame. The projection aspect is 4/3 in
/// both — the game's own — never the window's.
#[test]
fn the_cutscene_frame_is_the_games_43_band() {
    let (aspect, band) = nf_view::cutscene_frame(1280.0, 720.0);
    assert!((aspect - 4.0 / 3.0).abs() < 1e-6, "the cut-scene projects at the game's 4:3, got {aspect}");
    assert_eq!(band, [160.0, 0.0, 960.0, 720.0], "the band in a 1280×720 shot");
    let (aspect, band) = nf_view::cutscene_frame(960.0, 720.0);
    assert!((aspect - 4.0 / 3.0).abs() < 1e-6, "the cut-scene projects at the game's 4:3, got {aspect}");
    assert_eq!(band, [0.0, 0.0, 960.0, 720.0], "a 4:3 window keeps the full frame");
    let (_, band) = nf_view::cutscene_frame(1024.0, 768.0);
    assert_eq!(band, [0.0, 0.0, 1024.0, 768.0], "a 4:3 window keeps the full frame");
}

/// (2, geometry, through the real path) The symptom the compare showed: at the frame-400 script
/// camera the widescreen frustum the port used to render with sees cast members at the sides
/// that the game's 4:3 frame excludes. The scene's script characters are posed from their
/// sequence roots (the object sits at the origin, the root carries the world position
/// [M Obj::Anim doc; scene.rs's script-character pose path]); at the compare's frame 400 at
/// least one live scene actor — Dominique waiting in the left doorway per the PCSX2 compare —
/// lies inside the 16:9 frustum but outside the 4:3 one, and the wide frustum sees strictly
/// more of the cast.
#[test]
fn the_frame_400_camera_sees_side_content_the_games_frame_excludes() {
    use glam::{Mat4, Vec3};
    use nf_game::anim::script_pose_together;
    // 90 settle ticks + 310: the script clock lands on the compare's frame 400 (the trigger
    // fires on the first settle tick — the probe prints the camera and the cast's angles).
    let Some(scene) = kimiko_scene(310) else { return };
    let sim = scene.sim.as_ref().unwrap();
    let (pos, yaw, pitch, fov) = sim.script_camera().expect("the script camera is live at frame 400");
    let frustum = |aspect: f32| {
        // The port's projection (`Camera::view_proj`): vertical fov, horizontal from the aspect.
        let f = 1.0 / (fov * 0.5).tan();
        let (near, far) = (0.05f32, 4000.0);
        let proj = Mat4::from_cols_array_2d(&[[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]]);
        let fw = Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()).normalize();
        let s = fw.cross(Vec3::Y).normalize();
        let u = s.cross(fw);
        let view = Mat4::from_cols_array_2d(&[[s.x, u.x, -fw.x, 0.0], [s.y, u.y, -fw.y, 0.0], [s.z, u.z, -fw.z, 0.0], [-s.dot(pos.into()), -u.dot(pos.into()), fw.dot(pos.into()), 1.0]]);
        nf_view::gpu::extract_frustum((proj * view).to_cols_array_2d())
    };
    let inside = |pl: &[[f32; 4]; 6], p: [f32; 3]| pl.iter().all(|q| q[0] * p[0] + q[1] * p[1] + q[2] * p[2] + q[3] >= 0.0);
    let (wide, game) = (frustum(16.0 / 9.0), frustum(4.0 / 3.0));
    // Every character's render position right now: a live script actor's pose root (the pose
    // carries the object), everyone else their placement matrix (the waiting cast — Dominique —
    // stands in her doorway start pose; the scene's anim stream has not started for her yet).
    let mut side = Vec::new();
    let (mut n_wide, mut n_game) = (0usize, 0usize);
    for (ci, c) in scene.characters.iter().enumerate() {
        let p = match sim.char_script[ci] {
            Some(src) => sim.script_anim(src, c.skin.hashcode)
                .and_then(|(s, f)| script_pose_together(&sim.scripts, &sim.seqs, &c.skeleton, &c.skin, s, f))
                .map(|pose| pose[0].0)
                .unwrap_or([c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]]),
            None => [c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]],
        };
        (n_wide, n_game) = (n_wide + inside(&wide, p) as usize, n_game + inside(&game, p) as usize);
        if inside(&wide, p) && !inside(&game, p) { side.push((ci, p)); }
    }
    eprintln!("cast: {n_wide} in the 16:9 frustum, {n_game} in the game's 4:3 frame; side-band {side:?}");
    assert!(!side.is_empty(), "no live scene actor sits in the band the widescreen view added — the compare's frame-400 symptom (Dominique visible in the port, outside the game's frame) did not reproduce");
    assert!(n_wide > n_game, "the widescreen frustum must see more of the cast than the game's frame ({n_wide} vs {n_game})");
}
