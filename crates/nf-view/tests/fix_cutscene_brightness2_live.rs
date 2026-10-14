//! fix-cutscene-brightness2: the held brightness fix (102g, 0cbfbf8) makes the 07000002 intro's
//! cut-scene actors VANISH — the main agent's render at the user's spot
//! (`--bond -82.30,5.30,32.05,1.570,-0.100 --frame 170`) shows master drawing Bond and the man at
//! the computer desk (too bright), master + 0cbfbf8 drawing the same room with NO characters.
//!
//! Root cause [L live probe, this file's data on the held tree]: the held fix substituted
//! `Player::live_matrix` for the script actors' object matrix — but `live_matrix` is stream 0's
//! object ∘ the script matrix (`Script_Interp` 0x1d9a08 keys the SP object's obj+0x90 from stream
//! 0 [M nis.rs]), and on the intro stream 0 is the CAMERA stream: obj[0] Camera trans
//! [-27.25, 1.41, 34.39] == player[1] live trans. The actors were drawn inside the camera.
//! The premise was wrong too: VECTOR 0x2d6200 is the ORIGIN vector [L ACTION.ELF, nis.rs's Light
//! doc], so `Script_AnimStart` 0x1d8f40's `control_create_object(0, 0x2d6200, 0x2d6200, 0)`
//! creates the anim object at the origin — the object matrix is identity and the sequence's root
//! carries the actor in world space [M Obj::Anim doc; `AnimSeqTick` 0x1c28d0 takes the root motion
//! as the object's movement]. The held fix also moved the ambient ramp's sample point onto that
//! same camera matrix (its `script_pose` resolved the LAST covering window — the intro's zero-root
//! overlay track, root [0,0,0] — and composing the "base" landed the sample in the camera's cel):
//! the ramp must sample where the actor is DRAWN, i.e. the render path's own `script_pose_together`
//! root, which on the intro is the body track's world root [-25.51, 1.13, 30.14], beside Bond
//! [-27.17, 1.05, 29.80].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_02_intro(ticks: u32) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso().as_deref().expect("NF_ISO"), "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    let idle = Input::default();
    // Into the intro (the user's screenshot is frame ~170): no skip — the cut-scene plays.
    for _ in 0..ticks { sim.step_bond(idle, false, false, false); }
    scene
}

/// The live NIS camera (obj[0], `Script_CameraStart`): position, world matrix (rows right, up,
/// forward, position [M nis.rs Obj::Camera doc]) and vertical fov in radians.
fn script_camera(sim: &nf_view::scene::Sim) -> Option<([f32; 3], [[f32; 4]; 4], f32)> {
    sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
        nf_game::nis::Obj::Camera { matrix, fov, .. } =>
            Some(([matrix[3][0], matrix[3][1], matrix[3][2]], *matrix, *fov)),
        _ => None,
    })
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// The point's screen offsets through the game camera: None = behind or outside the frustum
/// (4:3 — the port's screen [M Camera_CalcViewAngles 0x1baf70's viewer+0x114 = 4/3 default]).
fn screen_of(cam_pos: [f32; 3], cam: [[f32; 4]; 4], fov: f32, p: [f32; 3]) -> Option<(f32, f32)> {
    let v = [p[0] - cam_pos[0], p[1] - cam_pos[1], p[2] - cam_pos[2]];
    let dot = |r: [f32; 4]| r[0] * v[0] + r[1] * v[1] + r[2] * v[2];
    let depth = dot(cam[2]);
    if depth <= 0.05 { return None; }
    let t = (fov * 0.5).tan();
    let (x, y) = (dot(cam[0]) / depth, dot(cam[1]) / depth);
    (x.abs() <= t * 4.0 / 3.0 + 0.05 && y.abs() <= t + 0.05).then_some((x, y))
}

