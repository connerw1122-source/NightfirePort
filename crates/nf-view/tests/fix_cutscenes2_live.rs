//! fix-cutscenes2 — the cut-scenes play (fix-cutscenes); make them right. The user's playtests
//! (2026-10-11/12, party07b): movement keys skipped the Kimiko scene (and its objective banner+sound
//! came through); the Drake fly-through drew the live party AND the script's guests; the mouths never
//! moved (the face tracks were not played); no voice lines fired.

use nf_game::nis::Request;
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

/// The skip action is the game's `Input_Action(-1, 0x19, 1)` [M SP_Update 0x1dbda8:129-141], bound to
/// raw pad mask 0x840 — R1 (0x800) or D-pad DOWN (0x040), pad 0 only [M psiInput_MapInputs
/// 0x10cd80:415-417]. Movement keys never skip; the user's W press skipped the 07000007 Kimiko scene
/// (2026-10-12 playtest). The port maps the action to ArrowDown and R; the viewer's dedicated keys
/// keep their own functions.
#[test]
fn only_the_game_skip_action_keys_skip_a_cutscene() {
    use nf_view::cutscene_skip_key;
    use winit::keyboard::KeyCode as K;
    assert!(cutscene_skip_key(K::ArrowDown), "D-pad down (pad 0x040) is the game's skip action");
    assert!(cutscene_skip_key(K::KeyR), "R (the R1 stand-in, pad 0x800) is the game's skip action");
    for k in [K::KeyW, K::KeyA, K::KeyS, K::KeyD, K::Space, K::KeyQ, K::KeyG, K::KeyE, K::KeyF, K::KeyT, K::KeyC, K::KeyN, K::KeyP, K::Escape, K::F1, K::F2, K::F11, K::F12, K::Enter, K::ArrowUp, K::ArrowLeft, K::ArrowRight] {
        assert!(!cutscene_skip_key(k), "{k:?} must not skip a cut-scene (movement and the viewer's own keys never skip)");
    }
}

/// 07000007: the Drake fly-through (0x06000072, SP #419) fires event 5 at tick 0 —
/// `Drone_EnableAll(0, hash)` [M Script_EventHandler 0x1d94c0 case 5 → 0x1383e8; `NDrone2_Enable(0)`
/// 0x149dd0 sets obj+0xF0 |= 0x10|0x20: not drawn, not picked] — and event 7 near its end (flag 0x80,
/// `Script_Free` 0x1da880 re-enables through `NDrone2_Enable(1)`, which restarts each anim at script
/// time 1). Before: the port drew the live party AND the script's guests (the user's duplicated
/// models, frame 1790).
#[test]
fn the_drake_flythrough_hides_the_live_party_and_restores_it() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [95.4, 49.6, -79.3]; // TouchOnce #356's volume (out 0x15)
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0072), "the fly-through did not start from trigger 356");
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.npc_enabled, "event 5 must turn the NPC globals off");
    let live: Vec<usize> = sim.drones.list.iter().enumerate().filter(|(_, d)| d.present).map(|(i, _)| i).collect();
    assert!(!live.is_empty(), "the party has live drones");
    let shown: Vec<usize> = live.iter().copied().filter(|&i| sim.drones.list[i].enabled).collect();
    assert!(shown.is_empty(), "the live party must be hidden during the scene (Drone_EnableAll(0), obj+0xF0 |= 0x10): still shown {shown:?}");
    let anims = sim.drones.scripts.objects.list.iter().flatten().filter(|o| matches!(o, nf_game::nis::Obj::Anim { .. })).count();
    assert!(anims >= 13, "the fly-through's 13 guest Anims must exist, got {anims}");

    // The game's own skip (the R1/D-down action stand-in) ends the scene; the 0x80 flag re-enables
    // the party, each anim restarted at script time 1 [M NDrone2_Enable(1) 0x149dd0:16-28].
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.scripts.skip_request = true;
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!playing(&scene, 0x0600_0072), "the game's skip action must end the fly-through");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.npc_enabled, "Script_Free's 0x80 flag re-enables the drones");
    let shown: Vec<usize> = sim.drones.list.iter().enumerate().filter(|(_, d)| d.present && d.enabled).map(|(i, _)| i).collect();
    assert!(!shown.is_empty(), "the re-enabled party must be shown again");
    let back: Vec<f32> = shown.iter().map(|&i| sim.drones.list[i].anim.time as f32).collect();
    assert!(back.iter().all(|t| *t <= 2.0), "re-shown drones replay from script time 1, got {back:?}");
}

