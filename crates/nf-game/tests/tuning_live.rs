//! The port's `read_tuning_vars` against the game's own `ReadTuningVars` (0x1d3a80), run in the R5900 interpreter over
//! the user's ACTION.ELF with `GameState+12` (0x2a3768 + 12) set to each level id 0x07000000..=0x0700004F.
//! Opt-in: `NF_ISO`.

use nf_formats::ee::Machine;
use nf_formats::iso9660::IsoImage;
use nf_game::tuning::{read_tuning_vars, Tuning};

#[test]
fn tuning_matches_the_game_for_every_level() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let defaults = Tuning::from_action_elf(&elf).unwrap();
    let addrs = Tuning::addresses(&elf).unwrap();
    let game = |level: u32| {
        let mut m = Machine::from_elf(&elf).unwrap();
        m.poke(0x2a3768 + 12, &level.to_le_bytes()).unwrap();
        m.call(0x1d3a80, 0, 0, 1 << 20).unwrap();
        addrs.iter().map(|&(n, a)| (n, f32::from_le_bytes(m.read(a, 4).unwrap().try_into().unwrap()).to_bits())).collect::<Vec<_>>()
    };
    let port = |level: u32| {
        let mut t = defaults.clone();
        read_tuning_vars(&mut t, level);
        Tuning::NAMES.iter().map(|&n| (n, t.get(n).unwrap().to_bits())).collect::<Vec<_>>()
    };
    for level in 0x0700_0000..=0x0700_004f {
        assert_eq!(port(level), game(level), "level {level:08x}");
    }
    // The values the drone code relies on for 07000005 (also read in the level-start snapshot work/ee_start_05.bin).
    let mut t = defaults.clone();
    read_tuning_vars(&mut t, 0x0700_0005);
    assert_eq!([t.DroneDamage_Easy, t.DroneDamage_Normal, t.DroneDamage_Hard, t.DroneDamage_Head, t.DroneDamage_Legs], [1.5, 1.0, 0.8, 10.0, 0.75]);
    // Sister: the group values differ between levels, so a port applying the wrong group would fail.
    assert_ne!(port(0x0700_0005), game(0x0700_0009));
    assert_ne!(port(0x0700_0005), port(0x0700_0050));
}
