//! fix-context-hints (100h): the context indicator + the hint bar.
//!
//! Two PCSX2 symptoms from the main agent (2026-10-13): standing at a usable thing the real game
//! shows (1) the hint bar at the top ("Hint: Use Watch Laser" in front of the 07000008 safe,
//! `--bond 155.19,43.12,-113.38,1.57,0`) and (2) the context icon (the game's third-icon system:
//! `ThirdIcon_Update` 0x196e80 writes the enabled icon's context type into `BLData+0x95F`, the
//! HUD's health-pane sprite 18 shows texture table 0x2FC168[ctx] while it is set). The port's
//! `icon_ctx` plumbing existed (fix-wire2/fix-08-walls) but nothing drew the icon and no hint
//! message ever reached the HUD.
//!
//! Game path, for every assertion here [M unless tagged]:
//! - `Hint_Create` 0x1b3248 / `Hint_Update` 0x1b3380 (placement kind 0x33, class 0x43): the
//!   one-shot hint markers — instance = {param 1 sound, param 0 label, param 3 on-channel,
//!   param 4 off-channel}; the update walks the marker's obj+0xD0 chain, and when the class-3
//!   player is in it (with the channels satisfied) posts `Text_AddMsg(vol, 0, 1,
//!   Txt_BindLabel(param 0), 0, 0xb4)` — TXTMSG type 1, the INFO pane — and deletes itself
//!   (obj+0xFE bit 0). One-shot per level load.
//! - `ThirdIcon_Update` 0x196e80 (placement kind 0xFA, class 0x42, ported in scene.rs by
//!   fix-wire2/fix-08-walls): with a class-3 object in its chain it writes the context type and
//!   (type 6 only, player substate != 7) posts label 0x1000291 ("Press Action" [D USATxt.dat
//!   entry fixups[1]+0x291]) the same way, on the icon instance's 4-tick timer (data+8: −1/tick,
//!   reset to 4 per write [M 0x196e80:15-17,52-53]).
//! - The type-1 message draws on the INFO pane (pane 4, `MsgInfoStatusPane` 0x319810 [L],
//!   `HUD_UpdateStatusPane` 0x19c310 — the same state machine the pickup pane ports): the bar is
//!   `InfoStatusSprInfo[2]` (480×26 at pane x 16, y 0 — the screen TOP [L ee_start_05.bin]), the
//!   text centred at (256, 16) in font 2 colour 0x7D6D59 with the black shadow [L], the caps
//!   riding the bar's ends [M 0x19c310:342-347]; the close shrinks 16 px/tick (the CREATE's
//!   slide 0x10 [L 0x319810+0x14]).
//! - The context icon (health pane sprite 18, `HUD_UpdateHealthPane` 0x19b9c8:273-303): while
//!   `BL+0x95F != 0xFF` the texture comes from the 0x2FC168 table (0x03000064+ctx — only
//!   0x66/0x67/0x6A exist on the disc [D tex-list]), the sprite draws at screen (400, 48) —
//!   the runtime x/y override the off-screen template [M disasm 0x19c124-0x19c14c] — texture-
//!   sized, layer 29, colour 0x7f7f7fff, and `ThirdIconTimer` = 30; on ctx 0xFF it fades out
//!   over that timer (alpha = timer·255/30 [M 0x19c160-0x19c1d4]).
//!
//! The assertions pin NUMBERS and LABEL IDS only (the strings come from the disc's USATxt.dat —
//! 0x0200000A = "Hint: Use Watch Laser", 0x010001E9 = "Press Action to open the control panel",
//! 0x1000291 = "Press Action" [D, read off the disc for this task] — never asserted here).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A level loaded with Bond in control at the spawn (the same warm-up the other live tests use).
fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let idle = Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
    scene
}

/// The first displayed INFO message's posting label id (the raw `Txt_BindLabel` id the game
/// posts, carried on the bound message — numbers only).
fn first_info_label(scene: &nf_view::scene::Scene) -> Option<u32> {
    scene.sim.as_ref().unwrap().hud.info_msgs.front().map(|m| m.label)
}