/// 07000007: Bond walks into TouchOnce #412 (out 0x48) → SP #420 plays 0x06000073 (the Kimiko scene,
/// 4326 ticks). Movement input must NOT skip it (the game's skip is the 0x19 action only), and the
/// objective banner hold (`hud_frozen`, set by the scene's camera request at tick 0, cleared at
/// `Script_Free` [M Script_CameraStart 0x1d9140:46-49; the banner SFX hold reads ScriptCam,
/// HUD_UpdateStatusPane 0x19c310:219-235]) must last the whole scene — the user's "obj doesnt wait
/// for kimiko cutscene and plays the sound immediately" was the W-skip.
#[test]
fn movement_input_leaves_the_kimiko_scene_playing_and_the_objective_held() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start from trigger 412");
    let sim = scene.sim.as_mut().unwrap();
    // Walk into the scene (the W press that skipped it in play): 150 ticks of forward movement.
    for _ in 0..150 { sim.step_bond(Input { forward: 1.0, ..Input::default() }, false, false, false); }
    assert!(playing(&scene, 0x0600_0073), "movement input skipped the Kimiko scene — only the game's 0x19 action may skip");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.hud_frozen, "the scene's camera must hold the objective banner (and its sound) while it plays");
}

/// The Kimiko scene's voice lines: stream 10 fires `Sound { id: 0x11d }` (3-D at stream 1, Kimiko) at
/// script tick 351, then 0x5c0/0x5c1 [D 07800007.bin 06000073 stream 10/12/13; the op-0x16 id is a
/// sound-bank effect id — Script_SoundStart 0x1d8820 → Sound_Play/Play3D 0x1dd880/0x1dd548 — and a
/// voice is a bank sub-track with FileRef < 0 → STREAMS.LUT/BIN (SFXInitialiseStream 0x9030)]. The
/// port must raise the same requests and resolve them against the level bank and the streams.
#[test]
fn the_kimiko_scene_fires_its_voice_lines() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start");
    // Stream 10's first line waits 0x351 = 849 ticks (the nf-extract dump prints hex), the second
    // pair waits 0x30a/0x10b — run past all of them. Flush the sound pool per tick (the viewer's
    // frame pass) and keep every `Start`: a line whose SoundEnd already fired would otherwise be
    // flushed as a Remove.
    let mut ids: Vec<u32> = Vec::new();
    for _ in 0..1500 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.audio.update(None, ([0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| false);
        for c in sim.drones.audio.take_commands() {
            if let nf_game::sound::Cmd::Start { id, .. } = c { ids.push(id); }
        }
    }
    assert!(ids.contains(&0x11d), "the Kimiko scene's first voice line (0x11d at tick 849) never fired; sounds {ids:?}");
    assert!(ids.contains(&0x5c0) || ids.contains(&0x5c1), "the scene's second/third lines (0x5c0/0x5c1) never fired; sounds {ids:?}");
    let (_, bank, _) = scene.sound_bank.as_ref().expect("the 07000007 sound bank must load");
    for id in [0x11d, 0x5c0, 0x5c1] {
        let e = bank.effect(id).unwrap_or_else(|| panic!("bank effect {id:#x} missing — the voice would be silence"));
        // The sub-tracks start at 0x44 (Effect::parse): every line must resolve to playable audio —
        // a stream (FileRef < 0 → STREAMS.LUT) or in-bank samples.
        let fx = nf_formats::soundbank::Effect::parse(e).expect("the effect block parses");
        assert!(fx.subs.iter().any(|s| s.file != 0), "voice {id:#x} must have sample sub-tracks, got {:?}", fx.subs);
        eprintln!("[voice {id:#x}] files {:?} pause_in_nis {}", fx.subs.iter().map(|s| s.file).collect::<Vec<_>>(), fx.pause_in_nis);
    }
    let (lut, bin) = scene.streams.as_ref().expect("STREAMS.LUT/BIN must load");
    assert!(lut.len() > 0x400, "the streams lut looks empty");
    assert!(!bin.is_empty(), "STREAMS.BIN must be loaded for the voices to decode");
}

