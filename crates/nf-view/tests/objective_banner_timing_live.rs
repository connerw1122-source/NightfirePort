//! objective-banner-timing — the 07000007 banner/sound timing through the REAL paths, cold AND
//! carried, plus the immediacy sister and the flash-window hold. Companion to
//! fix_objective_banner_live.rs (which pins the Kimiko-scene hold on a cold load with preset
//! states); this file pins the windows that task's tests did not:
//!
//! 1. the SISTER: outside any cut-scene the banner shows and sounds AT ONCE (the game's pane
//!    polls `Text_UpdateMsg` every frame; nothing delays it when the viewer's +0x2a HUD gate is
//!    up [M 0x1b11d8:16-18, 0x19c310 case 0]);
//! 2. the Kimiko hold through the 06 → 07 CARRY (the playtests' real flow), with the mission
//!    states a finished-06 player carries — the announce waits for the hand-back there too;
//! 3. the case-3 chime shape: the game plays PlayMe1st one pane frame AFTER the open and only
//!    while `ScriptCam == 0` [M 0x19c310:219-229] — a scene starting inside a banner's flash
//!    window holds the chime until the hand-back (the pane itself stays up, its timer frozen by
//!    the same gate [M 0x1b11d8:16-18]).

use nf_game::mission::LEVELS;
use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn level_index(level: u32) -> usize {
    LEVELS.iter().position(|l| l.level == level).expect("the level is in the mission table")
}

fn hear_banner_sfx(sim: &mut nf_view::scene::Sim, out: &mut Vec<(u32, u32)>, t: u32) {
    sim.drones.audio.update(None, ([0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| false);
    for c in sim.drones.audio.take_commands() {
        if let nf_game::sound::Cmd::Start { id, .. } = c {
            if matches!(id, 0x5fc | 0x1d5 | 0x1d4) { out.push((t, id)); }
        }
    }
}

/// ── The sister: outside a cut-scene the banner shows and sounds at once ─────────────────────────
/// The level's own first announce ("Rendezvous…", enabled by the start scripts' ch 0x47/0x48 arm
/// [L c07000007]) queues on a cold load with no camera anywhere near: hud_frozen stays false, the
/// pane opens the tick the message lands, PlayMe1st (1532) fires the next pane frame (case 3
/// [M 0x19c310:219-229]) and PlayMe2nd (469) when the steady phase begins (remaining + 60 ≤ life
/// [M 0x19c310:232-235]).
#[test]
fn outside_a_cutscene_the_banner_shows_and_sounds_at_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let mut sfx: Vec<(u32, u32)> = Vec::new();
    let mut queued_at = None;
    let mut opened_at = None;
    for t in 0..200u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        hear_banner_sfx(sim, &mut sfx, t);
        assert!(!sim.drones.hud_frozen, "no cut-scene runs at the load — the gate must stay up (t={t})");
        let sim = scene.sim.as_ref().unwrap();
        if let Some(m) = sim.hud.objective_msgs.front() {
            if queued_at.is_none() { queued_at = Some(t); }
            if m.opened && opened_at.is_none() { opened_at = Some(t); }
        }
    }
    let queued_at = queued_at.expect("the level's own announce queues at the load");
    let opened_at = opened_at.expect("the banner opens");
    assert!(opened_at - queued_at <= 1,
        "outside a cut-scene the banner opens at once (queued t={queued_at}, opened t={opened_at})");
    let (t1, id1) = *sfx.first().expect("the open chime plays");
    assert_eq!(id1, 0x5fc, "a new objective's chime is 1532 [M 0x19c310:160]");
    assert!(t1 >= opened_at && t1 - opened_at <= 2,
        "the chime is case 3 — the pane frame after the open (open t={opened_at}, chime t={t1})");
    let Some((t2, id2)) = sfx.iter().copied().find(|(_, id)| *id == 0x1d5) else {
        panic!("469 must play once the steady phase begins, sfx {sfx:?}");
    };
    assert!((55..=65).contains(&t2.saturating_sub(t1)), "469 follows 1532 by ~60 ticks, got +{}", t2 - t1);
    assert_eq!(sfx.iter().filter(|(_, id)| *id == 0x5fc).count(), 1, "exactly one announce in the window");
    eprintln!("[banner] sister: queued {queued_at}, opened {opened_at}, 1532@{t1}, 469@{t2}");
}

