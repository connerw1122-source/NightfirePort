//! safe-cutscene-actors-08: the 07000008 station-intro cut-scene (0x06000077, 3880 ticks) — its
//! script's nine `Script_AnimStart` actors, through the REAL start path, every one live, bound to
//! its skin and posed for the whole early window, and drawn by the render path's palette.
//!
//! The playtest (1013-1758): "Kiko is the only one showing in this cutscene besides me... I can't
//! see any other models. Any character models." The regression was fix-cutscenes2's
//! first-writer-wins window composition (d907aba): the intro's face-track windows won the shared
//! pose's low bones and their zero channels collapsed every script actor's root to the world
//! origin — off-camera in every shot, on 07000007 and 07000008 (docs/reports/fix-cutscene-draw.md).
//! fix-cutscene-draw gave the base (widest) track the bones it covers; these tests pin the 08
//! scene against a re-appearance of that signature (a live-but-at-the-origin actor passes a
//! liveness check and still shows nothing — the root must sit at the body track's library mark).
//!
//! Every pinned value is disc data: the scene hash and its nine (stream, skin, anim script)
//! triples are the pack 07800008.bin's stream decode [D]; the start plate TouchOnce #237
//! (139.2, 49.5, −81.2, out 0x49) and the spawn #0 (139.5, 50.5, −76.4) are the placements dump
//! [D docs/levels/07000008.md]. Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The station intro 0x06000077's actors: (stream, skin, anim script) — the pack's stream decode
/// [D 07800008.bin]. Kiko is stream 6 (0x0500003b, the same skin the Kimiko scene used on 07).
const ACTORS: [(u8, u32, u32); 9] = [
    (1, 0x0500_0014, 0x0600_020a), (2, 0x0500_0013, 0x0600_0209), (3, 0x0500_001b, 0x0600_020c),
    (4, 0x0500_000d, 0x0600_0207), (5, 0x0500_0016, 0x0600_020b), (6, 0x0500_003b, 0x0600_03ae),
    (7, 0x0500_0012, 0x0600_0208), (8, 0x0500_0026, 0x0600_0206), (9, 0x0500_0018, 0x0600_03af),
];

fn scene_char(scene: &nf_view::scene::Scene, st: u8, skin: u32) -> Option<usize> {
    scene.characters.iter().position(|c| c.script == Some((0x0600_0077, st)) && c.skin.hashcode == skin)
}

fn start_the_scene(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000008.bin", false).expect("07000008.bin loads");
    {
        let sim = scene.sim.as_mut().unwrap();
        // Onto the intro plate (the spawn is ~3.5 m from it; the plate fires on the first ticks).
        sim.bond.pos = [139.2, 49.5, -81.2];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(
        scene.sim.as_ref().unwrap().drones.scripts.players.iter()
            .any(|p| p.scripts.contains(&0x0600_0077) && p.state == 2),
        "the station intro 0x06000077 did not start from the plate #237 (out 0x49)"
    );
    scene
}

/// The census: every one of the scene's nine actors must be (a) created — a live `Obj::Anim`
/// (`script_anim` resolves), (b) bound — a pre-built character with that (script, stream) and a
/// skin that resolved from the level's own archive, (c) posed — `script_pose_together` finite
/// with the root at the body track's library mark, NOT the origin (the fix-cutscenes2 collapse).
#[test]
fn the_station_intro_actors_live_and_posed_through_the_plate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = start_the_scene(&iso);
    // (b) every script actor pre-built and skin-bound at load.
    for &(st, skin, _) in &ACTORS {
        let ci = scene_char(&scene, st, skin);
        assert!(ci.is_some(), "stream {st}: the scene's skin {skin:08x} built no character (unresolved skin or a missed Cmd::Anim)");
        assert!(scene.sim.as_ref().unwrap().skins.contains_key(&skin), "skin {skin:08x} is not in the level's skins");
    }
    let mut last = 0u32;
    for extra in [30u32, 120, 240, 480, 960] {
        for _ in 0..(extra - last) { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        last = extra;
        let sim = scene.sim.as_ref().unwrap();
        let sp = sim.drones.scripts.players.iter().find(|p| p.scripts.contains(&0x0600_0077)).expect("SP #294");
        assert_eq!(sp.state, 2, "the intro SP left state 2 at +{last}");
        for &(st, skin, asc) in &ACTORS {
            let Some((script, frame)) = sim.script_anim((0x0600_0077, st), skin) else {
                panic!("[+{last}] stream {st} skin {skin:08x}: no live Obj::Anim (the actor collapsed out of frame_state)");
            };
            assert_eq!(script, asc, "[+{last}] stream {st}: the live Obj::Anim binds the wrong anim script");
            let ci = scene_char(&scene, st, skin).expect("pre-built (checked above)");
            let pose = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &scene.characters[ci].skeleton, &scene.characters[ci].skin, script, frame)
                .unwrap_or_else(|| panic!("[+{last}] stream {st}: script_pose_together None for {script:08x} @ {frame}"));
            assert!(!pose.is_empty() && pose.iter().all(|(t, q)| t.iter().all(|v| v.is_finite()) && q.iter().all(|v| v.is_finite()) && q.iter().any(|v| *v != 0.0)),
                "[+{last}] stream {st}: non-finite/zeroed pose for {script:08x} @ {frame}");
            let root = pose[0].0;
            let at_origin = root.iter().all(|v| v.abs() < 0.01);
            assert!(!at_origin, "[+{last}] stream {st} skin {skin:08x}: the root collapsed to the world origin at {script:08x} @ {frame} (the fix-cutscenes2 signature — the face track won the low bones again)");
        }
    }
}

