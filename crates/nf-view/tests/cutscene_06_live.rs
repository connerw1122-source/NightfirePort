//! drake-cutscene-06: the 07000006 dressing cut-scene (0x06000071, ScriptPlayer #810) — the
//! playtest 1015-2142 "drake cutscene on o6 has some lag and glitchiness with objects".
//!
//! Part 2 (the object glitches), headless: the cut-scene runs to completion through the real
//! path (the LoadPoint trigger #746 → ch 0x83 → SP #810), every matrix the scene drives is
//! tracked per tick (the script camera, the two `Script_AnimStart` actors, the SP's live
//! matrix), and every discontinuity is flagged and matched against the disc's own script data:
//!
//! - the camera jumps exactly where the stream-0 key times cut [D 0x06000071 stream 0:
//!   WAIT/MARKER pairs at 150/151, 549/550, 874/875 — three designed hard cuts];
//! - the two actors' world roots jump at the frames where their animation sequences' ROOT
//!   channels themselves jump [D seq 04000174 (suit 0x05000007) f550→551 9.9 m, seq 04000173
//!   (tux 0x0500000d) f550→551 5.8 m and f875→876 2.6 m] — the game's own teleport-under-the-cut
//!   staging, each landing on a camera cut tick (the game's `Script_GetInterp` 0x1da158 /
//!   `AnimFrameResolve` root walk reproduces it; the port is faithful).
//!
//! So the test pins: (a) the scene completes headless (the closing event 0x12 loads 07000007
//! [M Script_EventHandler 0x1d94c0 case 0x12], the objective's ch 0x02 completes [L record 1]);
//! (b) every tracked matrix is continuous except at exactly those game-data discontinuities —
//! a NEW jump (a sim-side wrong key interp, a missed delete, an entity drawn before its create)
//! fails here; (c) the actor jumps coincide with camera cuts (within a tick), as on the disc.
//!
//! Opt-in via `NF_ISO` (the slim ISO is enough).

use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    // A fresh Bond with only the carried gates (walk_07000006_live.rs's recipe).
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// One tracked point per tick.
#[derive(Clone, Copy, Default)]
struct Track { t: u32, p: [f32; 3] }

/// The discontinuities of a trajectory: ticks where the step exceeds 0.6 m and 4× the previous
/// step (the task's "a discontinuity larger than the motion around it").
fn flags(v: &[Track]) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 1..v.len() {
        let d = (v[i].p[0] - v[i - 1].p[0]).powi(2) + (v[i].p[1] - v[i - 1].p[1]).powi(2) + (v[i].p[2] - v[i - 1].p[2]).powi(2);
        let d = d.sqrt();
        let prev = if i >= 2 {
            ((v[i - 1].p[0] - v[i - 2].p[0]).powi(2) + (v[i - 1].p[1] - v[i - 2].p[1]).powi(2) + (v[i - 1].p[2] - v[i - 2].p[2]).powi(2)).sqrt()
        } else { 0.0 };
        if d > 0.6 && d > 4.0 * prev.max(1e-3) { out.push(i); }
    }
    out
}

