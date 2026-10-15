//! fix-ladder-water (109i): the ladder dismount into the pond — the infinite re-latch — through the
//! REAL viewer path (`nf_view::load` + `Sim::step_bond`). Opt-in via `NF_ISO`.
//!
//! User playtest (1014-1725, level 07000002, real-input build): "I'm also trying to climb off the
//! ladder and get back into the water, and there's a bug with that. I'm infinitely latching onto the
//! ladder, and I can't get off of it."
//!
//! Game evidence (ACTION.ELF):
//! - The dismount [M `Player_Climb` 0x196310:88-96 + the below-bottom branch] sets regrab
//!   (+0x951) = 60 and `ChangeSubState(0)`; the yaw stays facing the ladder (the climb's ease).
//! - The swim entry [M `Player_InWater` 0x195e50:24-42] OVERWRITES the regrab with 5 and zeroes the
//!   local axes; `Player_CollLadder` 0x196118 excludes only substates 1/2, so a swimmer re-grabs.
//! - The pick that feeds it is the substate's own HITTEST [M `Player_Collision` 0x194df0]: the swim
//!   arm (case 3) is the low short capsule (radius 0.55, segment pos−0.38..pos−0.58 at the stand
//!   root), and every sphere candidate is refined against the actual ladder mesh
//!   (`Collide_Update` 0x1ea398:84-93 → `Collide_Intersect` 0x1ebaa8) — a thin rung/rail mesh, so
//!   the swim reach ends ~at the capsule radius off the axis.
//! - Pre-fix the port used the walk pick's stage-2 column (`min(radius,1)+capsule` = 1.55 on the hut
//!   ladder) for swimmers too: the dismounted swimmer (0.5 off the axis, the grab snap) drifts only
//!   ~0.2 within the 5-tick regrab (the `Player_Move` 0.2/tick ramp [M 0x191dd0:108-117]) — 0.7 is
//!   still inside 1.55, the grab snaps him back to 0.5 and the climb's yaw ease fights his turn: a
//!   limit cycle (50 re-grabs in 360 ticks on master, probe 1014).

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load() -> Sim {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// One step through the real path; the level's scripts grab Bond into cut-scenes within seconds,
/// so each tick carries the game's own skip action like the walkreal walker [M nis.rs].
fn step(sim: &mut Sim, input: nf_game::player::Input) {
    if sim.disabled { sim.drones.scripts.skip_request = true; }
    sim.step_bond(input, false, false, false);
}

fn hut_ladder(sim: &Sim) -> (usize, nf_game::ladder::Ladder) {
    let li = (0..sim.ladders.len()).min_by(|a, b| {
        let d = |l: &nf_game::ladder::Ladder| (l.centre[0] - -74.84).powi(2) + (l.centre[2] - 22.22).powi(2);
        d(&sim.ladders[*a]).total_cmp(&d(&sim.ladders[*b]))
    }).expect("07000002 has ladders");
    (li, sim.ladders[li].clone())
}

/// The user's get-off: climb the hut ladder out of the pond, action-dismount back into the water,
/// keep the stick pulled back (the natural "push off the ladder" hold). The dismount leaves Bond
/// facing the ladder, so the back-stick swims him AWAY from it — with the mesh-scale swim reach the
/// 5-tick regrab drift (≈0.2 [M 0x191dd0 ramp]) clears the grab and he swims free; with the walk
/// column (1.55) the expired regrab re-latched him every cycle (the infinite latch).
#[test]
fn dismount_into_the_pond_lets_go_of_the_ladder() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    let (li, lad) = hut_ladder(&mut sim);
    assert!(lad.bottom() < 3.5, "the picked ladder does not reach into the pond: bottom {}", lad.bottom());
    // Onto the ladder from the water: spawn mid-column (root 3.0 — inside the game's grabbable swim
    // column, root >= ~0.56 for this ladder [M 0x194df0 case 3: the low capsule reaches centre-dist
    // 3.87]; the pond bed itself sits below it, where the game's swim pick also misses) and swim in.
    let front = [lad.centre[0] + lad.yaw.sin() * 1.2, 3.0, lad.centre[2] + lad.yaw.cos() * 1.2];
    sim.bond = nf_game::player::Bond::new(front, lad.yaw + std::f32::consts::PI);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = None;
    for t in 0..300 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = Some(t); break; }
    }
    grabbed.expect("swimming into the hut ladder never attached");
    // Up to mid-ladder (above the pond surface 3.765 [D probe]).
    for _ in 0..60 { step(&mut sim, walk); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_CLIMB, "the climb ended early");
    // The action dismount: the action button with the stick neutral [M 0x196310:45-48
    // `Input_Action(7)` + forward < 0.05].
    step(&mut sim, nf_game::player::Input { jump: true, ..Default::default() });
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the action dismount did not fire");
    let dxz0 = ((sim.bond.pos[0] - lad.centre[0]).powi(2) + (sim.bond.pos[2] - lad.centre[2]).powi(2)).sqrt();
    // Keep the stick pulled back (no jump): swimming backward from the dismount facing = away.
    let push_off = nf_game::player::Input { forward: -1.0, ..Default::default() };
    for t in 0..240 {
        step(&mut sim, push_off);
        assert_ne!(sim.bond.substate, nf_game::player::SS_CLIMB,
            "+{t}: re-latched onto the ladder after the dismount (the infinite latch, pos {:?} regrab {})",
            sim.bond.pos, sim.bond.regrab);
    }
    // The escape is real: the drift carried him past the mesh-scale reach.
    let dxz = ((sim.bond.pos[0] - lad.centre[0]).powi(2) + (sim.bond.pos[2] - lad.centre[2]).powi(2)).sqrt();
    assert!(dxz > dxz0 + 0.3, "the back-stick drift never moved him off the ladder ({dxz:.3} vs {dxz0:.3})");
    eprintln!("dismount into the pond: free swim, dxz {dxz0:.3} -> {dxz:.3}");
}

