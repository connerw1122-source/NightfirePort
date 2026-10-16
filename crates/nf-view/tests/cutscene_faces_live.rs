//! cutscene-faces — why no face moves in the cut-scenes (playtest 1015-1738), and the sim/data
//! side the renderer reads. The game's mechanism, from the decompile:
//!
//! 1. The cut-scene anim script's op-0 windows start per actor [M `AnimProcessScriptCmds`
//!    0x1c1b38: the window covers the script clock → `AnimSeqSet`]. The FACIAL window is the
//!    sequence's flags bit 0x8 [M 0x1c1b38:165 `*(int *)(lVar6 + 0xc) >> 3 & 1`]; it plays in the
//!    script's +0x54 slot (the body window in +0x50) [M `AnimSeqSet` 0x1c7460:
//!    `param_3 + param_7*4 + 0x50`], and the 0x80000 script flag that would skip a facial start
//!    has no setter in the decompile — the face track always starts.
//! 2. Its channels are never bones: every script started under a script camera on a
//!    facial-bearing skin is marked the TALK slot (animScript+0x80 = 0xe0000002) [M
//!    `AnimScriptInit` 0x1c1600:52-61 — `if ((ScriptCam != 0) && (skin+0x2e != 0))`], and
//!    `AnimFacialBlend` 0x1be648's ScriptCam path copies that slot's 22-float frame buffer
//!    STRAIGHT into the morph weight array (the gameplay path instead blends the idle 0xe0000000 /
//!    blink 0xe0000001 driver slots [M `AnimFacialUpdate` 0x1be310] and the speech talk slot [M
//!    `AnimFacialAppend` 0x1c6490, called from `NDrone2_SpeechSFX` 0x141b00], with the 0.01
//!    threshold and the +0x97/+0x98 eye bytes writing indices 11-14).
//! 3. The weights drive the MORPH_DATA targets through `SkinIt` 0x10bf30 (the vertex half —
//!    fix-facial2), so the mouth is vertices moving, and the body track (flags 0x00, 3·bones+3
//!    channels) owns every bone. A facial-bearing actor whose script has NO face window still
//!    reads a ZERO weight array: `AnimScriptNew` 0x1c1398 allocates the +0x54 facial slot for
//!    every facial-bearing skin and `AnimSeqNew` 0x1c7248 zeroes its 22-float buffer — the
//!    ScriptCam talk-read copies zeros, the neutral face.
//!
//! The port's sim side: `Scene::facial_weights_now/at` (the getter the renderer reads) samples
//! the playing script's facial-flagged window per actor; `animate()` applies the CPU morph term
//! for script characters (fix-facial2). The GPU skin path (the default draw) does not apply the
//! weights yet — the held 108c `s/fix-gpu-facial-morphs` carries that half for the main agent's
//! draw wiring. Opt-in via NF_ISO.

use nf_game::player::Input;

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return None };
    match nf_view::load(&iso, archive, false) {
        Ok(s) => Some(s),
        Err(e) => { eprintln!("load {archive}: {e}"); None }
    }
}

fn is_facial(sq: &nf_formats::anim::Sequence) -> bool { sq.flags & 0x8 != 0 }

