//! kimiko-actors live tests (task 701): the 07000007 cut-scene actor sets — what the scripts
//! create (`Script_AnimStart` 0x1d8f40 / `Script_EntityStart` 0x1d8d78) versus what the port
//! DRAWS per scene phase, with no duplicates. The draw set is read through `Scene::char_hidden`
//! — the same gate the render's `frame_state` walks — so the assertions are about the picture,
//! not just the sim state.
//!
//! Sources: the pack decodes in the comments ([D], `nf-extract nis 07000007.bin <hash>`), the
//! decompile ([M]), and the measured root endpoints of the fix-cutscene-actors report ([D]).
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(archive: &str) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    nf_view::load(&iso, archive, false).expect("level loads")
}

fn playing(scene: &nf_view::scene::Scene, hash: u32) -> bool {
    scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| {
        p.scripts.contains(&hash) && p.state == 2 && p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)
    })
}

/// The live `Obj::Anim`s of `hash` as (stream, skin, script), sorted.
fn live_anims(scene: &nf_view::scene::Scene, hash: u32) -> Vec<(u8, u32, u32)> {
    let mut v: Vec<(u8, u32, u32)> = scene.sim.as_ref().unwrap().drones.scripts.objects.list.iter().flatten()
        .filter_map(|o| match o {
            nf_game::nis::Obj::Anim { skin, script, src: (h, st), .. } if *h == hash => Some((*st, *skin, *script)),
            _ => None,
        }).collect();
    v.sort_unstable();
    v
}

/// The characters' draw sets through the render's own gate: (script actors (stream, skin)) and
/// (drone-backed (skin, drone index)) that are NOT hidden this frame.
fn draw_sets(scene: &nf_view::scene::Scene) -> (Vec<(u32, u8, u32)>, Vec<(u32, usize)>) {
    let opts = nf_view::scene::ViewOpts::default();
    let sim = scene.sim.as_ref().unwrap();
    let mut vis_script = Vec::new();
    let mut vis_drone = Vec::new();
    for (ci, c) in scene.characters.iter().enumerate() {
        if scene.char_hidden(ci, opts) { continue }
        match c.script {
            Some((h, st)) => vis_script.push((h, st, c.skin.hashcode)),
            None => if let Some(di) = sim.char_drone.get(ci).copied().flatten() { vis_drone.push((c.skin.hashcode, di)) },
        }
    }
    vis_script.sort_unstable();
    vis_drone.sort_unstable();
    (vis_script, vis_drone)
}

/// No duplicates: for every skin, the number of visible characters equals the number of live
/// script objects on that skin (the drones are all frozen out while a cut-scene plays).
fn assert_no_duplicates(scene: &nf_view::scene::Scene, scripts: &[u32]) {
    let (vis_script, vis_drone) = draw_sets(scene);
    assert!(vis_drone.is_empty(), "level drones must all be frozen out of the draw while the scene plays: {vis_drone:?}");
    let mut live_per_skin: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for o in scene.sim.as_ref().unwrap().drones.scripts.objects.list.iter().flatten() {
        if let nf_game::nis::Obj::Anim { skin, src: (h, _), .. } = o {
            if scripts.contains(h) { *live_per_skin.entry(*skin).or_default() += 1; }
        }
    }
    let mut vis_per_skin: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (_, _, skin) in &vis_script { *vis_per_skin.entry(*skin).or_default() += 1; }
    assert_eq!(vis_per_skin, live_per_skin, "a skin is drawn a different number of times than the script has live objects on it (duplicates or a missing actor)");
}

