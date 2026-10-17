//! hint-bar-speed-05 (task 739): the hint bar's timing — the game's tick count, the fade step,
//! frame-rate independence, and the head-ticks-alone queue semantics.
//!
//! Game path, for every assertion here [M unless tagged]:
//! - The hint posts `Text_AddMsg(viewer, 0, 1, Txt_BindLabel(label), 0, 0xb4)` — TXTMSG type 1 on
//!   the INFO pane, life 0xB4 = 180 ticks. Hint_Update 0x1b3380:33 (the stack arg confirmed at
//!   0x1b3450 `addiu $t1, $zero, 0xb4`); ThirdIcon_Update 0x196e80:42 posts the ctx-6 "Press
//!   Action" (label 0x1000291) the same way on its instance timer.
//! - The pane update runs every tick: `HUD_Update` from the top of `Player_Update` 0x193600:79
//!   (no frame gating), walking all 22 panes to `HUD_UpdateStatusPane` 0x19c310.
//! - `Text_UpdateMsg` 0x1b11d8:48-50 decrements the pane's HEAD node by FRAME_RATE_MUL = 1.0
//!   [L 0x30d0d8] per tick — head only (the out-param `*param_3` IS the head; the queue behind
//!   waits and displays its FULL life when it surfaces). Delete at ≤ 0.
//! - The fade: while `time < 15.0` the middle/text alpha byte becomes `time · 17`
//!   [M 0x19c310:261-265 `fptoui(fVar22 * 17.0)`]; the bar closes 16 px per tick
//!   [M 0x19c310:327-336; the pane's slide secondary[3] = 0x10 [L docs/fields/hud-panes.md]].
//! - The port's sim is fixed 60 Hz at any render rate: the app's `RedrawRequested` scheduler is
//!   `ticks_for_frame` (crates/nf-view/src/lib.rs:87, `anim_time += dt·60`, capped) and each
//!   returned tick runs one `step_bond`. 240 rendered fps = dt 1/240 = 0.25 ticks/frame — the
//!   hint must last the SAME 180 sim ticks (the same 3 s of game time) as at 60 Hz.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let idle = Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
    scene
}

/// The 07000005 ctx-6 icon beside the vent crawl (the fix-context-hints live pin: its sphere
/// arms at the stand-in spot) [D placement data, probed by fix_context_hints_live's dump].
const VENT_ICON: [f32; 3] = [24.20, 31.93, 60.01];
const PRESS_ACTION: u32 = 0x0100_0291;

/// One tick until the "Press Action" message posts (Bond standing in the icon's sphere).
fn fire_hint(scene: &mut nf_view::scene::Scene) -> (u32, f32) {
    let idle = Input::default();
    for t in 0..200 {
        scene.sim.as_mut().unwrap().step_bond(idle, false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(m) = sim.hud.info_msgs.front() {
            if m.label == PRESS_ACTION { return (t, m.time); }
        }
    }
    panic!("the ctx-6 hint never posted at the vent icon");
}

/// Let Bond settle at the SILENT grapple spot (8.17 from the icon — the game reads no context
/// there [L PCSX2 2026-10-14]), so no post burns during the landing. The instance timer expires
/// meanwhile (it decrements regardless of range [M 0x196e80:15-17]).
fn settle_silent(scene: &mut nf_view::scene::Scene) {
    let idle = Input::default();
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([16.5, 29.6, 58.6], -2.01);
    }
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
}

/// The hint shows for the game's tick count: 0xB4 = 180 ticks from post to pop
/// [M 0x1b3380:33 + 0x1b11d8:48-50]. The message's own remaining time at first sight plus the
/// ticks it survives as the front must sum to exactly 180.
#[test]
fn the_press_action_hint_lasts_the_games_180_ticks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    settle_silent(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    let (_post_tick, first_time) = fire_hint(&mut scene);
    assert_eq!(first_time, 179.0,
        "the post tick already spends one of the 0xB4 ticks [M 0x1b3380:33, hud_tick drains then decrements] (got {first_time})");
    let idle = Input::default();
    let mut counted = 0u32;
    loop {
        scene.sim.as_mut().unwrap().step_bond(idle, false, false, false);
        counted += 1;
        let sim = scene.sim.as_ref().unwrap();
        match sim.hud.info_msgs.front() {
            Some(m) if m.label == PRESS_ACTION => assert!(counted < 400, "the hint never expired"),
            _ => break,
        }
    }
    assert_eq!(1 + counted, 180,
        "the hint displays exactly the game's 0xB4 = 180 ticks: the post tick + the rest [M 0x1b3380:33, 0x1b11d8:48-50] (got 1 + {counted})");
    // The close animation runs 16 px per tick from the 480-wide bar [M 0x19c310:327-336]: 30 ticks.
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.hud.info_close > 0.0, "the bar closes after the message expires");
}

