//! fix-objective-messages — the playtest's banner bugs (results 1013-1758 / 1013-1825), each driven
//! through the REAL path (`nf_view::load` + `Sim::step_bond`, the drawn rows measured off
//! `hud::overlay`'s quads, the layout the game computes in `HUD_UpdateStatusPane` 0x19c310):
//!
//! 1. **"I get two objective completion messages"** (07000007, the library table, picking up
//!    armour): for a type-2 message whose text IS "OBJECTIVE COMPLETE" the game hides the BODY row
//!    and shows only the status label — `HUD_UpdateStatusPane` case 2 strcmps the message text
//!    against `Txt_BindLabel(0x2000053)` and case 3 then forces the body sprite's colour to 0 in
//!    both phases [M 0x19c310 case 2 (IsComplete) + case 3's two `*(iVar8+0x10) = 0` stores]. The
//!    port drew the body row too — the same string twice, stacked.
//! 2. **a level start announces other levels' objectives**: the port's cold load fabricates a
//!    fresh-boot mission state (all records 0) for prior mission parts, and
//!    `Mission_MonitorObjectives` case 0 announces every state-0 record whose level is reached
//!    (`record.level <= GameState._12_4_` [M 0x1827d8:41-54]) — so 07000007 stacked 07000005's and
//!    07000006's banners before its own. The game never reaches that state: the record states live
//!    in `.data` and no load path reinitialises them (`ResetMap_GameInit` 0x1d5080 calls only
//!    `Mission_Init`, which touches current + future parts [M 0x182270:68-129]), so at any real
//!    arrival the prior parts sit ANNOUNCED — [L work/ee_start_0700000a.bin @ 0x2a3f78: mission
//!    07000009's rec1 (ch 0x46, uncompleted) is state **3** at 0700000a's start, with its condition
//!    channel still 0]. The port now marks prior parts' fresh state-0 records announced (2 for
//!    fail-objectives, else 3 — the monitor's own 0→2/3 transition [M 0x1827d8:47-51]) in
//!    `Mission::init`.
//! 3. **the banner text vs its box**: `Text_AddMsg` wraps the message text with
//!    `Font_WordWrapString` at `TextMsgFormats[type]`'s maxWidth — 480 for the type-2 objective
//!    text ([L TextMsgFormats 0x2bf108+0x14 = 0x1e0]; the box is `ObjectiveStatusSprInfo[2]`
//!    x 16 w 480 [L 0x319608+0x58]) — and `__Font_DrawText` advances a line by
//!    `glyph0.h + gap + 1.0` per '\n', each line left-aligned from the block's centred start
//!    [M 0x1ee598:65-71]. The port drew one unwrapped line, which runs past the box for the long
//!    objective strings.
//! 4. **"Mission Fail / Cover Lost" sits left of its box's centre**: the mission banner's rows use
//!    styles 0x30cdb8 / 0x30cce0, both `fe 03` = justify flag 2 = CENTRE (`Font_ParseFormat`
//!    0x1ee3e0 case 4 byte 3 → flag 2; `Font_DrawText` 0x1eee48 `flags & 2 → x -= width*0.5`) — the
//!    port passed its right-justify flag instead, shifting both rows left by half their width.

use nf_game::mission::LEVELS;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The font-text quads of an overlay frame, grouped into the 512×448 HUD space: (x0, x1, y0, y1)
/// per connected row band. Rows are measured by y band so the two banner rows separate.
struct Rows { quads: Vec<([f32; 2], [f32; 2])> } // (min, max) corner pairs in HUD pixels