/// The Kimiko scene (07000007 pack 07800007.bin, 0x06000073, SP #420 via TouchOnce #412 → ch 0x48).
/// The script's actors are exactly three `Script_AnimStart` streams [D the pack's decode]:
/// stream 1 → skin 0x0500003b on script 0x060004f1, stream 2 → 0x05000015 on 0x060004f2, stream 3 →
/// 0x0500000d (the tuxedo Bond double) on 0x06000225 — plus keyed entity 0x02000acb (stream 14,
/// type 0x31). Events: 5 (the drone freeze) and 4 (player disable) at tick 0; 6/7 at 4326.
/// Per phase: pre-scene (the fireplace Kimiko drone stands, Drake waits disabled), mid-scene (the
/// three doubles draw once each, every level drone frozen out of the draw, no duplicates), after
/// the skip (the doubles deleted, the freeze lifted, the ch-0x48 drone deleted, Bond at the
/// hand-back entity). The walks' [D] root endpoints are pinned too.
#[test]
fn the_kimiko_scene_draw_set_matches_the_script_per_phase() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load("07000007.bin");
    let mut kimiko_parked = false;
    let mut drake_standing = false;
    let mut drake_unseen = false;
    // ── phase 0: before the scene, from the level start (30 ticks settle: the drones' first
    // state-machine enter parks the WaitSwitch ones) ──
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        // The fireplace Kimiko drone (placement 414, skin 0x05000015 [D]): parked DISABLED on her
        // second channel 0x48 (the scene's own out channel) — she never visibly stands; the scene
        // brings her in as its stream-2 double (same skin) and `NDrone2_PreDroneControl`
        // 0x148db0:49-78 deletes her (tag 100) when the freeze lifts.
        let kimiko = sim.drones.list.iter().find(|d| d.skin == 0x0500_0015 && d.present && (d.pos[2] + 31.65).abs() < 0.5)
            .expect("the fireplace Kimiko drone is placed and present");
        kimiko_parked = !kimiko.enabled && kimiko.second_channel == 0x48;
        // Drake's drone (skin 0x05000016, second channel 0x15 = the fly-through's out channel):
        // CivilianGuard (0x30) ENABLED at his placement [D (126.82, 47.0, -94.33), out of sight from
        // the start] — the game hides him only through event 5's freeze at the trigger and deletes
        // him (tag 100) at the free [M NDrone2_PreDroneControl 0x148db0:49-78].
        let drake = sim.drones.list.iter().find(|d| d.skin == 0x0500_0016)
            .expect("Drake's drone is placed");
        drake_standing = drake.enabled && drake.second_channel == 0x15 && drake.state == 0x30;
        drake_unseen = !drake.drawn;
    }
    assert!(kimiko_parked, "the fireplace Kimiko drone waits on ch 0x48 disabled");
    assert!(drake_standing, "Drake's drone stands enabled (CivilianGuard) with its ch-0x15 second switch");
    // Nothing script-made draws yet; Drake's standing drone is not pictured from the start
    // (the drawn cull — 97 units east, out of the cone/LOS).
    assert!(draw_sets(&scene).0.is_empty(), "no cut-scene actor draws before any scene");
    assert!(drake_unseen, "Drake's drone is not pictured from the level start (drawn cull)");
    drop(scene);

    // ── the scene: trigger 412 ──
    let mut scene = load("07000007.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.3, 44.6, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start from trigger 412");

    // ── phase 1: mid-scene ──
    for _ in 0..510 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.npc_enabled, "event 5 must freeze the drones while the scene plays");
        assert_eq!(live_anims(&scene, 0x0600_0073),
            vec![(1u8, 0x0500_003b, 0x0600_04f1), (2, 0x0500_0015, 0x0600_04f2), (3, 0x0500_000d, 0x0600_0225)],
            "the live actors must be exactly the script's three Script_AnimStart streams");
    }
    assert_no_duplicates(&scene, &[0x0600_0073]);
    {
        let (vis_script, _) = draw_sets(&scene);
        assert_eq!(vis_script, vec![(0x0600_0073, 1, 0x0500_003b), (0x0600_0073, 2, 0x0500_0015), (0x0600_0073, 3, 0x0500_000d)],
            "the draw set mid-scene is the script's three actors, once each");
    }

    // ── phase 2: late-scene, the walks' [D] root endpoints hold ──
    for _ in 0..2940 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // The root-endpoint check: pose each live double and read its root translation.
    {
        let sim = scene.sim.as_ref().unwrap();
        for (st, skin, want) in [(3u8, 0x0500_000du32, [91.97f32, -31.67]), (2, 0x0500_0015, [94.13, -35.76])] {
            let c = scene.characters.iter().find(|c| c.script == Some((0x0600_0073, st)) && c.skin.hashcode == skin).expect("the double's character");
            let script = sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
                nf_game::nis::Obj::Anim { script, src: (h, st2), skin: k, .. } if *h == 0x0600_0073 && *st2 == st && *k == skin => Some(*script),
                _ => None }).expect("the double is live");
            let skeleton = c.skeleton.clone();
            let frame = sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
                nf_game::nis::Obj::Anim { frame, src: (h, st2), skin: k, .. } if *h == 0x0600_0073 && *st2 == st && *k == skin => Some(*frame),
                _ => None }).expect("live frame");
            let local = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &skeleton, &c.skin, script, frame).expect("the body window poses");
            let world = nf_formats::anim::world_pose(&local, &c.skin);
            let r = world[0][3];
            assert!((r[0] - want[0]).abs() < 0.15 && (r[2] - want[1]).abs() < 0.15,
                "stream {st}'s double must walk to its [D] root endpoint {want:?}, got [{:.2}, {:.2}]", r[0], r[2]);
        }
    }

    // ── phase 3: the skip — Script_Free kills the doubles, the ch-0x48 drone deletes ──
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.scripts.skip_request = true;
        for _ in 0..240 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(!playing(&scene, 0x0600_0073), "the skip must end the scene");
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(live_anims(&scene, 0x0600_0073).is_empty(), "Script_Free must delete the doubles (no keep-objects flag [D])");
        assert!(sim.drones.npc_enabled, "Script_Free's 0x80 flag re-enables the party");
        assert!(!sim.drones.list.iter().any(|d| d.skin == 0x0500_0015 && d.present && (d.pos[2] + 31.65).abs() < 0.5),
            "the fireplace Kimiko drone deletes through its 0x48 second switch when the freeze lifts");
        assert!((sim.bond.pos[0] - 91.52).abs() < 0.5 && (sim.bond.pos[1] - 43.85).abs() < 0.5 && (sim.bond.pos[2] + 31.09).abs() < 0.5,
            "Bond hands back at stream 6's entity key 498 [D], got {:?}", sim.bond.pos);
    }
}

