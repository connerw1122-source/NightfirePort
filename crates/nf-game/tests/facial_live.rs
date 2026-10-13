//! fix-facial: the facial morph decode + the AnimFacialBlend arithmetic (G-020/G-030). The skin's
//! facial count (disc +0x12 → AnimSkin+0x2e [M AnimProcessSkinData 0x1bd500]) is 22 on the
//! dialogue skins — the same count as the cut-scene scripts' face tracks, whose channels ARE the
//! morph weights [M AnimFacialBlend 0x1be648 reads the playing sequence's +0x84 frame buffer
//! straight into the object's weight array]. Opt-in via `NF_ISO`.

use nf_formats::{anim::Sequence, character, disc, iso9660::IsoImage, levelbin::LevelBin};
use nf_game::anim::{facial_blend, facial_weights, FEMALE_FACIAL_STATE_ANIMS, MALE_FACIAL_STATE_ANIMS};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &IsoImage<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let (entries, bin) = disc::files_bin_entries(iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
}

/// The dialogue skins carry facial count 22 [D disc census: the 05-prefix skins with a face],
/// matching the cut-scene face tracks' 22 channels.
#[test]
fn the_dialogue_skins_carry_the_22_morphs() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = match IsoImage::open(&iso) { Ok(i) => i, Err(e) => { eprintln!("iso: {e}"); return } };
    let Some(data) = read_archive(&img, "07000005.bin") else { eprintln!("07000005.bin missing"); return };
    let lb = match LevelBin::parse(&data) { Ok(l) => l, Err(e) => { eprintln!("levelbin: {e}"); return } };
    let skels: std::collections::HashMap<u8, u8> = lb.entries.iter().filter(|e| e.kind == 6)
        .filter_map(|e| character::parse_skeleton(lb.data(&data, e)).ok())
        .map(|s| (s.id as u8, s.bones)).collect();
    let mut counted = 0;
    let mut other = 0;
    let mut s7 = None;
    for e in lb.entries.iter().filter(|e| e.kind == 3) {
        let d = lb.data(&data, e);
        if d.len() < 0x15 { continue }
        let hash = u32::from_le_bytes(d[0..4].try_into().unwrap());
        if hash == 0x0500_0007 { s7 = Some(d[0x12]); }
        if (hash & 0xff00_0000) != 0x0500_0000 || character::parse_skin(d, |id| skels.get(&id).copied()).is_err() { continue }
        if d[0x12] == 22 { counted += 1; } else if d[0x12] != 0 { other += 1; }
    }
    // The pre-fix parse dropped +0x12 entirely (no `facial` field) — this assertion is the
    // failing-first pin for the decode.
    assert_eq!(s7, Some(22), "skin 05000007's facial count must be 22 (the dialogue morph set)");
    eprintln!("[facial] {counted} skins with facial 22, {other} with another count");
    assert!(counted >= 4, "the archive's dialogue skins share the 22-morph set (got {counted})");
    // The tables are the ELF's own data [D .data 0x2c6ea8/0x2c6ec0].
    assert_eq!(MALE_FACIAL_STATE_ANIMS[0], 0x0600_00f8);
    assert_eq!(MALE_FACIAL_STATE_ANIMS[4], 0x0600_00f3);
    assert_eq!(FEMALE_FACIAL_STATE_ANIMS[0], 0x0600_036d);
    assert_eq!(FEMALE_FACIAL_STATE_ANIMS[4], 0x0600_0370);
}