/// The Kimiko scene's characters play BOTH their tracks, the game's way: each anim script
/// (060004f1/060004f2) carries two op-0 windows from frame 1 — the 22-channel face track (first) and
/// the 222-channel body track (last) [D `nf-extract script-info 07000007.bin 060004f1`]. The game
/// classifies the face window as FACIAL by the sequence's flags bit 0x8 [M AnimProcessScriptCmds
/// 0x1c1b38: `*(int *)(lVar6 + 0xc) >> 3 & 1` → the script node's 0x40000 flag, the +0x54 facial
/// slot] and its channels are the skin's facial MORPH weights (`AnimFacialBlend` 0x1be648 reads the
/// playing sequence's frame buffer as the weight array; the counts match by construction — [D
/// measured] skin 0x0500003b facial 22 == face seq 0x04000467's 22 channels, values in [0, 1]), so
/// the face track moves the MOUTH through the morph targets (the character carries them) and never
/// the bones: the body track (222 = 3·73 + 3·1 channels, flags 0x00) owns every bone and its root
/// channels hold the actor's world position. The pre-fix reads got this wrong in both directions —
/// first-writer posed bones 0..5 from the face track's weights AND collapsed the root to the origin
/// (the missing cast); widest-wins restored the cast but left the bone-assertion of this test
/// failing because a facial window must not move bones at all.
#[test]
fn the_kimiko_scene_characters_play_their_face_tracks() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
        for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    // The Kimiko scene's guest: its Obj::Anim (the anim script 060004f1) names the skin; the
    // Character with that skin and the object's (script, stream) source carries the skeleton.
    let (skin, src, frame) = sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
        nf_game::nis::Obj::Anim { script, skin, frame, src, .. } if *script == 0x0600_04f1 => Some((*skin, *src, *frame)), _ => None })
        .expect("the anim object must be live once the scene runs");
    let char = scene.characters.iter().find(|c| c.skin.hashcode == skin && c.script == Some(src))
        .expect("the Kimiko scene's guest character must exist in the scene build");
    let script = 0x0600_04f1_u32;
    // The face window is the FIRST op-0 window's sequence, and it carries the game's facial flag.
    let fq = sim.scripts.get(&script).unwrap().sequences().next().unwrap().2;
    let face_seq = &sim.seqs[&fq];
    assert!(face_seq.flags & 0x8 != 0, "the face track must carry the game's facial flag, got flags {:#04x}", face_seq.flags);
    assert_eq!(face_seq.channels.len(), char.skin.facial as usize,
        "the face track's channels are the skin's facial morph weights ({} vs {})", face_seq.channels.len(), char.skin.facial);
    assert!(char.morph_targets.is_some(), "the guest's skin carries the facial morph targets the weights drive");
    // Sister pair 1 — the mouth animation lives in the MORPH WEIGHTS: across the face track's own
    // frames they open wide (the measured max is 1.0 — [D the probe sweep: 312/433 sampled frames
    // carry weight > 0.01, max 1.001]) and they MOVE (the vector at the widest frame differs from
    // the closed frame 1). At any single frame the mouth may legitimately be closed (frame 399 is).
    let count = char.skin.facial as usize;
    let mut max_w = 0f32;
    let mut max_frame = 1.0f32;
    let mut f = 1.0f32;
    while f <= face_seq.frames as f32 {
        let w = nf_game::anim::facial_weights(face_seq, f, count).expect("the face track must sample weights");
        let m = w.iter().cloned().fold(0.0f32, f32::max);
        if m > max_w { max_w = m; max_frame = f; }
        f += 10.0;
    }
    assert!(max_w > 0.5, "the mouth opens over the face track's frames (max weight {max_w:.3})");
    let w_open = nf_game::anim::facial_weights(face_seq, max_frame, count).unwrap();
    let w_closed = nf_game::anim::facial_weights(face_seq, 1.0, count).unwrap();
    assert!(w_open != w_closed, "the weights move over the face track's frames — the mouth animates");
    // The live scene path (what the renderer consumes) samples the same window at the object's
    // frame — equal to the direct track read at that frame (the same weights, not a second source).
    let w_live = scene.facial_weights_now(char).expect("the scene must sample the guest's facial weights right now");
    let w_at_frame = nf_game::anim::facial_weights(face_seq, frame.max(1.0), count)
        .expect("the face track must sample weights at the object's frame");
    assert_eq!(w_live, w_at_frame, "the scene's live weights are the face track's weights at the object's frame");
    // Sister pair 2 — the facial window never touches the BONE pose: the joined pose equals the
    // body-only pose on every bone, and the root stands at the body track's library position
    // (its channels read 83..91, 43.7, −35..−31 [D]) — the cast at the scripted world position,
    // not the origin (the pre-cutscene-draw bug).
    let body_only = nf_game::anim::script_pose(&sim.scripts, &sim.seqs, &char.skeleton, &char.skin, script, frame)
        .expect("the body track must pose (the old path)");
    let together = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &char.skeleton, &char.skin, script, frame)
        .expect("the joined pose must resolve");
    assert_eq!(body_only.len(), together.len(), "both poses cover the skeleton");
    // fix-cutscene-faces2: the classifier is the game's own flags bit 0x8 (asserted above), not the
    // channel count — the count matches by construction (asserted above too). The facial window
    // must not move bones: the joined pose is the body pose on every bone (fix-facial3's per-bone
    // loop, kept as the equality — same assertion), and the root stands at the scripted position.
    assert_eq!(body_only, together, "the facial window must not move bones — the body track owns the pose");
    let root = together[0].0;
    let root_len = (root[0] * root[0] + root[1] * root[1] + root[2] * root[2]).sqrt();
    assert!(root_len > 10.0, "the cast stands at the scripted world position (root {root:?}), not the origin");
}