fn font_quads(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, aspect: f32) -> Rows {
    // The font's texture id in the overlay's texture list (TEXTURES[5] = 0x03000003).
    let font_tex = scene.hud_textures.iter().position(|(h, _)| *h == 0x0300_0003).expect("the font loads") + 1;
    let mut is_font = vec![false; f.v.len()];
    for &(tex, _, start, count) in &f.runs {
        if tex == font_tex { for v in &mut is_font[start as usize..start as usize + count as usize] { *v = true; } }
    }
    // NDC → the 512×448 HUD space: x = ndc_x·(512/2)·(aspect/(4/3))·… — the overlay's ndc() maps
    // one HUD pixel to 2/SW·(4/3)/aspect NDC units, so x_ndc = (x−256)·(2/512)·(4/3)/aspect.
    let k = (2.0 / 512.0) * (4.0 / 3.0) / aspect;
    let mut quads = Vec::new();
    for (q, font) in f.v.iter().zip(&is_font) {
        if !font { continue; }
        // Every vertex is a quad corner: recover the HUD-pixel position.
        let x = q.p[0] / k + 256.0;
        let y = (1.0 - q.p[1]) * 224.0;
        quads.push(([x, y], [x, y]));
    }
    Rows { quads }
}

impl Rows {
    /// The x extent of the quads whose y falls in `y0..y1`, in HUD pixels.
    fn band(&self, y0: f32, y1: f32) -> Option<(f32, f32)> {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut any = false;
        for (a, b) in &self.quads {
            for p in [a, b] {
                if y0 <= p[1] && p[1] < y1 {
                    any = true;
                    lo = lo.min(p[0]); hi = hi.max(p[0]);
                }
            }
        }
        any.then_some((lo, hi))
    }
    fn any_in(&self, x0: f32, x1: f32, y0: f32, y1: f32) -> bool {
        self.quads.iter().any(|(a, b)| [a, b].iter().any(|p| x0 <= p[0] && p[0] <= x1 && y0 <= p[1] && p[1] <= y1))
    }
}

/// ── Item 2: a cold load of 07000007 announces ONLY 07000007's objective ────────────────────────
/// The port used to stack 07000005's "Breach the castle walls" and 07000006's "Find a way inside
/// to the party" in front of it (the fabricated fresh-boot states + case 0's `level <= current`).
#[test]
fn cold_07000007_announces_only_its_own_objective() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let lang = scene.sim.as_ref().unwrap().lang.as_ref().expect("USATxt.dat loaded").clone();
    let text = |id: u32| lang.bind(id).expect("the objective text binds").to_string();
    let (breach, inside, rendezvous) = (text(0x0400_0000), text(0x0400_0001), text(0x0400_0002));
    let mut seen: Vec<String> = Vec::new();
    for _ in 0..240 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        for m in &sim.hud.objective_msgs {
            if !seen.contains(&m.text) { seen.push(m.text.clone()); }
        }
    }
    eprintln!("[obj-msg] cold 07000007 banners: {seen:?}");
    assert!(!seen.iter().any(|t| *t == breach), "07000005's objective {breach:?} must not announce at 07000007");
    assert!(!seen.iter().any(|t| *t == inside), "07000006's objective {inside:?} must not announce at 07000007");
    assert!(seen.iter().any(|t| *t == rendezvous), "07000007's own objective {rendezvous:?} announces (its enable 0x47 fires from the level's start scripts)");
}

/// Sister: 07000008 (the mission's part 3) — only its own two objectives may announce.
#[test]
fn cold_07000008_announces_only_its_own_objectives() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load");
    let lang = scene.sim.as_ref().unwrap().lang.as_ref().expect("USATxt.dat loaded").clone();
    let text = |id: u32| lang.bind(id).expect("the objective text binds").to_string();
    let earlier: Vec<String> = [0x0400_0000, 0x0400_0001, 0x0400_0002, 0x0400_0003].iter().map(|id| text(*id)).collect();
    let own: Vec<String> = [0x0400_0004, 0x0400_0005].iter().map(|id| text(*id)).collect();
    // 07000008's own objectives are enable-gated (en 0x49/0x4a [L c07000008]); the level's TouchOnce
    // #15 (out 0x49, volume at [137.1, 48.5, -82.3] [L sim.drones.triggers]) sits by the spawn and
    // fires the first announce — walk into it through the real touch path.
    let mut seen: Vec<String> = Vec::new();
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [137.1, 48.5, -82.3];
    }
    for _ in 0..240 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        for m in &sim.hud.objective_msgs {
            if !seen.contains(&m.text) { seen.push(m.text.clone()); }
        }
    }
    eprintln!("[obj-msg] cold 07000008 banners: {seen:?}");
    for t in &earlier {
        assert!(!seen.contains(t), "the earlier objective {t:?} must not announce at 07000008");
    }
    assert!(seen.contains(&own[0]), "07000008's own objective {:?} announces once its enable 0x49 fires (touch #15)", own[0]);
}

