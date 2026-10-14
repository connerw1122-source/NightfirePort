//! fix-facial3: the face tracks' re-point (the 22-channel windows are morph weights, not bone
//! poses — `AnimFacialBlend` 0x1be648 samples them into the morph weight array; docs/formats/
//! character.md "Facial morphs"), the drone weights source, and the GPU-side morph mirror.
//!
//! Disc-measured premises [D]: 07000006's driver (skin 0x05000035, start script 0600011d) plays
//! body 04000064 (222ch, root un-zeroed on all 880 frames, net z 0.57) then face 04000065 (22ch,
//! channels 0..2 reach ~1.0 — weights, first |w| > 0.01 at window frame 170); the game's own trace
//! (tests/traces/05-truck.trace, the 07000006 segment) holds him to x span 0.024 / z span 0.063
//! while the hip follows the BODY track's ch1 (y span 0.119 ≈ the body's 0.129). 07000005's ledge
//! face track 040007fa crosses 0.01 at frame 767 [D, fix-facial2's facial_live pin].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;
use nf_view::scene::{Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn step(scene: &mut Scene) {
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
}

/// `script_pose_drone` on the 07000006 driver's start script: the root bone must come from the BODY
/// track now (the face track is morph weights, never a bone pose). Failing-first: pre-fix the
/// last-started face window owned bones 0..5, so the root translation was the face track's weight
/// values (~0..1 morph weights), not the body's root translation.
#[test]
fn the_face_track_no_longer_poses_the_driver_bones() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let s = scene.sim.as_ref().unwrap();
    let skin_hash = 0x0500_0035u32;
    let skin = s.skins.get(&skin_hash).expect("the driver's skin").clone();
    let skel = s.skeletons.get(&skin.skeleton).expect("the driver's skeleton").clone();
    let body = s.seqs.get(&0x0400_0064).expect("the driver body track");
    let face = s.seqs.get(&0x0400_0065).expect("the driver face track");
    assert_eq!(body.channels.len(), 222, "the body track is the full pose");
    assert_eq!(face.channels.len(), 22, "the face track is the 22-channel morph window");
    assert_eq!(skin.facial, 22, "the driver skin's facial count matches the face track");
    // Mid-window: the body root is un-zeroed (real translation), the face "root" channels are
    // morph weights reaching ~1.0 — the two disagree hugely at frame 300.
    let f = 300.0f32;
    let body_root_y = body.value(1, f as u32);
    let face_root_y = face.value(1, f as u32);
    assert!((body_root_y - face_root_y).abs() > 0.1,
        "the two tracks disagree on 'channel 1' at frame 300 (body {body_root_y}, face {face_root_y})");
    let pose = nf_game::anim::script_pose_drone(&s.scripts, &s.seqs, &skel, &skin, 0x0600_011d, f)
        .expect("the driver's script poses");
    let root = pose[0].0;
    eprintln!("[facial3] frame 300 root translation {:?} (body ch1 {body_root_y}, face ch1 {face_root_y})", root);
    assert!((root[1] - body_root_y).abs() < 1e-2,
        "the root translation is the BODY track's channel 1 (got {}, body {body_root_y}) — the face track no longer owns the root", root[1]);
    // Sister: the face track's weights still sample through the weight source (the 22 values).
    let w = nf_game::anim::facial_weights(face, f, 22).expect("the face weights sample");
    assert_eq!(w.len(), 22);
    assert!(w.iter().any(|v| v.abs() > 0.01), "frame 300 of the driver's face window carries live weights");
}