/// ── The Kimiko hold through the 06 → 07 carry (the playtests' real flow) ────────────────────────
/// A finished-06 player's carried states (every earlier part's objectives complete, the snapshot
/// riding the load the way the game's `.data` does [M LS_MakeMission/LS_LoadMission 0x1cd720/
/// 0x1cd880; map-tower2]), then the real trigger: TouchOnce #412 (out 0x48) starts SP #420 (the
/// Kimiko scene) and completes "Rendezvous…" / enables "Spy on secret meeting". From the trigger
/// to the scene's hand-back no banner may display and none of 1532/469/468 may play; after it the
/// COMPLETE announce opens first (468), and the NEW announce ("Spy on secret meeting") opens when
/// the pane frees, with 1532 and 469 in the game's case-3 shape.
#[test]
fn the_carry_holds_the_kimiko_announce_until_the_handback() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The 06 leg: load, complete every 07000005-mission objective up to and including 06's part
    // (the state a player who finished 06 carries), capture the RamSave with the mission snapshot.
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 06");
    let save = {
        let s = scene.sim.as_mut().unwrap();
        s.drones.health.health = 80.0;
        s.drones.channels.set(0x1D, 1, s.drones.frame);
        {
            let m = &mut s.drones.mission;
            for (i, l) in LEVELS.iter().enumerate() {
                if l.mission == 0x0700_0005 && l.level <= 0x0700_0006 {
                    for st in m.states[i].iter_mut() { *st = 4; }
                }
            }
        }
        for _ in 0..3 { s.step_bond(Input::default(), false, false, false); }
        RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels)
            .with_mission(&s.drones.mission)
    };
    drop(scene);
    // The 07 leg through the carry.
    let mut scene = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&save)).expect("carry load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
        // The load announce ("Rendezvous…") is session history by the time the player reaches the
        // trigger — drop it so the window under test isolates the trigger's announces.
        sim.hud.objective_msgs.clear();
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
    }
    // Flush the audio so the recording starts clean.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3 { hear_banner_sfx(sim, &mut Vec::new(), 0); }
    }
    // Walk into the volume; the trigger fires ch 0x48 and the scene's camera gate goes up.
    let mut trigger_tick = None;
    let mut sfx: Vec<(u32, u32)> = Vec::new();
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        hear_banner_sfx(sim, &mut sfx, t);
        if sim.drones.channels.on(0x48) { trigger_tick = Some(t); break; }
    }
    let trigger_tick = trigger_tick.expect("walking into the volume must fire trigger #412 (ch 0x48)");
    assert!(sfx.is_empty(), "no banner SFX around the trigger itself, got {sfx:?}");
    // The scene: from the trigger to the hand-back, no banner SFX and no display.
    let mut handback = None;
    for t in 0..5200u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        if !sim.drones.hud_frozen && t > 100 { handback = Some(t); break; }
        hear_banner_sfx(sim, &mut sfx, t);
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.hud.objective_msgs.iter().all(|m| !m.opened),
            "no banner may display while the scene holds the camera: {:?}",
            sim.hud.objective_msgs.iter().map(|m| (&m.text, m.opened)).collect::<Vec<_>>());
        assert!(t < 5100, "the Kimiko scene never handed control back");
    }
    let handback = handback.expect("the scene must end");
    assert!(sfx.is_empty(), "no banner SFX during the Kimiko scene (the user's 'plays the sound immediately'), got {sfx:?}");
    // The hand-back: the COMPLETE announce (obj 0, completed by the trigger) opens first — 468.
    let (t468, id468) = loop {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        let mut one = Vec::new();
        hear_banner_sfx(sim, &mut one, 0);
        if let Some(&(tt, id)) = one.first() { break (tt, id); }
    };
    assert_eq!(id468, 0x1d4, "the completed 'Rendezvous…' announce chimes 468 [M 0x19c310:157]");
    assert!(t468 == 0, "the COMPLETE chime fires the pane frame after the hand-back, got +{t468}");
    // …then the NEW announce opens when the COMPLETE banner (180 ticks) expires.
    let spy = LEVELS[level_index(0x0700_0007)].objectives[1];
    assert_eq!(spy.text, 0x0400_0003, "the Spy objective's record text id [L c07000007]");
    let mut sfx2: Vec<(u32, u32)> = Vec::new();
    let mut spy_opened = None;
    for t in 0..400u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        hear_banner_sfx(sim, &mut sfx2, t);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(m) = sim.hud.objective_msgs.front() {
            if m.status == 0x0200_0051 && m.opened && m.text == scene.sim.as_ref().unwrap().lang.as_ref().unwrap().bind(spy.text).unwrap() && spy_opened.is_none() {
                spy_opened = Some(t);
            }
        }
    }
    let spy_opened = spy_opened.expect("the Spy announce must open after the COMPLETE banner's 180 ticks");
    assert!(spy_opened >= 170, "the COMPLETE banner (180 ticks) plays out first, spy opened at +{spy_opened}");
    let Some((t1532, id1532)) = sfx2.iter().copied().find(|(_, id)| *id == 0x5fc) else {
        panic!("the Spy announce chimes 1532, sfx {sfx2:?}");
    };
    assert!((spy_opened as i32 - t1532 as i32).abs() <= 2, "1532 at the Spy announce's open");
    let Some((t469, _)) = sfx2.iter().copied().find(|&(t, id)| id == 0x1d5 && t > t1532) else {
        panic!("469 follows the Spy announce's chime, sfx {sfx2:?}");
    };
    assert!((55..=65).contains(&t469.saturating_sub(t1532)), "469 follows 1532 by ~60 ticks, got +{}", t469 - t1532);
    eprintln!("[banner] carry: trigger at {trigger_tick}, hand-back +{handback}, 468@+{t468}, spy opened +{spy_opened}: 1532@{t1532} 469@{t469}");
}

