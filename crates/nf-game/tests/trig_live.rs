//! The port's trig (`trig.rs`) against the game's own `ATAN2_APPROX` (0x1e4528), `PS2Sinf` (0x120700) and
//! `Vec_AngleDifference` (0x1e2b78), run in the R5900 interpreter over the user's ACTION.ELF. Opt-in: `NF_ISO`.

use nf_formats::ee::Machine;
use nf_formats::iso9660::IsoImage;
use nf_game::trig;

#[test]
fn trig_matches_the_game_bit_for_bit() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let mut m = Machine::from_elf(&elf).unwrap();
    let mut state = 0x1234_5678u32;
    let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
    let (mut atan_bad, mut sin_bad, mut diff_bad) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..3000 {
        let (a, b) = ((next() - 0.5) * 200.0, (next() - 0.5) * 200.0);
        m.set_fpr(12, a);
        m.set_fpr(13, b);
        m.call(0x1e4528, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::atan2_approx(a, b).to_bits() { atan_bad.push((a, b, m.fpr(0), trig::atan2_approx(a, b))); }
        let x = (next() - 0.5) * 40.0;
        m.set_fpr(12, x);
        m.call(0x120700, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::sinf(x).to_bits() { sin_bad.push((x, m.fpr(0), trig::sinf(x))); }
        let (p, q) = ((next() - 0.5) * 20.0, (next() - 0.5) * 20.0);
        m.set_fpr(12, p);
        m.set_fpr(13, q);
        m.call(0x1e2b78, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::angle_difference(p, q).to_bits() { diff_bad.push((p, q, m.fpr(0), trig::angle_difference(p, q))); }
    }
    assert!(atan_bad.is_empty(), "ATAN2_APPROX: {} of 3000 differ, e.g. {:?}", atan_bad.len(), &atan_bad[..atan_bad.len().min(4)]);
    assert!(sin_bad.is_empty(), "PS2Sinf: {} of 3000 differ, e.g. {:?}", sin_bad.len(), &sin_bad[..sin_bad.len().min(4)]);
    assert!(diff_bad.is_empty(), "Vec_AngleDifference: {} of 3000 differ, e.g. {:?}", diff_bad.len(), &diff_bad[..diff_bad.len().min(4)]);
}
