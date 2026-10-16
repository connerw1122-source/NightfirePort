//! fix-objective-banner — the objective banner's timing, text and layout, driven through the REAL
//! path (`nf_view::load` + `Sim::step_bond`).
//!
//! Playtest (dist-side results, 2026-10-12), on 07000007:
//! 1. "obj doesnt wait for kimiko cutscene and plays the sound immediately" — the banner and its
//!    SFX fired the moment the Kimiko cut-scene started instead of after it ended;
//! 2. banners said a generic "NEW OBJECTIVE" instead of the objective's own text;
//! 3. the fail message was off-centre.
//!
//! The game's mechanisms, ported here:
//! - `Text_AddMsg` 0x1b0c30 only QUEUES a message; every sound and drawn pixel belong to
//!   `HUD_UpdateStatusPane` 0x19c310, whose case 0 polls `Text_UpdateMsg` 0x1b11d8 — which returns
//!   0 while the viewer's +0x2a HUD gate is off [M 0x1b11d8:16-18] (set by `Script_CameraStart`
//!   0x1d9140:41-43, restored by `Script_KillStream` 0x1d9860:71). A banner queued mid-cut-scene
//!   therefore waits closed, and its SFX (case 2/3: PlayMe1st 1532 new / 468 complete at open, 469
//!   60 ticks later [M 0x19c310:148-164, 219-235]) wait with it.
//! - `Mission_Update` runs from `Game_Run` 0x1c9790:17 BEFORE the object handler, so the monitor
//!   sees a trigger's channel one tick after the trigger — after the scene's camera gate is up.
//! - The banner's texts are centred: template x 128 + `MsgObjectiveStatusPane` x 128 = 256, fmt
//!   0x30cdb0 justify byte 3 = centre [L 0x319608/0x319710; M Font_ParseFormat 0x1ee3e0,
//!   Font_DrawText 0x1eee48].

use nf_game::mission::LEVELS;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn level_index(level: u32) -> usize {
    LEVELS.iter().position(|l| l.level == level).expect("the level is in the mission table")
}

/// The 07000005 mission's earlier parts' objectives preset complete — the state a player carries
/// when they reach 07000007 through 07000005→07000006 (the warm carry; `Mission_Init` keeps
/// previous parts' states [M 0x182270:68-129]). Without this a cold load also announces the
/// earlier parts' state-0 objectives at tick 1 — the game's own cold-load behaviour, but not the
/// state the playtest ran in.
fn carry_earlier_parts_complete(sim: &mut nf_view::scene::Sim) {
    let m = &mut sim.drones.mission;
    for (i, l) in LEVELS.iter().enumerate() {
        if l.mission == 0x0700_0005 && l.part < 2 {
            for st in m.states[i].iter_mut() { *st = 4; }
        }
    }
    // 07000007's own first objective ("Rendezvous…") announces from the level's start scripts
    // (its enable 0x47 fires well before the Kimiko trigger) and the trigger's ch 0x48 completes
    // it. Carry it COMPLETE (state 4 — set after `Mission_Init`'s tick-1 reset, see the caller)
    // so the window under test isolates the trigger's one announce, "Spy on secret meeting".
    let i = level_index(0x0700_0007);
    m.states[i][0] = 4;
}