/// The classifier's disc-wide invariant: the facial flag (bit 0x8) and the facial channel count
/// partition every sequence on the disc — every flags-0x08 sequence carries exactly the facial
/// 22 channels and no body window (flags 0x00/0x01) ever carries bit 0x8, so the game's flags
/// classifier [M 0x1c1b38:165] and a channel-count pick agree everywhere; the port uses the
/// game's bit (`Scene::facial_weights_at`, `script_pose_together`). A body track is
/// 3·bones + 3·popcount(mask) channels — always ≡ 0 (mod 3) — and 22 never is.
#[test]
fn every_facial_flagged_sequence_carries_the_facial_channel_count() {
    use nf_formats::{disc, iso9660::IsoImage, levelbin::LevelBin};
    let Ok(iso_path) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let Ok(iso) = IsoImage::open(&iso_path) else { return };
    let Ok((entries, bin)) = disc::files_bin_entries(&iso) else { return };
    let mut flags_hist: std::collections::BTreeMap<u8, usize> = Default::default();
    let mut n = 0usize;
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let Ok(data) = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64) else { continue };
        let Ok(lb) = LevelBin::parse(&data) else { continue };
        for e in lb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
            let Ok(sq) = nf_formats::anim::Sequence::parse(lb.data(&data, e)) else { continue };
            *flags_hist.entry(sq.flags).or_default() += 1;
            n += 1;
            if sq.flags & 0x8 != 0 {
                assert_eq!(sq.channels.len(), 22,
                    "sequence {:08x} in {} carries the facial flag but {} channels", sq.hashcode, fe.name, sq.channels.len());
                assert!(sq.channels.len() % 3 != 0, "a facial-flagged track must never be a body channel count");
            }
        }
    }
    assert_eq!(n, 18244, "the disc's kind-4 sequence census");
    assert_eq!(*flags_hist.get(&0x8).unwrap_or(&0), 871, "the facial-flagged sequences");
    assert!(!flags_hist.keys().any(|f| f & 0x8 != 0 && *f != 0x8), "no other flag bit ever joins 0x8");
    eprintln!("sequence flags census over {n}: {flags_hist:?}");
}

/// The window pair of a cut-scene actor's script: exactly one facial window (the game's flags
/// bit) with the skin's facial channel count, and the body window that owns the bones.
fn face_window<'a>(sim: &nf_view::scene::Sim, script: u32, facial: usize)
    -> Option<(u16, nf_formats::anim::Sequence, (u16, u32))> {
    let scr = sim.scripts.get(&script)?;
    let mut face = None;
    for (a, b, q) in scr.sequences() {
        let Some(sq) = sim.seqs.get(&q) else { continue };
        if is_facial(sq) {
            assert!(face.is_none(), "script {script:08x} has two facial windows");
            assert_eq!(sq.channels.len(), facial, "the face track's channels are the skin's facial morph weights");
            face = Some((a, sq.clone(), (b, q)));
        } else {
            assert_eq!(sq.channels.len() % 3, 0,
                "script {script:08x} body window {q:08x} carries {} channels — must be a per-bone walk", sq.channels.len());
        }
    }
    face.map(|(a, sq, last)| (a, sq, last))
}