#[test]
fn the_dressing_scene_runs_headless_and_every_matrix_is_continuous_except_the_game_cuts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006");
    {
        let sim = scene.sim.as_mut().unwrap();
        // The truck-ride spawn (#496, gate 0x1D) — the playtest route.
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the level-start intro");
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 0..60 { sim.step_bond(input, false, false, false); }
    }
    let lp = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.out == 0x83).expect("the LoadPoint trigger (out 0x83) exists").clone();
    let c = lp.centre();
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [c[0], c[1] + 1.0, c[2]];
        for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x83) { break; } }
    }
    // The scene clock: from the first tick SP #810 is playing.
    let started = {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.scripts.players.iter().any(|p| p.scripts.contains(&0x0600_0071) && p.state == 2)
    };
    assert!(started, "the dressing cut-scene (SP #810, 0x06000071) did not start from the LoadPoint");

    let mut cam: Vec<Track> = Vec::new();
    let mut suit: Vec<Track> = Vec::new();
    let mut tux: Vec<Track> = Vec::new();
    let mut exit_tick = None;
    for t in 0..1300u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.level_exit.is_some() { exit_tick = Some(t); }
        if let Some((p, _, _, _)) = sim.script_camera() { cam.push(Track { t, p }); }
        // The two actors' world roots — the same pose the draw path uses (Scene::frame_state's
        // script branch: script_pose_together → world_pose, the root bone carried in world space).
        for ch in scene.characters.iter().filter(|c| c.script.is_some_and(|(h, _)| h == 0x0600_0071)) {
            let Some((script, frame)) = sim.script_anim((0x0600_0071, ch.script.unwrap().1), ch.skin.hashcode) else { continue };
            let Some(pose) = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &ch.skeleton, &ch.skin, script, frame) else { continue };
            let world = nf_formats::anim::world_pose(&pose, &ch.skin);
            let tr = Track { t, p: [world[0][3][0], world[0][3][1], world[0][3][2]] };
            match ch.skin.hashcode { 0x0500_0007 => suit.push(tr), 0x0500_000d => tux.push(tr), _ => {} }
        }
        if exit_tick.is_some() { break; }
    }
    // (a) The scene completes headless: the exit to the party + the objective's channel.
    // The mission monitor runs earlier in the tick than the scripts: give it the ticks after
    // the exit (walk_07000006_live.rs's note).
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0007), "the dressing scene's closing event 0x12 must load 07000007 [M 0x1d94c0 case 0x12]");
    let i6 = nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0006).unwrap();
    assert_eq!(sim.drones.mission.states[i6][0], 4, "'Find a way inside to the party' completes on ch 0x02 [L ee_start_05 record 1]");
    let exit_tick = exit_tick.expect("the scene ended");
    eprintln!("[dress] scene ran {exit_tick} ticks to the level exit");
    assert!((1000..1200).contains(&exit_tick), "the scene's length must be the script's 1129 ticks [D], got {exit_tick}");

    // (b) The camera's discontinuities are exactly the three designed hard cuts [D stream 0's
    // WAIT/MARKER pairs at 150/151, 549/550, 874/875]. The scene clock starts 1 tick before our
    // t=0 loop (the SP went state 2 during the trigger wait), so the observed ticks shift by 1-2.
    let cam_cuts = flags(&cam);
    eprintln!("[dress] camera discontinuities at loop ticks {cam_cuts:?} (of {} samples)", cam.len());
    assert_eq!(cam_cuts.len(), 3, "the camera must cut exactly 3 times — the game's own key-time cuts; extra cuts are sim-side bugs");
    for (i, cut) in cam_cuts.iter().enumerate() {
        let expect = [150u32, 549, 874][i] as i64;
        let got = cam[*cut].t as i64 - cam[0].t as i64;
        assert!((got - expect).abs() <= 2, "camera cut {i} at scene tick {got}, expected {expect}±2 [D stream 0 WAIT/MARKER]");
    }

    // (c) The actors' world roots are continuous except at the frames where their sequences'
    // ROOT CHANNELS themselves jump [D] — and each such jump lands on a camera cut (within one
    // tick of it), the game's teleport-under-the-cut staging.
    let seq_roots = |sim: &nf_view::scene::Sim, seq: u32, f: u32| -> [f32; 3] {
        let s = &sim.seqs[&seq];
        [s.value(0, f), s.value(1, f), s.value(2, f)]
    };
    let cut_ticks: Vec<i64> = cam_cuts.iter().map(|&i| cam[i].t as i64 - cam[0].t as i64).collect();
    for (skin, traj, seq) in [(0x0500_0007u32, &suit, 0x0400_0174u32), (0x0500_000d, &tux, 0x0400_0173)] {
        let jumps = flags(traj);
        eprintln!("[dress] skin {skin:08x} ({seq:08x}) discontinuities at loop ticks {jumps:?} (of {} samples)", traj.len());
        assert!(!jumps.is_empty(), "skin {skin:08x}: expected the sequence's own staged jumps to reproduce");
        for &j in &jumps {
            let tick = traj[j].t as i64 - traj[0].t as i64;
            assert!(cut_ticks.iter().any(|&c| (c - tick).abs() <= 2),
                "skin {skin:08x} jumps at scene tick {tick} with NO camera cut within 2 ticks — a visible teleport the game masks by staging under a cut");
        }
        // The jump magnitude is in the disc data itself: the sequence's root channel pair at the
        // jump frame differs by more than the flag threshold.
        // The actors' frames run 2 ahead of the loop ticks (frame 2.0 on the first tracked tick —
        // `Script_AnimStart`'s start=1 plus the scene clock's first increment), and the flag marks
        // the tick the jump LANDS on.
        let f1 = traj[jumps[0]].t + 2;
        let (a, b) = (seq_roots(sim, seq, f1 - 1), seq_roots(sim, seq, f1));
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        eprintln!("[dress] skin {skin:08x}: seq {seq:08x} root channel f{}→{} jumps {d:.2} m in the disc data", f1 - 1, f1);
        assert!(d > 0.6, "skin {skin:08x}: the actor's jump must come from the sequence's own root channels [D], got {d:.3} m");
    }
}