/// Sister pin: the swim-grab the fix must keep — swimming INTO the hut ladder from the pond still
/// attaches and climbs out (the 1013 fix-swim feature, `Player_CollLadder` 0x196118 excludes only
/// substates 1/2; the low swim capsule still reaches the ×1.5 sphere at the pond bed).
#[test]
fn swimming_into_the_ladder_still_grabs() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    let (li, lad) = hut_ladder(&mut sim);
    let front = [lad.centre[0] + lad.yaw.sin() * 1.2, 3.0, lad.centre[2] + lad.yaw.cos() * 1.2];
    sim.bond = nf_game::player::Bond::new(front, lad.yaw + std::f32::consts::PI);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = None;
    for t in 0..300 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = Some(t); break; }
    }
    let t = grabbed.expect("swimming into the hut ladder no longer attaches (the swim-grab regressed)");
    assert_eq!(sim.bond.ladder, Some(li), "the climb latched the wrong ladder");
    // Climb out: hold forward until the top dismount [M 0x196310:54-62 the +0.6 lunge].
    let mut landed = None;
    for _ in 0..600 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_WALK { landed = Some(sim.bond.pos[1]); break; }
    }
    let y = landed.expect("the climb never dismounted at the top");
    assert!(y > lad.top() - 0.6, "the top dismount did not reach the platform (y {y:.3} vs top {:.3})", lad.top());
    eprintln!("swim-grab pin: grabbed at +{t}, climbed out to y {y:.3}");
}