/// Frame-rate independence [the task's test]: the same hint at 240 rendered frames per second —
/// the app's exact scheduler (`ticks_for_frame` with dt = 1/240, 4-tick cap) driving one
/// `step_bond` per returned tick — displays the same 180 sim ticks as the 60 Hz run.
#[test]
fn the_hint_is_frame_rate_independent_at_240_rendered_fps() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let idle = Input::default();

    // 60 Hz reference: one step per tick, the lifetime counted from the post tick.
    let mut scene = load(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    settle_silent(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    let _ = fire_hint(&mut scene);
    let mut life60 = 1u32; // the post tick already shows the message
    loop {
        scene.sim.as_mut().unwrap().step_bond(idle, false, false, false);
        life60 += 1; // the pop tick counts too: the message was up for it
        let up = scene.sim.as_ref().unwrap().hud.info_msgs.front().is_some_and(|m| m.label == PRESS_ACTION);
        if !up { break; }
        assert!(life60 < 400, "the 60 Hz hint never expired");
    }
    assert_eq!(life60, 180, "the 60 Hz reference: the game's 0xB4 ticks [M 0x1b3380:33]");

    // 240 rendered fps: 4 frames per tick of wall time, the scheduler hands out 0.25 ticks/frame.
    let mut scene = load(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    settle_silent(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    let _ = fire_hint(&mut scene);
    let mut anim: f32 = 0.0;
    let mut life240 = 1u32; // sim ticks, post tick included
    let mut done = false;
    for _frame in 0..6000u32 {
        let n = nf_view::ticks_for_frame(&mut anim, 1.0 / 240.0, 4);
        for _ in 0..n {
            scene.sim.as_mut().unwrap().step_bond(idle, false, false, false);
            life240 += 1; // the pop tick counts too
            let up = scene.sim.as_ref().unwrap().hud.info_msgs.front().is_some_and(|m| m.label == PRESS_ACTION);
            if !up { done = true; break; }
            assert!(life240 < 400, "the 240 fps hint never expired");
        }
        if done { break; }
    }
    assert!(done, "the 240 fps run never expired the hint");
    assert_eq!(life240, 180,
        "the hint lasts the same 180 sim ticks at 240 rendered fps [M 0x1b11d8:48-50 — the step is per tick, not per frame] (got {life240})");
}

/// The fade step: alpha = time·17 while time < 15 [M 0x19c310:261-265 `fptoui(fVar22 * 17.0)`].
/// Pinned on the drawn text quad's alpha at two ticks of the fade window.
#[test]
fn the_info_text_fades_17_per_tick_over_the_last_15() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    settle_silent(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([23.0, 30.6, 59.8], -2.01);
    }
    let (_post, mut time) = fire_hint(&mut scene);
    // Walk the message to time = 10 (inside the fade window): alpha = 170.
    let idle = Input::default();
    let mut t = 0u32;
    while time > 10.0 {
        scene.sim.as_mut().unwrap().step_bond(idle, false, false, false);
        t += 1;
        time -= 1.0;
        assert!(t < 400);
    }
    let f = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    // The info text draws in the bar's band (y 113..139 [L MsgInfoStatusPane 0x319810]); its
    // colour is gs(0x7d6d59, time·17) — the RGB pins the text quads (the bar quads are 0x7f7f7f
    // at the constant 0x8d alpha), the alpha channel carries the fade (gs halves it: (a>>1)/128).
    let pix = |ndc_y: f32| (1.0 - ndc_y) * (448.0 / 2.0);
    let (r, b) = (0x7d as f32 / 128.0, 0x59 as f32 / 128.0);
    let alphas: Vec<f32> = f.v.iter()
        .filter(|v| (v.c[0] - r).abs() < 0.004 && (v.c[2] - b).abs() < 0.004
            && pix(v.p[1]) > 113.0 && pix(v.p[1]) < 140.0)
        .map(|v| v.c[3]).collect();
    assert!(!alphas.is_empty(), "the info text draws inside the bar band");
    for a in &alphas {
        let byte = (a * 128.0) * 2.0;
        assert!((byte - 170.0).abs() <= 2.0,
            "at time 10 the fade alpha is 10·17 = 170 [M 0x19c310:264] (got {byte})");
    }
}

/// The head ticks alone [M 0x1b11d8:48-73 — the out-param IS the head; the queue behind waits]:
/// a message queued behind another displays its FULL life when it surfaces. The port used to tick
/// every member in parallel, which spent the queued texts' lives before they ever showed — they
/// flashed past one after another (playtest 1016-1355, "the hints are being displayed at 1,000
/// miles an hour", 07000005).
#[test]
fn queued_messages_display_their_full_life() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let idle = Input::default();
    // Three pickup texts through the real drain path (`drones.taken` — the events Pickup_Handler
    // 0x18e650 emits), staggered 60 ticks apart; each carries the game's 180-tick life
    // [M 0x18e650 shared tail, docs/reports/fix-pickups-ammo.md]. Distinct kinds → distinct texts.
    let events = [nf_game::pickup::Taken::Armour, nf_game::pickup::Taken::Key { id: 7 }, nf_game::pickup::Taken::Bonus];
    let mut seen: Vec<(String, f32)> = Vec::new(); // (text, remaining at first sight)
    let mut last_text = String::new();
    let mut ev = 0usize;
    for tick in 0..1200u32 {
        if tick % 60 == 0 && ev < events.len() {
            scene.sim.as_mut().unwrap().drones.taken.push((tick, events[ev].clone()));
            ev += 1;
        }
        scene.sim.as_mut().unwrap().step_bond(idle, false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(m) = sim.hud.msgs.front() {
            if m.text != last_text {
                seen.push((m.text.clone(), m.time));
                last_text = m.text.clone();
            }
        } else if ev == events.len() && seen.len() == events.len() {
            break;
        }
        assert!(tick < 1190, "the queue never drained (got {seen:?})");
    }
    assert_eq!(seen.len(), 3, "all three texts surfaced (got {seen:?})");
    // The 2nd and 3rd waited in the queue: their lives must be UNTOUCHED — each surfaces with the
    // full 180. The parallel tick spent them while they waited (each would first show
    // 180 − 60·k remaining and pop early — the flash-past).
    for (i, (_, first)) in seen.iter().enumerate().skip(1) {
        assert!((*first - 180.0).abs() <= 1.0,
            "queued message {i} surfaces with its full 180-tick life [M 0x1b11d8:48-73 — only the head ticks] (got {first})");
    }
}