/// The overlay run index of a texture hash (`HudFrame.runs`'s tex = the hud_textures position + 1).
fn run_for(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32) -> bool {
    match scene.hud_textures.iter().position(|(h, _)| *h == hash) {
        Some(ti) => f.runs.iter().any(|(t, _, _, _)| *t == ti + 1),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// The data dump (kept as a test so a plain `cargo test -- --nocapture` prints it).
// ---------------------------------------------------------------------------

#[test]
fn dump_context_data_07000008_and_05() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000008.bin", "07000005.bin"] {
        let scene = load(&iso, level);
        let sim = scene.sim.as_ref().unwrap();
        println!("== {level} ==");
        println!("third icons (kind 0xFA): {}", sim.icons.len());
        for (i, ic) in sim.icons.iter().enumerate() {
            println!("  icon[{i}] ctx={} on={} at ({:.2},{:.2},{:.2}) r {:.3} cels {:?}", ic.ctx, ic.on_channel, ic.centre[0], ic.centre[1], ic.centre[2], ic.radius, sim.icon_cels[i]);
        }
        println!("hints (kind 0x33): {}", sim.hints.len());
        for (i, h) in sim.hints.iter().enumerate() {
            println!("  hint[{i}] label={:08x} sound={} on={} off={} at ({:.2},{:.2},{:.2}) r {:.3} cels {:?}",
                h.label, h.sound, h.on_channel, h.off_channel, h.centre[0], h.centre[1], h.centre[2], h.radius, sim.hint_cels[i]);
        }
    }
}

// ---------------------------------------------------------------------------
// (1) The hint bar at the 07000008 safe.
// ---------------------------------------------------------------------------

/// The main agent's safe spot [D PCSX2 2026-10-13]: the level's watch-laser hint marker
/// (label 0x0200000A = "Hint: Use Watch Laser" [D USATxt.dat entry 1822]) sits 5.5 units away
/// (158.88, 42.22, −117.34, r 6.458) and fires once — a type-1 INFO message with life 0xB4.
#[test]
fn the_safe_shows_the_watch_laser_hint_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        // The level data: exactly one hint marker carries the watch-laser label.
        assert!(sim.hints.iter().any(|h| h.label == 0x0200_000a),
            "07000008 places a Hint marker with label 0x0200000A [D placement param 0]");
        let hi = sim.hints.iter().position(|h| h.label == 0x0200_000a).unwrap();
        assert_eq!(sim.hints[hi].on_channel, 0, "the safe hint has no arming channel [D param 3]");
        assert_eq!(sim.hints[hi].off_channel, 0, "the safe hint has no suppression channel [D param 4]");
        // Bond at the safe, facing it.
        sim.bond = Bond::new([155.19, 43.12, -113.38], 1.57);
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert_eq!(first_info_label(&scene), Some(0x0200_000a),
        "the safe hint fires with the game's label id [M Hint_Update 0x1b3380 posts Txt_BindLabel(param 0)] (got {:?})",
        first_info_label(&scene));
    {
        let sim = scene.sim.as_ref().unwrap();
        let msg = sim.hud.info_msgs.front().expect("the bound message reached the INFO queue");
        assert!(!msg.text.is_empty(), "the label bound to the disc's string (USATxt.dat)");
        assert!(msg.time > 100.0 && msg.time <= 180.0,
            "the message carries the 0xB4 = 180-tick life, ticked down only by the steps since it fired [M 0x1b3380:33] (got {})", msg.time);
    }
    // One-shot [M Hint_Update 0x1b3380:41-42 — obj+0xFE bit 0 deletes the marker]: more ticks
    // in range never queue a second copy.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.hud.info_labels.len(), 0, "the marker does not re-fire while the message is up");
    assert_eq!(sim.hud.info_msgs.len(), 1, "still the one message");
    // Sister: the context id at the safe — the game's only 07000008 third icon is the zipline's
    // ctx-3 marker 21 units away in another cel [D probe; fix_08_walls_live pins its straddle
    // set = cell 12], so `BLData+0x95F` stays 0xFF here and the context icon hides.
    assert_eq!(sim.bond.icon_ctx, 0xff,
        "no third icon arms at the safe [D the level's one icon is the zipline's] (got {})", sim.bond.icon_ctx);
    let f = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    assert!(!run_for(&scene, &f, 0x0300_006a) && !run_for(&scene, &f, 0x0300_0066) && !run_for(&scene, &f, 0x0300_0067),
        "no context icon quad while the context is 0xFF [M 0x19b9c8:274]");
    // Sister: the INFO bar's message sits in the pane queue while it displays (the bar geometry
    // is the render check — the HUD sheet is shared with the health pane, so runs can't pin it).
    assert!(sim.hud.info_msgs.front().is_some(), "the INFO pane has a message to display");
}