/// ── Item 1: an OBJECTIVE COMPLETE banner draws ONE row, not two ────────────────────────────────
/// The real path: 07000007's TouchOnce #412 (out ch 0x48) completes "Rendezvous…" (its condition
/// channel is 0x48) and enables "Spy on secret meeting" (its enable is 0x48) in one tick. The
/// queued head is the COMPLETE message; while it displays the body row must be empty (the game
/// hides it [M 0x19c310 case 2/3]) and the status label alone carries "OBJECTIVE COMPLETE".
#[test]
fn an_objective_complete_banner_draws_one_row_not_two() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    // The warm carry from 05/06 (the state the playtest ran in): the earlier parts' records preset
    // complete, 07's own first objective carried complete too — the window under test is the
    // trigger's one completion, exactly as fix-objective-banner's test frames it.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
        let m = &mut sim.drones.mission;
        for (i, l) in LEVELS.iter().enumerate() {
            if l.mission == 0x0700_0005 && l.part < 2 {
                for st in m.states[i].iter_mut() { *st = 4; }
            }
        }
        // (07's own rec0 stays at its real state — the start scripts announced it, so the trigger's
        // ch 0x48 still has a completion to post.)
        for _ in 0..118 { sim.step_bond(Input::default(), false, false, false); }
        sim.hud.objective_msgs.clear();
        sim.bond.pos = [94.2, 43.7, -45.9]; // TouchOnce #412's volume
    }
    // Walk into the trigger; the completion queues when ch 0x48 rises.
    let mut queued = false;
    for _ in 0..600 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x48) && sim.hud.objective_msgs.iter().any(|m| m.status == 0x0200_0053) {
            queued = true;
            break;
        }
    }
    assert!(queued, "walking into the rendezvous volume completes the objective (ch 0x48)");
    // The cut-scene holds the banner; wait for the hand-back and the head to open, then draw.
    let mut opened = false;
    for _ in 0..6000 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.hud_frozen { continue; }
        if sim.hud.objective_msgs.front().is_some_and(|m| m.status == 0x0200_0053 && m.opened) {
            opened = true;
            break;
        }
    }
    assert!(opened, "the OBJECTIVE COMPLETE banner opens after the scene's hand-back");
    let frame = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    let rows = font_quads(&scene, &frame, 16.0 / 9.0);
    // The status row (template y 20) carries the banner's text…
    let status = rows.band(0.0, 24.0);
    assert!(status.is_some(), "the OBJECTIVE COMPLETE status label draws");
    // …and the body row (template y 40) draws NOTHING for a complete message — the port used to
    // draw the same string there again, the user's "two objective completion messages".
    assert!(!rows.any_in(0.0, 512.0, 24.0, 46.0),
        "the body row must stay hidden while OBJECTIVE COMPLETE displays");
    // Sister: the status row's text is centred on the box (16..496 → centre 256).
    let (lo, hi) = status.unwrap();
    let centre = (lo + hi) / 2.0;
    assert!((centre - 256.0).abs() <= 3.0, "the status label centres on the box (centre {centre})");
}