/// The 07000005 ledge cut-scene's face track (06000996/97's 22-channel pair → sequence 040007fa,
/// frames 1..0x5dc [D docs/formats/animation.md]) sampled over the first 60 ticks: the weights
/// equal the sequence's own channel evaluation (`AnimFrameCopy`'s arithmetic) and the blend
/// passes the talk slot through under a script camera (`AnimFacialBlend`'s ScriptCam branch).
#[test]
fn the_face_track_samples_to_morph_weights_over_the_dialogue() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = match IsoImage::open(&iso) { Ok(i) => i, Err(e) => { eprintln!("iso: {e}"); return } };
    let Some(anim) = read_archive(&img, "07F00005.bin") else { eprintln!("07F00005.bin missing"); return };
    let alb = match LevelBin::parse(&anim) { Ok(l) => l, Err(e) => { eprintln!("animbin: {e}"); return } };
    let seq_entry = alb.entries.iter().find(|e| e.kind == 4 && e.hash() == 0x0400_07fa).expect("the ledge face-track sequence is in 07F00005");
    let seq = match Sequence::parse(alb.data(&anim, seq_entry)) { Ok(s) => s, Err(e) => { eprintln!("seq: {e}"); return } };
    assert_eq!(seq.channels.len(), 22, "the face track carries exactly the 22 morph channels");
    // Sister: the pre-fix state had no weights path at all — the sampling must produce 22 weights
    // matching the channel evaluation at every tick (the decompile's AnimFrameCopy arithmetic).
    for t in 1..=60u32 {
        let w = facial_weights(&seq, t as f32, 22).expect("the face track samples");
        for (c, v) in w.iter().enumerate() {
            assert!((v - seq.value(c, t)).abs() <= 1e-6, "tick {t} channel {c}: {v} != {}", seq.value(c, t));
        }
    }
    // The dialogue moves the face somewhere in the track (a morph weight exceeds the blend's
    // 0.01 override threshold) — the frozen-face symptom's direct counter-evidence.
    let mut first_move = None;
    for t in 1..=seq.frames as u32 {
        let w = facial_weights(&seq, t as f32, 22).expect("samples");
        if w.iter().any(|v| v.abs() > 0.01) { first_move = Some(t); break; }
    }
    eprintln!("[facial] first tick with a weight over 0.01: {first_move:?} of {} frames", seq.frames);
    assert!(first_move.is_some(), "the face track never moves a morph — the dialogue would still be frozen");
    // The ScriptCam branch: the talk slot (0xe0000002) copies straight over the previous weights.
    let t = first_move.unwrap();
    let w = facial_weights(&seq, t as f32, 22).unwrap();
    let mut out = vec![0.5f32; 22];
    facial_blend(&mut out, None, None, Some(&w), [0, 0], true);
    for (o, s) in out.iter().zip(&w) {
        assert_eq!(*o, *s, "the ScriptCam branch must copy the talk slot straight");
    }
}

/// `AnimFacialBlend` (0x1be648)'s non-cut-scene arithmetic: slot 0 copies straight; slots 1/2
/// override only weights with |w| > 0.01; the eye bytes (+0x97/+0x98) write indices 13/14 and
/// 11/12 as value/255 and 1 − value/255, each over the same 0.01 threshold.
#[test]
fn facial_blend_matches_the_decompile_arithmetic() {
    let slot0 = vec![0.1f32; 22];
    let slot1 = vec![0.9f32; 22];
    let slot2 = vec![0.005f32; 22]; // below the override threshold — must NOT take effect
    let mut out = vec![0.75f32; 22];
    facial_blend(&mut out, Some(&slot0), Some(&slot1), Some(&slot2), [0, 0], false);
    // slot 0 copies, slot 1 overrides every entry, slot 2's 0.005 stays under 0.01. The eye
    // bytes [0, 0]: value 0 skips the closed writes (14/11), the open writes (1.0) land at 13/12.
    for (i, v) in out.iter().enumerate() {
        let want = if i == 12 || i == 13 { 1.0 } else { 0.9 };
        assert!((v - want).abs() < 1e-6, "index {i}: {v} != {want}");
    }
    // A slot-2 weight above the threshold overrides slot 1.
    let mut slot2b = vec![0.0f32; 22];
    slot2b[3] = 0.5;
    facial_blend(&mut out, Some(&slot0), Some(&slot1), Some(&slot2b), [0xff, 0xff], false);
    assert!((out[3] - 0.5).abs() < 1e-6, "slot 2's 0.5 overrides");
    assert!((out[4] - 0.9).abs() < 1e-6, "the untouched entries keep slot 1");
    // The eye bytes: 0xFF/255 = 1.0 at the closed indices (14 for +0x97, 11 for +0x98); the
    // open indices (13/12) fall to 0/255 = 0 — under the threshold, untouched.
    let mut out = vec![0.75f32; 22];
    facial_blend(&mut out, Some(&slot0), None, None, [0xff, 0xff], false);
    assert!((out[14] - 1.0).abs() < 1e-6 && (out[11] - 1.0).abs() < 1e-6, "the closed-lid weights");
    assert!((out[13] - 0.1).abs() < 1e-6 && (out[12] - 0.1).abs() < 1e-6, "the open-lid inverse is 0 — under threshold, untouched (slot 0's 0.1 stays)");
    // Half-closed: 128/255 ≈ 0.502 and 127/255 ≈ 0.498 — both over the threshold.
    let mut out = vec![0.75f32; 22];
    facial_blend(&mut out, Some(&slot0), None, None, [128, 128], false);
    assert!((out[14] - 128.0 / 255.0).abs() < 1e-6 && (out[13] - 127.0 / 255.0).abs() < 1e-6);
    assert!((out[11] - 128.0 / 255.0).abs() < 1e-6 && (out[12] - 127.0 / 255.0).abs() < 1e-6);
}