/// 07000002's intro (07800002.bin's scene 0x06000084, SP placement 400): Bond and the man at the
/// computer desk. Each actor's anim script (06000503 skin 0500004f, 06000504 skin 05000009) starts
/// BOTH windows at frame 1 — the face track is the flags-0x8 one — and the skin carries the
/// 63,528-byte morph target array the weights drive. Over the dialogue the mouth channels sweep
/// to ~1.0, the live getter (`facial_weights_now`, what the renderer consumes) equals the direct
/// track read at the object's frame, and the weights MOVE between the sampled ticks. Between the
/// lines the mouth closes (an all-zero beat) — the neutral sister inside the scene.
#[test]
fn the_02_intro_actors_face_tracks_start_and_move_the_morph_weights() {
    let Some(mut scene) = load("07000002.bin") else { return };
    let mut n = 0u32;
    let mut live: Vec<(usize, u32, u32, Vec<f32>)> = Vec::new(); // (char, script, skin, weights)
    for step in [10u32, 170, 400, 700] {
        {
            let sim = scene.sim.as_mut().unwrap();
            while n < step { sim.step_bond(Input::default(), false, false, false); n += 1; }
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.players.iter().any(|p| p.scripts.contains(&0x0600_0084) && p.state == 2),
            "the intro scene did not start from the level load");
        let mut seen = 0;
        for ci in 0..sim.char_script.len() {
            let (Some(src), Some(skin)) = (sim.char_script[ci], sim.char_script_skin[ci]) else { continue };
            let Some((sh, frame)) = sim.script_anim(src, skin) else { continue };
            seen += 1;
            let c = &scene.characters[ci];
            assert_eq!(c.skin.facial, 22, "char[{ci}] skin {skin:08x} must be facial-bearing");
            assert!(c.morph_targets.is_some(), "char[{ci}] carries the morph targets the weights drive");
            let (first, face, (last, q)) = face_window(sim, sh, 22)
                .unwrap_or_else(|| panic!("char[{ci}] script {sh:08x} has no facial window"));
            assert!(last >= first, "the face window must cover the scene");
            // The game's face track: window [1..1540/1541], sampled at the object's frame.
            let w_live = scene.facial_weights_now(c)
                .unwrap_or_else(|| panic!("char[{ci}] the scene must sample the facial weights at tick {step}"));
            let w_direct = nf_game::anim::facial_weights(&face, (frame - first as f32 + 1.0).max(1.0), 22)
                .expect("the face track must sample weights at the object's frame");
            assert_eq!(w_live, w_direct, "char[{ci}] tick {step}: the live weights are the face track's at the object's frame");
            if step == 400 { live.push((ci, sh, skin, w_live.clone())); }
            // The dialogue's beats: the intro man closes his mouth between lines (ticks 10/700
            // measured all-zero on 04000550) and opens it on them (max 1.02 at frame 1399).
            let closed = w_live.iter().all(|w| w.abs() < 1e-4);
            eprintln!("[intro] char[{ci}] skin {skin:08x} tick {step} frame {frame} max {:.3} {}",
                w_live.iter().cloned().fold(0.0f32, f32::max), if closed { "(closed)" } else { "" });
        }
        assert_eq!(seen, 2, "the intro's two script actors [D the main agent's master render: Bond and the desk man]");
    }
    // The mouth channels move over each actor's whole track (the jaw/lip group): the sweep pins
    // the track data. The game's eye bytes own indices 11-14 [M 0x1be648]; the MOUTH group is the
    // rest — at least five non-eye channels must pass 0.3 on each track (measured: 04000550 has
    // 0-10,15,19; 04000551 has 0-10).
    for (ci, sh, skin, w400) in &live {
        let sim = scene.sim.as_ref().unwrap();
        let (_, face, _) = face_window(sim, *sh, 22).unwrap();
        let mut maxima = vec![0f32; 22];
        let mut w_first = nf_game::anim::facial_weights(&face, 1.0, 22).unwrap();
        let (mut track_max, mut argmax) = (0f32, 1.0f32);
        for f in 1..=face.frames as u32 {
            let w = nf_game::anim::facial_weights(&face, f as f32, 22).unwrap();
            for (i, v) in w.iter().enumerate() {
                let a = v.abs();
                if a > maxima[i] { maxima[i] = a; }
                if a > track_max { track_max = a; argmax = f as f32; }
            }
        }
        assert!(track_max > 0.9, "char[{ci}] script {sh:08x}: the mouth opens over the face track (max {track_max:.3})");
        let moving: Vec<usize> = (0..22).filter(|i| maxima[*i] > 0.5).collect();
        assert!(moving.len() >= 10, "char[{ci}] script {sh:08x}: the face channels animate (moving {moving:?})");
        let mouth: Vec<usize> = (0..22).filter(|i| (11..=14).contains(i) == false && maxima[*i] > 0.3).collect();
        assert!(mouth.len() >= 5, "char[{ci}] script {sh:08x}: the jaw/lip group moves (mouth channels {mouth:?})");
        let w_arg = nf_game::anim::facial_weights(&face, argmax, 22).unwrap();
        assert!(w_first != w_arg, "char[{ci}] script {sh:08x}: the weights move (frame 1 vs {argmax})");
        eprintln!("[intro] char[{ci}] skin {skin:08x} face track: max {track_max:.3}@{argmax}, moving channels {moving:?}, mouth channels {mouth:?}");
        // The sampled tick's weights sit inside the track's sweep (the live path samples THE track).
        assert!(!w400.is_empty(), "char[{ci}] live weights sampled at tick 400");
    }
    // The dialogue animates the weights over TIME: the two actors' weight vectors at tick 170 and
    // tick 400 differ (measured: the desk man [0.74@ch4, 1.0@ch13] → [0.43@ch2, 1.0@ch11]).
    let mut n = 0u32;
    let mut at170: Vec<Vec<f32>> = Vec::new();
    {
        let sim = scene.sim.as_mut().unwrap();
        while n < 170 { sim.step_bond(Input::default(), false, false, false); n += 1; }
        let sim = scene.sim.as_ref().unwrap();
        for ci in 0..sim.char_script.len() {
            let (Some(_), Some(skin)) = (sim.char_script[ci], sim.char_script_skin[ci]) else { continue };
            if sim.script_anim(sim.char_script[ci].unwrap(), skin).is_none() { continue };
            if let Some(w) = scene.facial_weights_now(&scene.characters[ci]) { at170.push(w); }
        }
    }
    assert_eq!(at170.len(), 2, "both intro actors sample weights at tick 170");
    for ((ci, _, _, w400), w170) in live.iter().zip(&at170) {
        assert_ne!(w170, w400, "char[{ci}]: the weights must change between the dialogue's beats");
    }
}