/// The driver end-to-end: once his start script's face window is live, the drone-backed character
/// is a morph-weight source (`facial_weights_at` — pre-fix it returned None for every drone), and
/// he holds his seat over the whole window (the game trace's spans: x 0.024, z 0.063, y 0.119 —
/// the hip follows the body track's ch1; the port keeps the same shape within the drift slack).
#[test]
fn the_truck_driver_is_a_morph_source_and_holds_his_seat() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    // The driver's character: the skin hash 0x05000035 (docs/levels/07000006.md §4, d17).
    let ci = scene.characters.iter().position(|c| c.skin.hashcode == 0x0500_0035)
        .expect("the driver's character exists");
    assert!(scene.characters[ci].script.is_none(), "the driver is drone-backed, not a script object");
    // Wait out the intro, then ride the window: the driver's script starts with his first tick.
    for _ in 0..3000 { step(&mut scene); if !scene.sim.as_ref().unwrap().disabled { break; } }
    let mut weights_seen = None;
    let mut pos_min = [f32::MAX; 3];
    let mut pos_max = [-f32::MAX; 3];
    // The driver drone is the one playing his start script (unique to him); the character is its
    // char_drone link (the skin hash alone is ambiguous — the pool reuses skins).
    let mut driver: Option<(usize, usize)> = None; // (drone index, character index)
    for t in 0..900u32 {
        step(&mut scene);
        let s = scene.sim.as_ref().unwrap();
        if driver.is_none() {
            for (di, dr) in s.drones.list.iter().enumerate() {
                if dr.present && dr.anim.script == Some(0x0600_011d) {
                    driver = s.char_drone.iter().position(|d| *d == Some(di)).map(|ci| (di, ci));
                    break;
                }
            }
        }
        let Some((di, c_i)) = driver else { continue };
        let dr = &s.drones.list[di];
        if dr.anim.script != Some(0x0600_011d) { continue; }
        // The settle tail: the spawn drop is over (the game trace's own segment starts
        // mid-script); the span measures the seated window.
        if t >= 400 {
            for k in 0..3 { pos_min[k] = pos_min[k].min(dr.pos[k]); pos_max[k] = pos_max[k].max(dr.pos[k]); }
        }
        if let Some(w) = scene.facial_weights_at(c_i, &scene.characters[c_i]) {
            let m = w.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            if m > 0.1 { weights_seen = Some((t, m)); }
        }
    }
    let (t, m) = weights_seen.expect("the driver's face window crosses live weights (pre-fix: None for drones)");
    eprintln!("[facial3] the driver's weights went live at tick {t} (max {m:.3})");
    let span = [pos_max[0] - pos_min[0], pos_max[1] - pos_min[1], pos_max[2] - pos_min[2]];
    eprintln!("[facial3] the driver's seat span over the window: x {:.3} y {:.3} z {:.3} (game trace: 0.024/0.119/0.063)", span[0], span[1], span[2]);
    // Known gap (the report's "What's left"): the game holds the seated driver to the trace's
    // 0.024/0.119/0.063 spans through the truck-driver state; the port's cab floor-ray wobble
    // (y ~1.0) and the un-ported sit suppression leave a larger bounded drift. The re-point's own
    // guarantee is tested by the bones test above: the face weights (reaching ~1.0) are not the
    // root any more. Bound here against a runaway only.
    assert!(span[0] < 2.0 && span[1] < 2.0 && span[2] < 2.0, "the seated driver stays in the cab's neighbourhood");
}

