//! Headless NF_ISO tests for subtitles-data: the FMV briefing subtitle track built from the
//! global pack's intro script (07000500.bin, `movie::briefing_subtitles`) and the NIS cut-scene
//! track wired through the real Sim (`Drones::subtitle_track`), each asserted in numbers/ids.
//!
//! Game handlers behind the assertions: `Script_TextStart` 0x1db2b8 (`Text_AddMsg(-1, 0, 4,
//! Txt_BindLabel(label), 0, time)`), the flag gate at `Text_AddMsg` 0x1b0c30 (a type-4 add with
//! id 0 passes only when `DAT_002a37c8 != 0`), the head timer in `Text_UpdateMsg` 0x1b11d8, the
//! skip flush in `Script_FFwd` 0x1dac78:28-29 (`Text_FlushAllSubtitles` 0x1b15c8), and the
//! intro-ScriptPlayer creation in `ResetMap_Load` 0x1d6360:40-99.
//!
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The briefing track is the pack's script walked on `Script_Run`'s clock: a command after
/// `Wait(t)` fires on tick t+1 (the pre-increment clock — `Script_Update` 0x1da998:33,39 adds
/// FRAME_RATE_MUL to the script and per-stream timers after `Script_Run`), the FIFO chain queues
/// same-tick texts behind each other. Parity [M]: `FRAME_RATE_MUL = 60.0/FRAME_RATE`
/// (`GS_SetRefreshRate` 0x17d2b8:15) = 1.0 at the game's 60 Hz [L ee_snapshot 0x30d0d8 = 1.0], so
/// the port's fixed-60 Hz 1 tick = +1.0 matches the game's fresh-boot rate; the game's FMV path
/// runs MUL 2.0 (GS_SetRefreshRate 30), which the port's fixed 60 Hz clock does not model [S].
/// (`Text_UpdateMsg` 0x1b11d8 advances only the displayed head).
#[test]
fn briefing_track_is_pinned_to_the_disc_script() {
    let Some(path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let track = nf_view::movie::briefing_subtitles(&path, 0x0710_0005, true).expect("07000500.bin parses")
        .expect("0x07100005 (the Exchange) has an intro row");
    let lines = track.lines();
    // The disc script's full timeline [D 07000500.bin's 0x06000961; the dump this test first printed
    // and now pins]: Wait(183) → Text(0x06000068, 1132), then the script's own Waits pace the next
    // three lines — the gaps are the script's, the FIFO only prevents overlaps.
    assert_eq!(lines, &[
        nf_game::subtitle::SubtitleLine { label: 0x0600_0068, start: 184, end: 184 + 1132 },
        nf_game::subtitle::SubtitleLine { label: 0x0600_0069, start: 1340, end: 1340 + 756 },
        nf_game::subtitle::SubtitleLine { label: 0x0600_006a, start: 2113, end: 2113 + 988 },
        nf_game::subtitle::SubtitleLine { label: 0x0600_006b, start: 3570, end: 3570 + 555 },
    ], "the four intro lines at the script's own ticks");
    assert_eq!(track.active_subtitle(183), None);
    assert_eq!(track.active_subtitle(184), Some(0x0600_0068));
    assert_eq!(track.active_subtitle(1339), None, "the script's gap between lines 1 and 2");
    assert_eq!(track.active_subtitle(1340), Some(0x0600_0069));
    assert_eq!(track.active_subtitle(4124), Some(0x0600_006b), "the last displaying tick");
    assert_eq!(track.active_subtitle(4125), None, "sister: the track ends with the script");
}

/// A movie code the game's table has no row for (0x0700004a wraps to 0x710004a [M
/// `ResetMap_LevelToLoad` 0x1d3800]; `ResetMap_Load`'s chain has no arm → no ScriptPlayer): the
/// port's query returns None without touching the disc. Sister: a code IN the table resolves.
#[test]
fn codes_without_a_table_row_build_nothing() {
    let Some(path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    assert!(nf_view::movie::briefing_subtitles(&path, 0x0710_004a, true).unwrap().is_none());
    assert!(nf_view::movie::briefing_subtitles(&path, 0x0710_0009, true).unwrap().is_some(),
        "0x07100009 (Night Shift) is in the game's chain");
}

/// Off yields none, through the real input path: with `subtitle_track.enabled` false (the fresh
/// profile's default [M profile.rs]) the Sim's op-0x1e add is dropped exactly like the game's
/// `Text_AddMsg` gate [M 0x1b0c30] — no script_msgs, no HUD line, an empty track. Sister: the ON
/// run right below queues the line.
#[test]
fn subtitles_off_drops_the_script_lines_in_the_sim() {
    let Some(path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let Ok(mut scene) = nf_view::load(&path, "07000500.bin", false) else { eprintln!("07000500.bin does not load headless — skipped"); return };
    assert!(!scene.sim.as_ref().unwrap().drones.subtitle_track.enabled, "the fresh default is Off");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.scripts.spawn_once(0x0600_0000, 0x0600_0961, [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    }
    for _ in 0..230 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.script_msgs.is_empty(), "the flag gate dropped the add (script_msgs)");
    assert!(!sim.hud.msgs.iter().any(|m| m.script), "no script line reached the HUD");
    assert!(sim.drones.subtitle_track.is_empty(), "the track is empty");
    assert_eq!(sim.drones.subtitle_track.active_subtitle(sim.drones.frame), None);
}

/// The ON path through the real Sim: the op-0x1e add lands on the track at the script tick and
/// `active_subtitle(frame)` — the per-tick query the draw code calls — returns the label from the
/// right tick. Sister: nothing at 100 ticks (before Wait(183)'s release) and with the flag off
/// (the test above).
#[test]
fn subtitles_on_queues_the_line_on_the_track_at_the_right_tick() {
    let Some(path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let Ok(mut scene) = nf_view::load(&path, "07000500.bin", false) else { eprintln!("07000500.bin does not load headless — skipped"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.subtitle_track.enabled = true;
        sim.drones.scripts.spawn_once(0x0600_0000, 0x0600_0961, [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    }
    for _ in 0..100 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.subtitle_track.is_empty(), "sister: nothing before Wait(183) releases");
        assert_eq!(sim.drones.subtitle_track.active_subtitle(sim.drones.frame), None);
    }
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let frame = sim.drones.frame;
    assert_eq!(sim.drones.subtitle_track.active_subtitle(frame), Some(0x0600_0068),
        "the line the script queued (Wait(183) → tick 184) is active at frame {frame}");
    assert!(sim.hud.msgs.iter().any(|m| m.script && (m.life - 1132.0).abs() < 1e-3),
        "the HUD drain consumed the add into a 1132-tick script line (the game's Text_UpdateMsg head)");
    // The track's line matches the push the Sim made: start = the script tick (184), end = +1132.
    let lines = sim.drones.subtitle_track.lines();
    assert_eq!(lines.len(), 1, "one add so far");
    assert_eq!((lines[0].label, lines[0].start, lines[0].end), (0x0600_0068, 184, 184 + 1132));
}
