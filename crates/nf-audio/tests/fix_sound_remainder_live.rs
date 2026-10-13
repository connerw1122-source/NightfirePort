//! Opt-in (NF_ISO): the G-032 remainder on the real bank — `PauseInNis` (SFXParameters+0x3F) pausing while a
//! cut-scene plays (`ES_HandleNisPauseSfx`, sound.md §8) and `SFXPreCache` (0x39f4: streams only, the same-id
//! take-over, the `KeyOnFrameDelay` window).

use nf_audio::{sfx::Driver, Cmd};
use nf_formats::iso9660::IsoImage;
use nf_formats::soundbank::{bank_path, bank_slots, Bank, Effect};

fn driver() -> (Driver, nf_formats::soundbank::Bank, Vec<u8>) {
    let Some(path) = std::env::var_os("NF_ISO") else { panic!("skip: NF_ISO not set") };
    let iso = IsoImage::open(&path).unwrap();
    let get = |p: &str| iso.read_file(iso.find(p).unwrap()).unwrap();
    let slot = *bank_slots(&get("PS2/SBINFO.SBI")).unwrap().iter().position(|&h| h == 5).unwrap();
    let bank = Bank::parse(&get(&bank_path("ENGLISH", slot, "SHF")), &get(&bank_path("ENGLISH", slot, "SFX"))).unwrap();
    let sbf = get(&bank_path("ENGLISH", slot, "SBF"));
    let mut d = Driver::new();
    d.load_bank(slot, bank.clone(), sbf.clone());
    d.set_listener([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    (d, bank, sbf)
}

/// A `PauseInNis` sound pauses while `NisPlaying` (pitch 0 on its voices, the item skipped — its end test and
/// counter freeze), and resumes when the cut-scene quits [M sound.md §8, `ES_HandleNisPauseSfx`].
#[test]
fn pause_in_nis_pauses_while_the_cutscene_plays() {
    let (mut d, bank, _sbf) = driver();
    // The bank's PauseInNis effects (+0x3F != 0): the menu/ambience family.
    let id = (0..bank.effects.len() as u32).find(|&id| {
        bank.effect(id).and_then(|b| Effect::parse(b).ok()).is_some_and(|e| e.pause_in_nis != 0 && e.count == 1 && e.subs[0].file >= 0)
    }).expect("the bank has PauseInNis effects") as usize;
    let mut out = Vec::new();
    assert_eq!(d.start(&mut out, id as u32, Some([1.0, 0.0, 0.0]), 1, None), 0, "{id} starts");
    d.set_nis_playing(true);
    d.update(&mut out, &[]);
    let zeros = out.iter().any(|c| matches!(c, Cmd::Set { pitch: 0, .. }));
    assert!(zeros, "the live voices get pitch 0: {out:?}");
    // Sister: a sound without the flag keeps its pitch while the NIS plays.
    let plain = (0..bank.effects.len() as u32).find(|&id| {
        bank.effect(id).and_then(|b| Effect::parse(b).ok()).is_some_and(|e| e.pause_in_nis == 0 && e.count == 1 && e.subs[0].file >= 0)
    }).expect("the bank has plain effects");
    let mut out2 = Vec::new();
    assert_eq!(d.start(&mut out2, plain, Some([1.0, 0.0, 0.0]), 2, None), 0);
    d.update(&mut out2, &[]);
    assert!(!out2.iter().any(|c| matches!(c, Cmd::Set { pitch: 0, .. })), "{out2:?}");
}

/// `SFXPreCache(id, frames)` (0x39f4): only 1-sample streams can be precached; the real start inside the
/// `frames + 5` window takes the precached item over instead of starting a second one; after the window a
/// fresh start makes a new item [M sound.md §3.2].
#[test]
fn precache_streams_only_and_dedups_the_real_start() {
    let (mut d, bank, _sbf) = driver();
    // A streamed guard line: 175 = STREAMS entry 211 (bank_live pins it).
    let mut out = Vec::new();
    d.pre_cache(&mut out, 175, 10);
    assert_eq!(d.items(), 1, "the stream is pre-set-up");
    // A non-stream cannot be precached: no item appears.
    d.pre_cache(&mut out, 0xf5, 10);
    assert_eq!(d.items(), 1, "only streams precache");
    // The real start inside the window takes the item over (no second item, no second voice).
    assert_eq!(d.start(&mut out, 175, Some([1.0, 0.0, 0.0]), 7, None), 0);
    assert_eq!(d.items(), 1);
    assert!(out.iter().any(|c| matches!(c, Cmd::Play { .. })), "the line is audible");
    // Sister: past the window (frames + 5 updates) a new start makes a new item.
    for _ in 0..16 { d.update(&mut out, &[]); }
    assert_eq!(d.start(&mut out, 175, Some([1.0, 0.0, 0.0]), 8, None), 0);
    assert_eq!(d.items(), 2);
}