/// The Drake fly-through (07000007 pack 07900007.bin, 0x06000072, SP #419 via TouchOnce #356 →
/// ch 0x15). The script's actors are exactly 13 `Script_AnimStart` streams [D the pack's decode],
/// all spawned at tick 0 and killed at 3000 — (stream, skin, script):
/// 1:05000016/060001fd (Drake), 2:05000015/060003b2, 3:0500000d/060001fe (the tuxedo Bond double),
/// 4:05000022/060003b3, 5:05000019/060001ff, 6:05000020/060003b4, 7:05000021/06000200,
/// 8:05000024/060003b5, 9:05000023/060003b6, 10:05000019/06000201, 11:05000019/06000202,
/// 12:05000022/060003b7, 13:05000020/060003b8 — plus keyed entity 0x020004d0 (stream 14, type 0x19).
/// Events: 5 at 0 (the freeze), 4 at 0, 6/7 at 2999/3000. Drake's own drone (0x05000016, second
/// channel 0x15) waits disabled and deletes when the freeze lifts — he exists on the terrace only
/// as his script double while the scene plays.
#[test]
fn the_drake_flythrough_draw_set_matches_the_script_with_no_duplicates() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return };
    let want: Vec<(u8, u32, u32)> = vec![
        (1, 0x0500_0016, 0x0600_01fd), (2, 0x0500_0015, 0x0600_03b2), (3, 0x0500_000d, 0x0600_01fe),
        (4, 0x0500_0022, 0x0600_03b3), (5, 0x0500_0019, 0x0600_01ff), (6, 0x0500_0020, 0x0600_03b4),
        (7, 0x0500_0021, 0x0600_0200), (8, 0x0500_0024, 0x0600_03b5), (9, 0x0500_0023, 0x0600_03b6),
        (10, 0x0500_0019, 0x0600_0201), (11, 0x0500_0019, 0x0600_0202), (12, 0x0500_0022, 0x0600_03b7),
        (13, 0x0500_0020, 0x0600_03b8),
    ];
    let mut scene = load("07000007.bin");
    let mut drake_standing2 = false;
    let mut drake_unseen2 = false;
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        // Drake is not on the terrace before the scene — his drone stands far east at [D]
        // (126.82, 47.0, -94.33), out of the start view; the drawn cull keeps him unpictured, and
        // event 5 freezes him out the moment the trigger fires.
        let drake = sim.drones.list.iter().find(|d| d.skin == 0x0500_0016).expect("Drake's drone is placed");
        drake_standing2 = drake.enabled && drake.second_channel == 0x15;
        drake_unseen2 = !drake.drawn;
    }
    assert!(drake_standing2, "Drake's drone stands enabled with its second switch");
    assert!(drake_unseen2, "Drake is not pictured from the level start (drawn cull)");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [95.4, 49.6, -79.3];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0072), "the fly-through did not start from trigger 356");
    // Mid-scene: exactly the 13 script actors draw, once each; every level drone frozen out.
    for _ in 0..510 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.npc_enabled, "event 5 must freeze the drones while the fly-through plays");
        let mut live = live_anims(&scene, 0x0600_0072);
        live.sort_unstable();
        assert_eq!(live, want, "the live actor set must be the script's 13 streams");
    }
    assert_no_duplicates(&scene, &[0x0600_0072]);
    // The Bond double stands by the trigger watching ([D] root ~97.2, 50.2, -83.3), not at the origin.
    {
        let sim = scene.sim.as_ref().unwrap();
        let c = scene.characters.iter().find(|c| c.script == Some((0x0600_0072, 3))).expect("the Bond double's character");
        let skeleton = c.skeleton.clone();
        let frame = sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
            nf_game::nis::Obj::Anim { frame, src: (h, st), skin: k, .. } if *h == 0x0600_0072 && *st == 3 && *k == 0x0500_000d => Some(*frame),
            _ => None }).expect("the double is live");
        let local = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &skeleton, &c.skin, 0x0600_01fe, frame).expect("poses");
        let world = nf_formats::anim::world_pose(&local, &c.skin);
        let r = world[0][3];
        assert!((r[0] - 97.2).abs() < 1.0 && (r[2] + 83.3).abs() < 1.0,
            "the Bond double watches from the terrace ([D] root ~97.2,-83.3), got [{:.2}, {:.2}]", r[0], r[2]);
    }
    // The end: the doubles die at 3000, the freeze lifts, Drake's drone deletes (second switch 0x15).
    for _ in 0..2600 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(live_anims(&scene, 0x0600_0072).is_empty(), "the fly-through's doubles die at its end (no keep-objects [D])");
        assert!(sim.drones.npc_enabled, "the freeze lifts at the free");
        assert!(!sim.drones.list.iter().any(|d| d.skin == 0x0500_0016 && d.present),
            "Drake's drone deletes through its ch-0x15 second switch when the freeze lifts");
    }
}