/// fix-ladder-water part 2 — the cut-scene skip must stop the playing voice:
/// user playtest 1014-1725: "Skip the cutscene in the middle of dialogue, and the dialogue still kept
/// going until it finished... just like the voice line itself." Level 07000004's intro runs the
/// engine cut-scene with live kind-5 voice streams (5 at once [probe]).
///
/// Game evidence (ACTION.ELF):
/// - The NIS voice is a kind-5 sound stream (`Script_SoundStart` 0x1d8820 sets the stream kind 5).
///   `Script_KillStream` 0x1d9860:96-99 case 5 calls `Sound_Stop(stream+0x10)`, and `Script_Free`
///   0x1da880:27 kills every stream when the script ends — the skip's fast-forward ends the script
///   mid-line, so the game's voice stops AT the skip. The port's `kill()` ignored kind 5.
/// - The dialogue text: `Script_FFwd` 0x1dac78:26-30 (the ScriptCam arm) calls
///   `Text_FlushAllSubtitles` 0x1b15c8 (the type-4/5 messages die) at the skip; the port kept the
///   shown line and the catch-up's remaining lines queued up in the HUD.
#[test]
fn skipping_the_cutscene_stops_the_dialogue_voice() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = {
        let mut scene = nf_view::load(&iso().expect("NF_ISO"), "07000004.bin", false).expect("load 07000004");
        scene.sim.take().expect("sim")
    };
    // Wait for the cut-scene's voices (no skip — let them start on the real path).
    let listener = ([0.0f32; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    let mut captured: Vec<((u32, u8), nf_game::sound::Handle)> = Vec::new();
    let mut seen: std::collections::HashSet<(u32, u8)> = Default::default();
    let mut removed = std::collections::HashSet::new();
    for _ in 0..1500 {
        step(&mut sim, Default::default());
        sim.drones.audio.update(None, listener, &|_| true);
        for c in sim.drones.audio.out.iter() {
            match c {
                nf_game::sound::Cmd::Start { handle, .. } => captured.retain(|(_, h)| h != handle),
                nf_game::sound::Cmd::Remove { handle } => { removed.insert(*handle); }
                _ => {}
            }
        }
        sim.drones.audio.out.clear();
        for (src, h, _) in sim.drones.script_sounds.iter() {
            if seen.insert(*src) { captured.push((*src, *h)); }
        }
        if captured.len() >= 3 { break; }
    }
    assert!(!captured.is_empty(), "no NIS voice started within 25 s — nothing to skip");
    // The skip through the real lane (`Scripts::skip_request`, what the viewer's key sets).
    sim.drones.scripts.skip_request = true;
    step(&mut sim, Default::default());
    // The fade (<=45 ticks [M 0x1dac78:48-51]) then the catch-up and the free: the voices die here.
    for _ in 0..150 {
        step(&mut sim, Default::default());
        sim.drones.audio.update(None, listener, &|_| true);
        for c in sim.drones.audio.out.iter() {
            if let nf_game::sound::Cmd::Remove { handle } = c { removed.insert(*handle); }
        }
        sim.drones.audio.out.clear();
    }
    let still: Vec<_> = captured.iter().filter(|(_, h)| !removed.contains(h)).collect();
    assert!(still.is_empty(), "NIS voices kept playing after the skip: {still:?}");
    eprintln!("skip stopped {} captured NIS sound(s)", captured.len());
}

/// Sister: a script that ends NORMALLY (no skip) also stops a still-playing sound — `Script_Free`
/// 0x1da880:27 kills every stream, case 5 → `Sound_Stop` [M 0x1d9860:96-99]. Pre-fix neither path
/// stopped a kind-5 stream (the sounds only died on their own `Script_SoundEnd` op 0x17).
#[test]
fn script_end_stops_a_still_playing_sound() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = {
        let mut scene = nf_view::load(&iso().expect("NF_ISO"), "07000004.bin", false).expect("load 07000004");
        scene.sim.take().expect("sim")
    };
    let listener = ([0.0f32; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    let mut captured: Vec<((u32, u8), nf_game::sound::Handle)> = Vec::new();
    let mut seen: std::collections::HashSet<(u32, u8)> = Default::default();
    let mut removed = std::collections::HashSet::new();
    for _ in 0..1500 {
        step(&mut sim, Default::default());
        sim.drones.audio.update(None, listener, &|_| true);
        for c in sim.drones.audio.out.iter() {
            match c {
                nf_game::sound::Cmd::Start { handle, .. } => captured.retain(|(_, h)| h != handle),
                nf_game::sound::Cmd::Remove { handle } => { removed.insert(*handle); }
                _ => {}
            }
        }
        sim.drones.audio.out.clear();
        for (src, h, _) in sim.drones.script_sounds.iter() {
            if seen.insert(*src) { captured.push((*src, *h)); }
        }
        if captured.len() >= 3 { break; }
    }
    assert!(!captured.is_empty(), "no NIS voice started within 25 s");
    // Let the script(s) run out naturally: every captured sound must eventually get its Remove
    // (its own end op or the script's free).
    for _ in 0..2400 {
        step(&mut sim, Default::default());
        sim.drones.audio.update(None, listener, &|_| true);
        for c in sim.drones.audio.out.iter() {
            if let nf_game::sound::Cmd::Remove { handle } = c { removed.insert(*handle); }
        }
        sim.drones.audio.out.clear();
        if captured.iter().all(|(_, h)| removed.contains(h)) { break; }
    }
    let missing: Vec<_> = captured.iter().filter(|(_, h)| !removed.contains(h)).collect();
    assert!(missing.is_empty(), "NIS sounds outlived their scripts: {missing:?}");
    eprintln!("natural ends stopped {} NIS sound(s)", captured.len());
}