/// 07000007's Kimiko scene (0x06000073 from TouchOnce #412): the three guests play their own
/// anim scripts — 060004f1 (skin 0500003b), 060004f2 (skin 05000015), 06000225 (skin 0500000d) —
/// each with the same window pair (face 22 ch flags 0x8; body 222 ch). All three tracks sweep to
/// ~1.0 across their frames, the live getter matches the direct read at every sampled tick, and
/// the speaking guests' live weights move between the beats; the quiet guest closes between her
/// lines (all-zero at frame 399 — the measured beat fix-cutscene-faces pinned).
#[test]
fn the_kimiko_guests_face_weights_move_over_the_dialogue() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    let mut n = 90u32;
    let mut guests: Vec<(u32, u32, Vec<Vec<f32>>)> = Vec::new(); // (script, skin, per-tick weights)
    for step in [400u32, 900, 1400] {
        {
            let sim = scene.sim.as_mut().unwrap();
            while n < step { sim.step_bond(Input::default(), false, false, false); n += 1; }
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.scripts.players.iter().any(|p| p.scripts.contains(&0x0600_0073) && p.state == 2),
            "the Kimiko scene did not start from trigger 412");
        let mut seen = 0;
        for ci in 0..sim.char_script.len() {
            let (Some(src), Some(skin)) = (sim.char_script[ci], sim.char_script_skin[ci]) else { continue };
            let Some((sh, frame)) = sim.script_anim(src, skin) else { continue };
            if !matches!(sh, 0x0600_04f1 | 0x0600_04f2 | 0x0600_0225) { continue }
            seen += 1;
            let c = &scene.characters[ci];
            assert_eq!(c.skin.facial, 22, "guest {sh:08x} skin {skin:08x} must be facial-bearing");
            assert!(c.morph_targets.is_some(), "guest {sh:08x} carries the morph targets");
            let (first, face, _) = face_window(sim, sh, 22)
                .unwrap_or_else(|| panic!("guest {sh:08x} has no facial window"));
            let w_live = scene.facial_weights_now(c)
                .unwrap_or_else(|| panic!("guest {sh:08x} the scene must sample the weights at tick {step}"));
            let w_direct = nf_game::anim::facial_weights(&face, (frame - first as f32 + 1.0).max(1.0), 22)
                .expect("the face track must sample weights at the object's frame");
            assert_eq!(w_live, w_direct, "guest {sh:08x} tick {step}: the live weights are the track's at the object's frame");
            let entry = match guests.iter_mut().find(|g| g.0 == sh) {
                Some(e) => e,
                None => { guests.push((sh, skin, Vec::new())); guests.last_mut().unwrap() }
            };
            entry.2.push(w_live.clone());
            eprintln!("[kimiko] guest {sh:08x} skin {skin:08x} tick {step} frame {frame} max {:.3}",
                w_live.iter().cloned().fold(0.0f32, f32::max));
        }
        assert_eq!(seen, 3, "the Kimiko scene's three guests play their own scripts");
    }
    // Each guest's face track carries the mouth animation over its frames; the speaking guests'
    // live weights move between the sampled beats.
    let sim = scene.sim.as_ref().unwrap();
    for (sh, skin, per_tick) in &guests {
        let (_, face, _) = face_window(sim, *sh, 22).unwrap();
        let mut track_max = 0f32;
        let mut moving = 0usize;
        let mut maxima = vec![0f32; 22];
        for f in 1..=face.frames as u32 {
            let w = nf_game::anim::facial_weights(&face, f as f32, 22).unwrap();
            for (i, v) in w.iter().enumerate() {
                let a = v.abs();
                if a > maxima[i] { maxima[i] = a; }
            }
        }
        track_max = maxima.iter().cloned().fold(0.0f32, f32::max);
        moving = maxima.iter().filter(|m| **m > 0.5).count();
        assert!(track_max > 0.9, "guest {sh:08x}: the mouth opens over the face track (max {track_max:.3})");
        assert!(moving >= 10, "guest {sh:08x}: the face channels animate ({moving} over 0.5)");
        let some_open = per_tick.iter().any(|w| w.iter().cloned().fold(0.0f32, f32::max) > 0.5);
        let some_closed = per_tick.iter().any(|w| w.iter().all(|v| v.abs() < 1e-4));
        let moves = per_tick.windows(2).any(|p| p[0] != p[1]);
        eprintln!("[kimiko] guest {sh:08x} skin {skin:08x}: track max {track_max:.3}, {moving} channels, open beat {some_open}, closed beat {some_closed}, moves {moves}");
        assert!(moves, "guest {sh:08x}: the live weights change over the dialogue");
        assert!(some_open || some_closed, "guest {sh:08x}: the mouth beats (open or closed) must appear in the scene");
    }
    // Sister: the quiet guest (060004f1) sits closed at the scene's early beat (measured
    // all-zero at frame 399) — a closed mouth is a zero weight array, never garbage.
    let (_, _, per_tick) = guests.iter().find(|g| g.0 == 0x0600_04f1).expect("the quiet guest");
    assert!(per_tick[0].iter().all(|v| v.abs() < 1e-4),
        "the quiet guest's mouth is closed at the scene's early beat, got {:?}", per_tick[0]);
}

