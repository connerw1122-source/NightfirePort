//! `weapon_data +0x14` is an **f32** auto-aim accuracy, read by `Check_AutoAim` (0x1a73f8:148) as
//! `*(f32 *)(weapon_data + w*0x10c + 0x14) * 0.01 * …` [M]. The port read it as u16 — the low 2 bytes of
//! every one of these floats are 0x0000, so `autoaim_accuracy` returned 0.0 for every weapon and the
//! auto-aim cone was always zero-width (player-port-check §1). Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::weapon::WeaponData;

fn weapon_data() -> Option<WeaponData> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    Some(WeaponData::from_action_elf(&elf).unwrap())
}

#[test]
fn autoaim_accuracy_is_the_f32_at_plus_0x14() {
    let Some(wd) = weapon_data() else { eprintln!("skip: NF_ISO not set"); return };
    // [L ee_start_05.bin @ weapon_data 0x2bf150]: record 2 (P2K) +0x14 = 0x42c80000 = 100.0; the SG5
    // (0x16) +0x14 = 0x42700000 = 60.0.
    let p2k = wd.autoaim_accuracy(2);
    assert!((p2k - 100.0).abs() < 1e-3, "P2K auto-aim accuracy = 100.0 [L], got {p2k}");
    let sg5 = wd.autoaim_accuracy(0x16);
    assert!((sg5 - 60.0).abs() < 1e-3, "SG5 auto-aim accuracy = 60.0 [L], got {sg5}");
    // Sister: the u16 reading this test replaces — the low 2 bytes at +0x14 are 0x0000 for both records,
    // so the old accessor returned 0.0 and this assertion could not have passed before the fix.
    let raw = |i: usize| u16::from_le_bytes([wd.records[i][0x14], wd.records[i][0x15]]);
    assert_eq!((raw(2), raw(0x16)), (0, 0), "the low half of the f32 is zero — the old u16 read was always 0");
    // And the accessor is the f32: zeroing the record's +0x14 zeroes it (the comparison can fail).
    let mut zeroed = wd.clone();
    zeroed.records[2][0x14..0x18].fill(0);
    assert_eq!(zeroed.autoaim_accuracy(2), 0.0);
}
