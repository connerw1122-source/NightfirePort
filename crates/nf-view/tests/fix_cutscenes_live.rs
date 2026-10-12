//! fix-cutscenes live tests: the loadable cut-scene packs (`SP_LoadScript` 0x1dbc08 → `isLoadable` 0x1d69a0 →
//! `LoadableLoad` 0x1d69e8 → `LoaderLoad` 0x1d4a00) through the real `nf_view::load` path.
//!
//! The level archive's type-0xC entries register (script hash, flag) pairs (`LoaderProcess` 0x1d4880 case 0xC:
//! u32 hash at +0, u8 flag at +4). A ScriptPlayer whose script hash is registered streams the pack file named
//! with the CURRENT LEVEL's id and the (flag+7) nibble at bits 20-23 (`LoadableLoad` builds
//! `GameState+0xC & 0xff0fffff | ((flag+7) << 20)`, `LoaderLoad` case 0 sprintf's it into "%08x.bin"):
//! 07000007's (0x06000072, 2) → `07900007.bin`, (0x06000073, 1) → `07800007.bin`; 07000008's (0x06000077, 1) →
//! `07800008.bin`, (0x06000078, 2) → `07900008.bin`. The packs hold the cut-scene NIS scripts (kind 7) and their
//! animation sequences (kind 4) — the party's Drake fly-through (0x06000072, 3000 ticks, 37 streams), the party's
//! hand-back cut-scene (0x06000073, 4326 ticks) and the gondola level's intro (0x06000077, 3880 ticks) exist
//! nowhere else on the disc.
//!
//! The user's playtest (2026-10-11): the cut-scene where Bond first sees Drake (07000007, the party) doesn't
//! play, and on the level after the party (07000008, the gondola) the cut-scene that should start almost
//! immediately doesn't play. Before the fix the port's script lib held only the level archive's type-7 entries,
//! so the mode-0 ScriptPlayers deleted themselves on their first trigger (`Player::load`'s missing-script arm
//! sets state 5) — these tests drive the real touch path (Bond's capsule vs the trigger triangles) and failed.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let iso = std::env::var("NF_ISO").ok()?;
    Some(nf_view::load(&iso, archive, false).expect("level loads"))
}

/// A player playing `hash` in state 2 with its script actually running (the game's cut-scene state).
fn playing(scene: &nf_view::scene::Scene, hash: u32) -> bool {
    scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| {
        p.scripts.contains(&hash) && p.state == 2 && p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)
    })
}

/// Whether the pack scripts are in the runtime lib at all.
fn lib_has(scene: &nf_view::scene::Scene, hash: u32) -> bool {
    scene.sim.as_ref().unwrap().drones.scripts.lib.contains_key(&hash)
}

/// Whether the SP owning `hash` was deleted (the old missing-script arm: state 5).
fn deleted(scene: &nf_view::scene::Scene, hash: u32) -> bool {
    scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| p.scripts.contains(&hash) && p.deleted)
}