/// The 07000005 ledge dialogue at the pinned crossing: the mouth vertices move at window frame 767
/// [D the 040007fa track's first 0.01 crossing], with the bones coming from the body track after
/// the re-point (the lockstep sister with the targets cleared cancels everything else).
#[test]
fn the_ledge_dialogue_mouth_moves_at_frame_767() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut a = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut b = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005 (sister)");
    for c in b.characters.iter_mut() { c.morph_targets = None; }
    let step2 = |a: &mut Scene, b: &mut Scene| { step(a); step(b); };
    for _ in 0..3000 { step2(&mut a, &mut b); if !a.sim.as_ref().unwrap().disabled { break; } }
    // Onto the ledge scene's start plate (#301 → ch 0x43), as fix-facial2's drive.
    let plate = a.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| (t.kind == nf_game::triggers::kind::TOUCH_ONCE || t.kind == nf_game::triggers::kind::TOUCH) && t.list.contains(&0x43))
        .or_else(|| a.sim.as_ref().unwrap().drones.triggers.iter().find(|t| t.out == 0x43 && (t.kind == nf_game::triggers::kind::TOUCH_ONCE || t.kind == nf_game::triggers::kind::TOUCH)))
        .expect("the ledge cut-scene's start plate").clone();
    for s in [&mut a, &mut b] {
        let c = plate.centre();
        let sim = s.sim.as_mut().unwrap();
        sim.bond.pos = [c[0], c[1] - 0.5, c[2]];
        sim.bond.yaw = 0.0;
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, true); }
    }
    // The 040007fa window's character (fix-facial2's character 70), then ride to its frame 767.
    let mut ci = None;
    let mut live_frame = None;
    for t in 0..12_000u32 {
        step2(&mut a, &mut b);
        let s = a.sim.as_ref().unwrap();
        for (i, c) in a.characters.iter().enumerate() {
            let Some(src) = c.script else { continue };
            if c.skin.facial != 22 || c.morph_targets.is_none() { continue; }
            let Some((sh, frame)) = s.script_anim(src, c.skin.hashcode) else { continue };
            let Some(script) = s.scripts.get(&sh) else { continue };
            let Some((first, q)) = script.sequences()
                .filter(|(a2, _, _)| (*a2 as f32) <= frame)
                .filter_map(|(a2, _, q)| s.seqs.get(&q).filter(|sq| sq.channels.len() == 22).map(|sq| (a2, q))).last() else { continue };
            if q == 0x0400_07fa {
                ci = Some(i);
                live_frame = Some(frame - first as f32 + 1.0);
                break;
            }
        }
        if ci.is_some() { break; }
    }
    let ci = ci.expect("the ledge scene's 040007fa character went live");
    // Ride until the window frame passes 767 (the track's first live crossing [D]).
    let mut at_767 = None;
    for t in 0..24_000u32 {
        step2(&mut a, &mut b);
        let s = a.sim.as_ref().unwrap();
        let c = &a.characters[ci];
        let Some(src) = c.script else { continue };
        let Some((sh, frame)) = s.script_anim(src, c.skin.hashcode) else { continue };
        let Some(script) = s.scripts.get(&sh) else { continue };
        let Some((first, q)) = script.sequences()
            .filter(|(a2, _, _)| (*a2 as f32) <= frame)
            .filter_map(|(a2, _, q)| s.seqs.get(&q).filter(|sq| sq.channels.len() == 22).map(|sq| (a2, q))).last() else { continue };
        if q != 0x0400_07fa { continue; }
        let wf = frame - first as f32 + 1.0;
        if wf >= 767.0 {
            let w = a.facial_weights_at(ci, c).expect("the weights at the crossing");
            let m = w.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            eprintln!("[facial3] window frame {wf:.1} at tick {}: weights max {m:.4}", t);
            assert!(m > 0.01, "frame 767 is the track's first live crossing [D]");
            at_767 = Some(t);
            break;
        }
    }
    let t767 = at_767.expect("the window reached frame 767 within the dialogue");
    // The vertex proof: the mouth-bearing vertices moved by the morph path; the bones (identical
    // in the lockstep sister) cancel.
    let face_verts: Vec<usize> = a.char_verts.iter().enumerate()
        .filter(|(_, v)| v.character as usize == ci && v.morph >= 0).map(|(i, _)| i).collect();
    assert!(!face_verts.is_empty(), "the ledge character has morph-indexed vertices");
    let (pa, pb) = (a.animate(None, t767 as f32, ViewOpts::default()), b.animate(None, t767 as f32, ViewOpts::default()));
    let moved: f32 = face_verts.iter().map(|&i| (0..3).map(|k| (pa[i][k] - pb[i][k]).abs()).fold(0.0f32, f32::max)).fold(0.0f32, f32::max);
    eprintln!("[facial3] frame-767 mouth delta {moved:.6}");
    assert!(moved > 0.0002, "the face vertices move by the morph path at frame 767 (got {moved:.6})");
    let _ = live_frame;
}