/// The intro's two live script actors must DRAW in the cut-scene set — object matrix = the
/// character's own (identity: the object sits at the origin, the pose's root carries it in world
/// space), the drawn world position the anim's root beside Bond, visible through the intro's NIS
/// camera, lit dark (the room's ambient, non-zero and non-white). On the held tree the object
/// matrix was the NIS camera, which drew the actors inside it: nothing on screen.
#[test]
fn the_02_intro_actors_draw_in_the_set_not_at_the_camera() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_02_intro(170);
    let bond = scene.sim.as_ref().unwrap().bond.pos;
    let fs = scene.frame_state(None, 170.0, nf_view::scene::ViewOpts::default());
    let sim = scene.sim.as_ref().unwrap();
    let (cam_pos, cam, fov) = script_camera(sim).expect("the intro's NIS camera is live at frame 170");
    let mut seen = 0;
    for ci in 0..sim.char_script.len() {
        let Some(src) = sim.char_script[ci] else { continue };
        let Some(skin) = sim.char_script_skin[ci] else { continue };
        if sim.script_anim(src, skin).is_none() { continue; }
        seen += 1;
        let (_, world, m) = fs.posed[ci].as_ref()
            .unwrap_or_else(|| panic!("the live intro actor char[{ci}] is posed — master draws it (the main agent's render)"));
        assert!(!fs.hidden[ci], "the live intro actor char[{ci}] is drawn");
        // The render-verified object model [M Obj::Anim doc; 0x2d6200 is the origin vector
        // [L ACTION.ELF] → Script_AnimStart 0x1d8f40 creates the object at the origin]: the
        // object matrix is the character's own, not the NIS player's live (stream-0/camera) matrix.
        let cm = scene.characters[ci].matrix;
        for r in 0..4 { for k in 0..4 {
            assert!((m[r][k] - cm[r][k]).abs() < 1e-4,
                "char[{ci}]'s object matrix is the character's own (identity) — got {:?}, character {:?}; \
                 the held fix's live_matrix put the actors at the NIS camera and they vanished",
                m[r], cm[r]);
        }}
        // The drawn world position: the pose's root, in the set beside Bond (the main agent's
        // master render: Bond in the doorway, the man by the computer desk).
        let root = [world[0][3][0], world[0][3][1], world[0][3][2]];
        let d = dist(root, bond);
        assert!(d < 3.0, "char[{ci}] draws at its anim's world root {root:?} beside Bond {bond:?} (dist {d:.2}) — not off in the scene graph");
        // Visible through the intro's own NIS camera (the user's beat: both actors on screen).
        let (x, y) = screen_of(cam_pos, cam, fov, root)
            .unwrap_or_else(|| panic!("char[{ci}] at {root:?} is OUTSIDE the intro camera's frustum (cam {cam_pos:?})"));
        // The colour: the room's dark ambient — non-zero (drawn), non-white (not the old bug).
        let mul = sim.char_multiplier(ci);
        assert!(mul.iter().all(|&f| f > 0.0), "char[{ci}]'s draw multiplier is non-zero (got {mul:?}) — a zeroed actor is the vanish in another coat");
        assert!(mul.iter().all(|&f| f < 0.5), "char[{ci}]'s draw multiplier is the dark room ambient, not the white sentinel (got {mul:?})");
        eprintln!("char[{ci}] root {root:?} screen ({x:.2},{y:.2}) mul {mul:?}");
    }
    assert_eq!(seen, 2, "the intro's two live script actors at frame 170 [D the main agent's master render: Bond and the man at the desk]");
}

/// Sister: the ambient ramp samples the DRAWN position — the render path's own root (in a cel,
/// beside Bond), never the NIS camera's matrix and never None. On the held tree the sample was the
/// camera [-27.25, 1.41, 34.39] (its cel — the right ambient by luck); on master it was None (the
/// strict filter) and the actors stayed the 0xff sentinel.
#[test]
fn the_ambient_ramp_samples_the_drawn_position_not_the_camera() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load_02_intro(170);
    let sim = scene.sim.as_ref().unwrap();
    let (cam_pos, _, _) = script_camera(sim).expect("the intro's NIS camera is live at frame 170");
    let bond = sim.bond.pos;
    let mut seen = 0;
    for ci in 0..sim.char_script.len() {
        let Some(src) = sim.char_script[ci] else { continue };
        let Some(skin) = sim.char_script_skin[ci] else { continue };
        if sim.script_anim(src, skin).is_none() { continue; }
        seen += 1;
        let pos = sim.characters_script_pos(ci)
            .unwrap_or_else(|| panic!("char[{ci}]'s ramp position resolves — the render's own resolution, not the strict filter's None"));
        let dc = dist(pos, cam_pos);
        assert!(dc > 0.5, "char[{ci}]'s ramp sample {pos:?} is not the NIS camera {cam_pos:?} (dist {dc:.2})");
        assert!(dist(pos, bond) < 3.0, "char[{ci}]'s ramp sample {pos:?} is the drawn spot beside Bond {bond:?}");
        let amb = sim.ambient_at(pos).expect("the drawn spot is inside a cel — the ramp has a target");
        eprintln!("char[{ci}] ramp sample {pos:?} amb {amb:?}");
        assert!(amb.iter().all(|&b| b < 0x40), "the sampled cel's ambient is the dark room (got {amb:?})");
    }
    assert_eq!(seen, 2, "the intro's two live script actors at frame 170");
}