/// The second marker by the safe ("Press Action to open the control panel", label 0x010001E9
/// [D USATxt.dat entry 1489]) sits 6.7 units from the safe spot with r 3.889 — Bond must NOT
/// fire it from the safe spot (sister for the one-shot + the spatial gating).
#[test]
fn the_control_panel_hint_stays_silent_from_the_safe_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(sim.hints.iter().any(|h| h.label == 0x0100_01e9), "the control-panel marker exists [D]");
        sim.bond = Bond::new([155.19, 43.12, -113.38], 1.57);
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert_ne!(first_info_label(&scene), Some(0x0100_01e9),
        "the control-panel hint does not fire from the safe spot (its r 3.889 sphere ends 6.7 away)");
}

// ---------------------------------------------------------------------------
// (2) The context icon at the 07000005 grapple spot.
// ---------------------------------------------------------------------------

/// The main agent's grapple spot [D PCSX2 2026-10-13]: the ctx-6 icon at (24.20, 31.93, 60.01)
/// (cel 14) arms 8.1 units away — `BLData+0x95F` reads 6, the HUD shows the table's ctx-6
/// texture 0x0300006A (the standing figure [D 07000008.bin entry 53 tex 3]), and the type-6
/// icon posts the generic "Press Action" hint (0x1000291) once.
#[test]
fn the_grapple_spot_reads_context_6_and_shows_the_figure_icon() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([16.5, 29.6, 58.6], -2.01);
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.icon_ctx, 6,
        "the grapple spot reads the ctx-6 icon [D the level's icons; M ThirdIcon_Update writes the type] (got {})", sim.bond.icon_ctx);
    assert_eq!(first_info_label(&scene), Some(0x100_0291),
        "the type-6 icon posts the generic Press Action hint [M 0x196e80:39-44] (got {:?})", first_info_label(&scene));
    // The HUD draws the ctx-6 texture while the context is set.
    let f = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    assert!(run_for(&scene, &f, 0x0300_006a), "the context icon draws texture 0x0300006A [M 0x2FC168[6]]");
    // Sister: walking away clears the context and the icon fades out within ThirdIconTimer's
    // 30 ticks [M 0x19b9c8:275-282].
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([104.95, 24.96, 108.82], 0.0);
        for _ in 0..35 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.icon_ctx, 0xff, "away from the icon the context clears [M 0x1949a8:22 resets it]");
    let f = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    assert!(!run_for(&scene, &f, 0x0300_006a), "the icon faded out within the 30-tick timer");
}

// ---------------------------------------------------------------------------
// (3) Away from everything, nothing shows.
// ---------------------------------------------------------------------------

/// At the 07000005 spawn (no icon or marker within any arm region) the context stays 0xFF, no
/// INFO label queues, and the HUD draws neither the context icon nor the top bar.
#[test]
fn away_from_contexts_the_hud_shows_none() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.icon_ctx, 0xff, "no context at the spawn");
    assert!(sim.hud.info_labels.is_empty(), "no INFO label queued");
    assert!(sim.hud.info_msgs.is_empty(), "no INFO message bound");
    let f = nf_view::hud::overlay(&scene, 16.0 / 9.0);
    assert!(!run_for(&scene, &f, 0x0300_006a) && !run_for(&scene, &f, 0x0300_0066) && !run_for(&scene, &f, 0x0300_0067),
        "no context icon quad");
}