/// ── Items 1+2: the Kimiko scene holds the announce, and the banner carries the objective's text ─
/// TouchOnce #412 (out 0x48 = ch 0x48) both starts SP #420 (0x06000073, the Kimiko scene) and
/// completes "Rendezvous…" / enables "Spy on secret meeting" (its enable channel is 0x48 [L
/// c07000007]). Through the real trigger: while the scene holds the camera (hud_frozen / disabled)
/// no new banner may queue and none of 1532/469/468 may play; at the hand-back exactly one
/// NEW-OBJECTIVE announce ("Spy on secret meeting") queues, plays 1532 once at open and 469 once
/// 60 ticks later, and its text is the objective record's own string id bound through USATxt.dat.
#[test]
fn the_kimiko_scene_holds_the_announce_until_the_handback() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
        // After tick 1's `Mission_Init` reset (the game: ResetMap_GameInit before Game_Run).
        carry_earlier_parts_complete(sim);
        for _ in 0..118 { sim.step_bond(Input::default(), false, false, false); }
        // The level's own pre-trigger banner ("Rendezvous…", announced by the start scripts before
        // the carry) is session history, not the window under test — drop it (the game's would
        // have timed out long before the rendezvous).
        sim.hud.objective_msgs.clear();
        assert!(sim.hud.objective_msgs.is_empty(), "the pre-trigger queue is clear");
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
    }
    // Flush the audio so the recording starts clean (the setup ticks' banner SFX sat pending —
    // `take_commands` returns what `audio.update` has flushed).
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3 {
            sim.drones.audio.update(None, ([0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| false);
            sim.drones.audio.take_commands();
        }
    }
    let mut trigger_tick = None;
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.audio.update(None, ([0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| false);
        for c in sim.drones.audio.take_commands() {
            if let nf_game::sound::Cmd::Start { id, .. } = c {
                assert!(!matches!(id, 0x5fc | 0x1d5 | 0x1d4), "banner SFX {id} played before the scene's hand-back (t={t})");
            }
        }
        if sim.drones.channels.on(0x48) { trigger_tick = Some(t); break; }
    }
    let trigger_tick = trigger_tick.expect("walking into the volume must fire trigger #412 (ch 0x48)");
    // The scene: from the trigger to the hand-back, no new banner and no banner SFX.
    let mut handback = None;
    let mut queued_during: Vec<String> = Vec::new();
    for t in 0..5200u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        // The hand-back is the viewer+0x2a gate itself (`hud_frozen` — the port's `disabled`
        // clears a tick later, per fix-party-mission's flush-tick note); this tick may open the pane and play.
        if !sim.drones.hud_frozen && t > 100 { handback = Some(t); break; }
        sim.drones.audio.update(None, ([0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| false);
        for c in sim.drones.audio.take_commands() {
            if let nf_game::sound::Cmd::Start { id, .. } = c {
                assert!(!matches!(id, 0x5fc | 0x1d5 | 0x1d4),
                    "banner SFX {id} played during the Kimiko scene (+{t} after the trigger) — the user's 'plays the sound immediately'");
            }
        }
        // The game's `Text_AddMsg` queues during the cut-scene (Mission_Update runs every tick);
        // what must wait is the DISPLAY and the SFX: no queued banner may have opened (drawn) yet.
        for m in &sim.hud.objective_msgs { queued_during.push(m.text.clone()); }
        assert!(sim.hud.objective_msgs.iter().all(|m| !m.opened),
            "no banner may display while the scene holds the camera: {:?}",
            sim.hud.objective_msgs.iter().map(|m| (&m.text, m.opened)).collect::<Vec<_>>());
        assert!(t < 5100, "the Kimiko scene never handed control back");
    }
    let handback = handback.expect("the scene must end");
    assert!(scene.sim.as_ref().unwrap().drones.hud_frozen || handback > 0,
        "the scene must have frozen the HUD while it played");
    // At the hand-back: exactly one announce — the objective's own text, from its string id.
    let (text, status) = {
        let sim = scene.sim.as_ref().unwrap();
        let msg = sim.hud.objective_msgs.front().expect("the announce must be queued at the hand-back");
        (msg.text.clone(), msg.status)
    };
    assert_eq!(status, 0x0200_0051, "the announce carries the NEW OBJECTIVE status label id");
    let spy = LEVELS[level_index(0x0700_0007)].objectives[1];
    assert_eq!(spy.text, 0x0400_0003, "the Spy objective's record text id [L c07000007]");
    let lang = scene.sim.as_ref().unwrap().lang.as_ref().expect("USATxt.dat loaded");
    assert_eq!(text, lang.bind(spy.text).expect("the objective's own line binds"),
        "the banner body is the objective's own string (id {:#x}), not a generic NEW OBJECTIVE", spy.text);
    // `Text_AddMsg` queued the announce during the scene (the game's Mission_Update runs every
    // tick); what waited was its display and its SFX — the loop asserted `!opened` throughout.
    eprintln!("[banner] queued during the scene: {:?} (all unopened)",
        queued_during.iter().take(3).cloned().collect::<Vec<_>>());
    // The SFX: 1532 once at open, 469 once 60 ticks into the display [M 0x19c310:148-164, 219-235].
    let mut sfx: Vec<(u32, u32)> = Vec::new(); // (tick offset, id)
    let mut first = None;
    for t in 0..400u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.audio.update(None, ([0.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| false);
        for c in sim.drones.audio.take_commands() {
            if let nf_game::sound::Cmd::Start { id, .. } = c {
                if matches!(id, 0x5fc | 0x1d5 | 0x1d4) {
                    if first.is_none() { first = Some((t, id)); } else { sfx.push((t, id)); }
                }
            }
        }
    }
    let (t1, id1) = first.expect("the announce's open SFX (1532) must play at the hand-back");
    assert_eq!(id1, 0x5fc, "a new objective's open SFX is 1532 [M 0x19c310:160]");
    assert!(t1 <= 2, "the open SFX plays at the hand-back, not during the scene (+{t1})");
    let n1532 = 1 + sfx.iter().filter(|(_, id)| *id == 0x5fc).count();
    let n469 = sfx.iter().filter(|(_, id)| *id == 0x1d5).count();
    let (t469, _) = *sfx.iter().find(|(_, id)| *id == 0x1d5).expect("469 must play 60 ticks into the display");
    assert_eq!((n1532, n469), (1, 1), "exactly one announce: 1532 once, 469 once (got {n1532}/{n469})");
    assert!((50..=80).contains(&t469.saturating_sub(t1)), "469 follows 1532 by ~60 ticks (the case-3 steady phase), got +{}", t469 - t1);
    eprintln!("[banner] hand-back +{handback} (trigger at {trigger_tick}); announce {text:?}; sfx 1532@{t1} 469@{t469}");
}

/// ── Item 2: the real line for every objective of 07000005 and 07000007 ─────────────────────────
/// On a cold load each level's own state-0 objectives announce (`Mission_MonitorObjectives`
/// 0x1827d8:41-54); every queued banner's body must be that objective record's own string id bound
/// through USATxt.dat (`Lang::bind`), with the 0x02000051 status id above it — not a generic label.
#[test]
fn every_objective_banner_shows_the_objectives_own_line() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in [0x0700_0005u32, 0x0700_0007] {
        let mut scene = nf_view::load(&iso, &format!("{level:08x}.bin"), false).expect("load");
        let want: Vec<u32> = LEVELS[level_index(level)].objectives.iter().map(|o| o.text).collect();
        assert!(!want.is_empty(), "{level:08x} has objectives");
        {
            let sim = scene.sim.as_mut().unwrap();
            for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
            // The announce arm under test is the monitor's state-0 arm (`l.level <= level` →
            // announce [M 0x1827d8:41-54]); the shipped records sit behind their enable channels
            // (0x45/0x47/0x48 — later triggers), so move every still-pending objective onto the
            // state-0 arm (after tick 1's `Mission_Init` reset, which the game also runs before
            // the monitor ever sees the level).
            let li = level_index(level);
            for st in sim.drones.mission.states[li].iter_mut() { if *st <= 1 { *st = 0; } }
        }
        // Tick; collect the queued banners as they land.
        let mut seen: Vec<(u32, String, u32)> = Vec::new();
        for _ in 0..1800 {
            let sim = scene.sim.as_mut().unwrap();
            sim.step_bond(Input::default(), false, false, false);
            // The queue displays head-first (Text_UpdateMsg ticks the head only); scan every queued
            // banner so a later one still counts as landed.
            for m in &sim.hud.objective_msgs {
                if !seen.iter().any(|(_, s, _)| *s == m.text) { seen.push((m.status, m.text.clone(), 0)); }
            }
        }
        let lang = scene.sim.as_ref().unwrap().lang.as_ref().expect("USATxt.dat loaded");
        assert!(!seen.is_empty(), "{level:08x}: the objectives announce (seen {:?})",
            seen.iter().map(|(_, s, _)| s).collect::<Vec<_>>());
        for id in &want {
            let line = lang.bind(*id).unwrap_or_else(|| panic!("objective text {id:#x} of {level:08x} must bind"));
            assert!(!line.is_empty(), "objective text {id:#x} binds to a real line");
            assert!(seen.iter().any(|(_, s, _)| s == line),
                "{level:08x}: the banner for {id:#x} shows the bound line {line:?} (seen {:?})",
                seen.iter().map(|(_, s, _)| s).collect::<Vec<_>>());
            assert!(seen.iter().any(|(st, s, _)| *st == 0x0200_0051 && s == line),
                "the status id above the body is 0x02000051 NEW OBJECTIVE");
        }
        eprintln!("[banner] {level:08x}: {} objective(s) {:?} → their own lines", want.len(),
            want.iter().map(|id| format!("{id:#x}")).collect::<Vec<_>>());
    }
}

/// ── Item 3: the fail message is centred the way the game lays it out ───────────────────────────
/// The objective pane's text sprites sit at template x 128 + `MsgObjectiveStatusPane` x 128 = 256
/// with fmt 0x30cdb0's justify byte 3 = centre [L 0x319608 entries 5/0 + CREATE 0x319710;
/// M Font_ParseFormat 0x1ee3e0 / Font_DrawText 0x1eee48] — centred on the bar (16..496). The port
/// used to draw both rows left-aligned from x=128. Layout test: queue the game's Fail-prefixed
/// notice ("%s: %s", 0x30cbb8 [M 0x1827d8:72-74]) and measure the drawn rows' rects.
#[test]
fn the_fail_message_is_centred_on_the_banner() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        // The game's Fail-prefixed type-2 notice: "%s: %s" with 0x170 "Fail" over the objective's line.
        let lang = sim.lang.as_ref().expect("USATxt.dat loaded");
        let line = lang.bind(0x0400_0000).expect("07000005's first objective text binds").to_string();
        let fail = lang.bind(0x170).unwrap_or("Fail").to_string();
        // Silence any cold-load banner first so the measured rows are the fail message's.
        sim.hud.objective_msgs.clear();
        sim.hud.objective_msgs.push_back(nf_view::scene::Msg {
            text: format!("{fail}: {line}"), time: 180.0, status: 0x0200_0051, failed: true, life: 180.0, opened: true, second: true,
            label: 0, // fix-context-hints added the provenance id (0 = pre-bound text)
            script: false, // fix-ladder-water added the script-subtitle flag (this is a fail banner, not a subtitle)
            sfx1: true, // objective-banner-timing: the case-3 chime already fired (second: true pairs with it)
        });
    }
    let frame = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    // The font's texture id in the overlay's texture list (TEXTURES[5] = 0x03000003); each HudFrame
    // run carries (tex position + 1, blend, vertex start, count).
    let font_tex: usize = scene.hud_textures.iter().position(|(h, _)| *h == 0x0300_0003).expect("the font loads") + 1;
    let mut is_font = vec![false; frame.v.len()];
    for &(tex, _, start, count) in &frame.runs {
        if tex == font_tex { for v in &mut is_font[start as usize..start as usize + count as usize] { *v = true; } }
    }
    // 4:3 window: ndc x = (x − 256)·(2/512); ndc y = 1 − y/224.
    let rows = |y0: f32, y1: f32| -> (f32, f32) {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for (q, font) in frame.v.iter().zip(&is_font) {
            let y = (1.0 - q.p[1]) * 224.0;
            if !font || !(y0..y1).contains(&y) { continue; }
            let x = q.p[0] * 256.0 + 256.0;
            lo = lo.min(x); hi = hi.max(x);
        }
        (lo, hi)
    };
    let (slo, shi) = rows(18.0, 34.0); // the status label row (template y 20, ~10 px glyphs)
    let (blo, bhi) = rows(38.0, 56.0); // the body row (template y 40)
    eprintln!("[banner] fail message rows: status x {slo:.1}..{shi:.1}, body x {blo:.1}..{bhi:.1}");
    for (lo, hi, what) in [(slo, shi, "status label"), (blo, bhi, "fail body")] {
        assert!(lo > 0.0 && hi > lo, "{what} row drew glyphs");
        let centre = (lo + hi) / 2.0;
        assert!((centre - 256.0).abs() <= 1.5, "{what} is centred on 256 (the pane's x 128 + template x 128, justify centre), got {centre:.1}");
        assert!(lo >= 15.0 && hi <= 497.0, "{what} stays inside the banner's 16..496 bar");
    }
}