/// ── The flash-window hold: a scene starting right after a banner's open holds its chime ─────────
/// The game's PlayMe1st is case 3 — the pane frame after case 2's open — and only while
/// `ScriptCam == 0` [M 0x19c310:219-229]. A banner that opens the tick before a cut-scene starts
/// keeps the pane open (case 3 runs; only `Text_UpdateMsg`'s advance is gated [M 0x1b11d8:16-18]),
/// its timer frozen mid-flash-window, and the chime waits for the hand-back. The port must do the
/// same (the chime is the first unfrozen pane frame in the flash window). The banner here is
/// hand-queued (the level has no real objective that completes one tick before a scene); the pane
/// machinery under test is the real one.
#[test]
fn a_scene_starting_in_the_flash_window_holds_the_open_chime() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
        sim.hud.objective_msgs.clear();
        // Queue a fresh NEW-OBJECTIVE banner the game way (unopened): it opens on this tick's pane
        // frame and would chime on the next.
        sim.hud.objective_msgs.push_back(nf_view::scene::Msg {
            text: "flash window probe".to_string(), time: 300.0, status: 0x0200_0051, failed: false,
            life: 300.0, opened: false, second: false, label: 0, script: false,
            sfx1: false,
        });
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
    }
    let mut sfx: Vec<(u32, u32)> = Vec::new();
    let mut opened = None;
    let mut frozen_from = None;
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        hear_banner_sfx(sim, &mut sfx, t);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.hud_frozen { frozen_from = Some(t); break; }
        if let Some(m) = sim.hud.objective_msgs.front() {
            if m.opened && m.text == "flash window probe" && opened.is_none() { opened = Some(t); }
        }
    }
    let frozen_from = frozen_from.expect("the Kimiko scene must start");
    let opened = opened.expect("the probe banner opens before the scene");
    assert!(frozen_from - opened <= 2, "the scene must start inside the banner's flash window (open {opened}, frozen {frozen_from})");
    // Through the scene: the chime must NOT play while the camera holds.
    let mut handback = None;
    for t in 0..5200u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        if !sim.drones.hud_frozen && t > 100 { handback = Some(t); break; }
        hear_banner_sfx(sim, &mut sfx, t);
        assert!(t < 5100, "the Kimiko scene never handed control back");
    }
    let handback = handback.expect("the scene must end");
    assert!(!sfx.iter().any(|&(t, _)| t >= frozen_from), "no banner SFX while the scene holds the camera, got {sfx:?}");
    // At the hand-back the banner is still in its flash window (the timer froze) — the chime fires.
    let mut chime = None;
    for t in 0..10u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        let mut one = Vec::new();
        hear_banner_sfx(sim, &mut one, t);
        if one.iter().any(|&(_, id)| id == 0x5fc) { chime = Some(t); break; }
    }
    let chime = chime.unwrap_or_else(|| panic!("the held chime must fire at the hand-back, sfx {sfx:?}"));
    assert!(chime <= 2, "the chime fires the first unfrozen pane frame in the flash window, got +{chime}");
    eprintln!("[banner] flash-window hold: open {opened}, scene from {frozen_from}, hand-back +{handback}, chime +{chime}");
}