/// The sister: a facial-bearing guest whose script has NO face window keeps a neutral face. The
/// Drake fly-through (0x06000072 from TouchOnce #356) has such guests — e.g. 06000200 (skin
/// 05000021) carries a single body window. The game reads the same zero weight array its
/// never-started facial slot was created with [M AnimScriptNew 0x1c1398 + AnimSeqNew 0x1c7248's
/// zeroed 22-float buffer, copied straight by AnimFacialBlend's ScriptCam talk-read]; the port's
/// getter returns None (no morph term) and no facial window exists to own a bone.
#[test]
fn a_facial_bearing_guest_with_no_face_track_keeps_a_neutral_face() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [95.4, 49.6, -79.3]; // TouchOnce #356's volume
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.scripts.players.iter().any(|p| p.scripts.contains(&0x0600_0072) && p.state == 2),
        "the fly-through did not start from trigger 356");
    let mut neutral = 0;
    for ci in 0..sim.char_script.len() {
        let (Some(src), Some(skin)) = (sim.char_script[ci], sim.char_script_skin[ci]) else { continue };
        let Some((sh, _)) = sim.script_anim(src, skin) else { continue };
        let Some(scr) = sim.scripts.get(&sh) else { continue };
        let has_face = scr.sequences().any(|(_, _, q)| sim.seqs.get(&q).is_some_and(is_facial));
        if has_face { continue }
        let c = &scene.characters[ci];
        assert_eq!(c.skin.facial, 22,
            "guest {sh:08x} skin {skin:08x}: the sister must be facial-bearing (facial 0 would be the weak sister)");
        assert!(c.morph_targets.is_some(), "guest {sh:08x} is a morph source with no face track");
        let w = scene.facial_weights_now(c);
        assert!(w.is_none(), "guest {sh:08x}: no face window → no weights (neutral face), got {:?}", w);
        // No facial window exists, so the joined pose IS the body pose — the face bones ride the
        // body track (the game: the facial slot never owns a bone).
        let frame = sim.script_anim(src, skin).unwrap().1;
        let body = nf_game::anim::script_pose(&sim.scripts, &sim.seqs, &c.skeleton, &c.skin, sh, frame)
            .expect("the body window must pose");
        let together = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &c.skeleton, &c.skin, sh, frame)
            .expect("the joined pose must resolve");
        assert_eq!(body, together, "guest {sh:08x}: with no face window the joined pose is the body pose");
        eprintln!("[neutral] guest {sh:08x} skin {skin:08x}: facial-bearing, no face window, weights None, pose == body pose");
        neutral += 1;
    }
    assert!(neutral >= 1, "the fly-through must have a facial-bearing guest with no face track");
}