/// 07000007 (the party): Bond walks into TouchOnce placement 356 (out 0x15, mask 8, box
/// [92.87, 49.01, -79.78]-[97.97, 53.82, -78.93]) → ch 0x15 → ScriptPlayer #419 (mask 1, out 0x15) plays
/// 0x06000072 — the "Bond first sees Drake" fly-through (37 streams: the camera sweep and every guest's
/// animation, no control take-over). Before the fix: the SP deleted itself and nothing played.
#[test]
fn party_drake_flythrough_starts_when_bond_walks_in() {
    let Some(mut scene) = load("07000007.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    assert!(lib_has(&scene, 0x0600_0072), "0x06000072 (the party fly-through, pack 07900007.bin) must be in the script lib");
    assert!(lib_has(&scene, 0x0600_0073), "0x06000073 (the party hand-back scene, pack 07800007.bin) must be in the script lib");
    {
        let sim = scene.sim.as_mut().unwrap();
        // Inside trigger 356's volume; a few ticks settle him (the toucher is Bond's capsule each tick).
        sim.bond.pos = [95.4, 49.6, -79.3];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(!deleted(&scene, 0x0600_0072), "SP #419 deleted itself on its trigger — the pack script was missing");
    assert!(playing(&scene, 0x0600_0072), "the Drake fly-through (0x06000072) did not start when Bond walked into trigger 356");
    // The scene must actually stage itself: its camera stream (op 0xD, `Script_CameraStart`) pushed a camera
    // object, and its anim streams (`Script_AnimStart`) built animated guests — which need the pack's kind-4
    // sequences to resolve a pose.
    let sim = scene.sim.as_ref().unwrap();
    let has_cam = sim.drones.scripts.objects.list.iter().flatten().any(|o| matches!(o, nf_game::nis::Obj::Camera { .. }));
    let anims: Vec<f32> = sim.drones.scripts.objects.list.iter().flatten().filter_map(|o| match o {
        nf_game::nis::Obj::Anim { frame, .. } => Some(*frame), _ => None }).collect();
    assert!(has_cam, "the fly-through's camera stream never started");
    assert!(anims.iter().any(|f| *f > 1.0), "no animated guest advanced past its first frame (sequences missing?): {anims:?}");
}

/// 07000007: Bond walks into TouchOnce placement 412 (out 0x48, mask 8, lock 0x63, box
/// [93.31, 43.24, -46.43]-[95.23, 44.12, -45.41], the y-44.5 hall floor) → ch 0x48 → ScriptPlayer #420 plays
/// 0x06000073 (the control-taking cut-scene: 3 × event 4/5/6/7 cycles).
#[test]
fn party_hall_cutscene_starts_from_the_floor_trigger() {
    let Some(mut scene) = load("07000007.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.3, 44.6, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(!deleted(&scene, 0x0600_0073), "SP #420 deleted itself on its trigger — the pack script was missing");
    assert!(playing(&scene, 0x0600_0073), "the party hand-back cut-scene (0x06000073) did not start from trigger 412");
}

/// 07000008 (the gondola level): Bond spawns at (139.5, 50.5, -76.4) and the intro cut-scene should start
/// "almost immediately" — TouchOnce placement 237 (out 0x49, mask 8, box [137.14, 48.49, -82.35]-[141.24,
/// 50.46, -80.15]) → ch 0x49 → ScriptPlayer #294 ("ScriptPlayer_Trigger_1stNIS") plays 0x06000077 from
/// 07800008.bin (3880 ticks, 30 streams).
#[test]
fn gondola_intro_starts_when_bond_walks_in() {
    let Some(mut scene) = load("07000008.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    assert!(lib_has(&scene, 0x0600_0077), "0x06000077 (the gondola intro, pack 07800008.bin) must be in the script lib");
    assert!(lib_has(&scene, 0x0600_0078), "0x06000078 (pack 07900008.bin) must be in the script lib");
    {
        let sim = scene.sim.as_mut().unwrap();
        // From the spawn, walk the real path into trigger 237's volume (about 3.5 m towards −z).
        sim.bond.pos = [139.2, 50.5, -80.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(!deleted(&scene, 0x0600_0077), "SP #294 deleted itself on its trigger — the pack script was missing");
    assert!(playing(&scene, 0x0600_0077), "the gondola intro (0x06000077) did not start when Bond walked into trigger 237");
}

/// Sister: the urn ScriptPlayer on 07000007 (#312, script 0x0600007A — IN the level archive, mode 64 preload)
/// must be primed (state 1, loaded) at level start, not deleted — its trigger is a bullet hit (mask 2,
/// `SP_Hit`), which the port does not model yet; the priming itself is the create-time load beat
/// (`SP_CreateScriptPlayer` 0x1db9a8 → `SP_LoadScript`, mode 64 ≠ 0/1/8 → play+update+stop, state 1 [L
/// ee_start_05: the 07000005 EndLevel SP is state 1 with clock 2.0 at level start]).
#[test]
fn party_urn_preload_is_primed_at_level_start() {
    let Some(mut scene) = load("07000007.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    // One tick of the viewer's loop: the create-time load beat runs at the first `Scripts::tick` (the game does
    // it during the level load; the port defers it to the first tick).
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    let urn = sim.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_007a).expect("urn SP exists");
    assert!(!urn.deleted && urn.state == 1, "the mode-64 urn SP must be primed (state 1), got state {} deleted={}", urn.state, urn.deleted);
    assert!(urn.info[0].is_some(), "the urn script (0x0600007A, in the level archive) is loaded at create");
}
