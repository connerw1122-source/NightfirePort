//! The playtest crash 1015-2142 (the attract trailer's return and the dossier page both died in
//! nf-audio `Driver::update`, `fx.subs[cued[k]]`: "the len is 0 but the index is 0"). The front
//! end restarts its music pair (0x1d7 / 0x470) over the live one; the per-id voice limit rejects
//! the new start in `SFXSetup`, and the rejected item stayed in the list with an empty cue list
//! until the next update indexed it. A rejected start now leaves nothing behind.
//!
//! Opt-in via `NF_ISO`; silent (NF_SILENT — no output device).

fn scene() -> Option<nf_view::scene::Scene> {
    // SAFETY: set before any audio thread starts; every test in this binary sets the same value.
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = std::env::var("NF_ISO").ok()?;
    Some(nf_view::load(&iso, "07000048.bin", false).expect("the front-end map loads"))
}

#[test]
fn restarting_the_menu_music_pair_never_panics_and_leaves_one_item_each() {
    let Some(mut scene) = scene() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sound = nf_view::build_sound(&mut scene).expect("the front-end map has a bank");
    let all: Vec<u32> = (0..nf_audio::sfx::HW_VOICES as u32).collect();
    let mut out = Vec::new();
    for id in [0x1d7u32, 0x470] {
        assert_eq!(sound.driver.start(&mut out, id, None, 0, None), 0, "the first start of {id:#x} is accepted");
    }
    let live = sound.driver.ids();
    for round in 0..4 {
        for id in [0x1d7u32, 0x470] {
            // The restart over the live item: the per-id limit rejects it (start returns non-zero).
            assert_ne!(sound.driver.start(&mut out, id, None, 0, None), 0, "round {round}: the restart of {id:#x} is rejected");
        }
        for f in 0..300 {
            let mut o = Vec::new();
            sound.driver.update(&mut o, if f % 3 == 0 { &all } else { &[] });
        }
        assert_eq!(sound.driver.ids(), live, "round {round}: the rejected starts left nothing behind");
    }
    // Sister: the list really holds the pair (an empty list would pass the comparison trivially).
    assert_eq!(live.len(), 2, "the two music items are live");
}