/// ── Item 3: the objective text stays inside the banner box (the 480-px wrap) ───────────────────
/// Queue every level's own objective lines as the game's type-2 message (the same queue the
/// monitor feeds) and measure the drawn body rows at both window shapes: every LINE must lie
/// within the box's x span (16..496 — `ObjectiveStatusSprInfo[2]` x=16 w=480 [L 0x319608+0x58];
/// `Text_AddMsg` wraps the text at TextMsgFormats[2]'s 480 [L 0x2bf108+0x14]).
#[test]
fn every_objective_text_wraps_inside_the_banner_box() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for aspect in [16.0f32 / 9.0, 4.0 / 3.0] {
        for level in LEVELS.iter() {
            for o in level.objectives {
                {
                    let sim = scene.sim.as_mut().unwrap();
                    let lang = sim.lang.as_ref().expect("USATxt.dat loaded");
                    let line = lang.bind(o.text).unwrap_or_else(|| panic!("objective {:#x} of {:08x} binds", o.text, level.level)).to_string();
                    sim.hud.objective_msgs.clear();
                    sim.hud.objective_msgs.push_back(nf_view::scene::Msg {
                        text: line, time: 300.0, status: 0x0200_0051, failed: false, life: 300.0, opened: true, second: true,
                        label: 0, // fix-context-hints added the provenance id (0 = pre-bound text)
                        script: false, // fix-ladder-water added the script-subtitle flag (this is an objective line, not a subtitle)
                        sfx1: true, // objective-banner-timing: the case-3 chime already fired (second: true pairs with it)
                    });
                }
                let frame = nf_view::hud::overlay(&scene, aspect);
                let rows = font_quads(&scene, &frame, aspect);
                for (y0, y1, what) in [(0.0f32, 24.0f32, "status"), (24.0, 46.0, "body")] {
                    if let Some((lo, hi)) = rows.band(y0, y1) {
                        assert!(lo >= 15.0 && hi <= 497.0,
                            "{:.2}: {:08x} objective {:#x} {} row spans {:.1}..{:.1}, outside the box 16..496", aspect, level.level, o.text, what, lo, hi);
                    }
                }
                assert!(rows.any_in(0.0, 512.0, 24.0, 46.0), "{:08x} objective {:#x} draws a body row", level.level, o.text);
            }
        }
    }
}

/// ── Item 4: the Mission Fail banner's rows centre on the box ───────────────────────────────────
/// The real path: ch 0x61 (the alarm channel — `Mission_MonitorObjectives`' standing fail hook
/// [M 0x1827d8:98-104]) sets FailLabel 0x4000032 and ch 99; `Mission_Update` case 2 posts the
/// type-3 "Mission Fail". The game centres both rows (styles 0x30cdb8/0x30cce0, justify 3 = centre
/// [M Font_ParseFormat 0x1ee3e0 / Font_DrawText 0x1eee48]) — the port shifted them left.
#[test]
fn the_mission_fail_banner_rows_centre_on_their_box() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
        sim.hud.objective_msgs.clear();
        // The game's own fail-hook write: switch_channels[0x61] = 1.
        sim.drones.channels.set(0x61, 1, sim.drones.frame);
    }
    let mut failed = false;
    for _ in 0..20 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        if sim.hud.mission_msgs.iter().any(|m| m.text.contains("Fail")) { failed = true; break; }
    }
    assert!(failed, "the alarm hook fails the mission and the type-3 banner queues");
    let frame = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    let rows = font_quads(&scene, &frame, 16.0 / 9.0);
    // The mission banner's rows: main at template y 42, fail label at 64 (box y 156..259).
    for (y0, y1, what) in [(190.0f32, 216.0f32, "Mission Fail"), (216.0, 240.0, "Cover Lost")] {
        let band = rows.band(y0, y1).unwrap_or_else(|| panic!("the {what} row draws"));
        let centre = (band.0 + band.1) / 2.0;
        assert!((centre - 256.0).abs() <= 3.0, "the {what} row centres on the box (centre {centre}, span {:?})", band);
    }
}
