//! Opt-in (NF_ISO): 07000005's music (track 15 = PS2/MUSIC/MFX_0/MFX_1) through the stream player.

use nf_audio::{music::Player, Cmd};
use nf_formats::iso9660::IsoImage;
use nf_formats::music::{music_path, track_file, Smf};
use std::sync::Arc;

#[test]
fn castle_music_flows_jumps_and_ends() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let get = |p: &str| iso.read_file(iso.find(p).unwrap()).unwrap();
    let n = track_file(&get("PS2/MFXINFO.MXI"), 15).unwrap().unwrap();
    assert_eq!(music_path(n, "SMF"), "PS2/MUSIC/MFX_0/MFX_1.SMF");
    let smf = Smf::parse(&get(&music_path(n, "SMF"))).unwrap();
    let ssd = Arc::new(get(&music_path(n, "SSD")));
    assert_eq!((ssd.len(), smf.base_volume), (10_749_696, 99));
    let mut out = Vec::new();
    let mut p = Player::start(smf, ssd, 0, &mut out);
    // The game's level-start volume: 70 · 100 · 99 → 69 → 47 → SPU 7700 / 16383.
    assert!((p.gain(70, 100, 10000) - 7700.0 / 16383.0).abs() < 1e-4);
    let (mut played, mut queued) = (0u64, 0u64);
    let frame = |p: &mut Player, played: &mut u64, queued: &mut u64| {
        *played = (*played + 533).min(*queued);
        let mut out = Vec::new();
        p.update(*played, &mut out);
        for c in out { match c { Cmd::MusicAppend(v) => *queued += v.len() as u64, Cmd::MusicTruncate(at) => *queued = at, _ => {} } }
    };
    for c in &out { if let Cmd::MusicAppend(v) = c { queued += v.len() as u64; } }
    // 30 s: never runs dry (the read-ahead keeps a lap queued), still the start section reported.
    for _ in 0..1800 { frame(&mut p, &mut played, &mut queued); assert!(queued > played, "underrun at {played}"); }
    assert_eq!(p.now_playing, 0);
    // Combat: section 4 is instant — reported at once, the new data 14 336 samples ahead of the playhead.
    p.request(4, false);
    let before = played;
    frame(&mut p, &mut played, &mut queued);
    assert_eq!((p.now_playing, p.last_jump), (4, 4));
    assert!(queued >= before + 14_336);
    for _ in 0..600 { frame(&mut p, &mut played, &mut queued); }
    // Stealth 6 is deferred: cut at the last bar of the first read-ahead buffer after the one being written (≥ one
    // lap, 0.84 s; later when that lap has no bar), then reported.
    p.request(6, false);
    let t0 = played;
    let mut at = None;
    for _ in 0..400 { frame(&mut p, &mut played, &mut queued); if at.is_none() && p.now_playing == 6 { at = Some(played - t0); } }
    let at = at.expect("deferred jump completed");
    eprintln!("deferred jump after {:.2} s", at as f64 / 32000.0);
    assert!((26_880..=128_000).contains(&at), "{at}");
    // End sting 9 (instant, ends with a type-9 marker): the stream stops about its 4.7 s later.
    p.request(9, true);
    let t1 = played;
    let mut stop = None;
    for _ in 0..800 { frame(&mut p, &mut played, &mut queued); if stop.is_none() && p.stopped { stop = Some(played - t1); } }
    let stop = stop.expect("stopped");
    eprintln!("stopped {:.2} s after the end request", stop as f64 / 32000.0);
    // Sister: not before the sting's 4.7 s (+ the 0.448 s lead).
    assert!(stop as f64 / 32000.0 > 4.7 && (stop as f64 / 32000.0) < 6.5, "{stop}");
}