/// The render path: with the script camera driving the eye and the portal walk feeding
/// `skin_palette`'s room mask, none of the nine actors' palettes may go FAR (room-hidden) or
/// collapse — what the live draw loop uploads while the scene plays.
#[test]
fn the_station_intro_actors_survive_the_render_path() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = start_the_scene(&iso);
    let g = scene.cel_graph.clone();
    let (layout, _) = scene.skin_layout();
    let far = [[0.0f32; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
    let mut last = 0u32;
    for extra in [30u32, 240, 960] {
        for _ in 0..(extra - last) { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        last = extra;
        let sim = scene.sim.as_ref().unwrap();
        let Some((pos, yaw, pitch, _fov)) = sim.script_camera() else { panic!("no script camera at +{last}") };
        let world = scene.sim.as_ref().map(|s| &s.world);
        let vp = view_proj(pos, yaw, pitch, 16.0 / 9.0);
        let depth0 = depth0_game(vp, pos);
        let mut state = nf_view::scene::ViewerCel::default();
        let cell = nf_view::scene::eye_cell(&g, &mut state, pos, world);
        let vis: Option<Vec<bool>> = cell.and_then(|c| nf_view::scene::portal_visible_cels_from_cell(&g, c, pos, &depth0))
            .map(|cell_vis| g.room_to_cell.iter().map(|m| m.map(|c2| cell_vis[c2]).unwrap_or(true)).collect());
        let (pal, _) = scene.skin_palette(&layout, None, 300.0, nf_view::scene::ViewOpts::default(), vis.as_deref());
        for &(st, skin, _) in &ACTORS {
            let ci = scene_char(&scene, st, skin).expect("pre-built");
            let base = layout.char_base[ci] as usize;
            let nb = scene.characters[ci].skeleton.bones as usize;
            let mats = &pal[base..=(base + nb).min(pal.len() - 1)];
            assert!(!mats.iter().all(|m| *m == far), "[+{last}] stream {st} skin {skin:08x}: palette FAR (room-hidden) at the script camera's eye cell {cell:?}");
            assert!(!mats.iter().all(|m| m[0] == [0.0; 4] && m[1] == [0.0; 4] && m[2] == [0.0; 4]),
                "[+{last}] stream {st} skin {skin:08x}: palette collapsed (frame_state hidden)");
        }
    }
}

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near_p, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near_p - far), -1.0],
        [0.0, 0.0, near_p * far / (near_p - far), 0.0],
    ]);
    let cp = pitch.cos();
    let fw = Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let view = Mat4::from_cols_array_2d(&[
        [s.x, u.x, -fw.x, 0.0],
        [s.y, u.y, -fw.y, 0.0],
        [s.z, u.z, -fw.z, 0.0],
        [-s.dot(Vec3::from(pos)), -u.dot(Vec3::from(pos)), fw.dot(Vec3::from(pos)), 1.0],
    ]);
    (proj * view).to_cols_array_2d()
}

fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}
