//! Opt-in (NF_ISO): level 07000005's bank through the driver.

use nf_audio::{sfx::Driver, Cmd};
use nf_formats::iso9660::IsoImage;
use nf_formats::soundbank::{bank_path, bank_slots, Bank};

#[test]
fn level_05_bank_plays_through_the_driver() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let get = |p: &str| iso.read_file(iso.find(p).unwrap()).unwrap();
    // `Sound_Ready`: 07000005 loads bank hash 5, slot 4 in SBINFO.SBI.
    let slot = bank_slots(&get("PS2/SBINFO.SBI")).unwrap().iter().position(|&h| h == 5).unwrap();
    assert_eq!(slot, 4);
    let bank = Bank::parse(&get(&bank_path("ENGLISH", slot, "SHF")), &get(&bank_path("ENGLISH", slot, "SFX"))).unwrap();
    let sbf = get(&bank_path("ENGLISH", slot, "SBF"));
    assert_eq!((bank.samples.len(), bank.effects.len()), (252, 225));
    // Every sample decodes as a voice plays it; the 7 looped ones repeat.
    let looped = (0..252).filter(|&k| bank.decode(&sbf, k).unwrap().1.is_some()).count();
    assert_eq!(looped, 7);
    let mut d = Driver::new();
    d.load_bank(slot, bank, sbf);
    d.set_listener([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let mut out = Vec::new();
    // Bond's P2K (0x403, attached to the listener): one front + rear pair, audible.
    assert_eq!(d.start(&mut out, 0x403, Some([0.0; 3]), 1, None), 0);
    // The pickup sound (0xF5: inner 5, outer 30) 3 units to the left, then 100 units away (beyond its outer radius).
    assert_eq!(d.start(&mut out, 0xf5, Some([3.0, 0.0, 0.0]), 2, None), 0);
    let plays: Vec<(f32, f32)> = out.iter().filter_map(|c| match c { Cmd::Play { left, right, .. } => Some((*left, *right)), _ => None }).collect();
    eprintln!("{plays:?}");
    assert_eq!(plays.len(), 4, "two 3-D sounds: front and rear voices each");
    assert!(plays[0].0 > 0.0 && plays[2].0 > plays[2].1, "{plays:?}");
    let before = out.len();
    // Sister: out of range → rejected, no voice.
    assert_eq!(d.start(&mut out, 0xf5, Some([100.0, 0.0, 0.0]), 3, None), 1);
    assert_eq!(out.len(), before);
    d.update(&mut out, &[]);
    assert!(d.is_playing(1) && d.is_playing(2) && !d.is_playing(3));
}
