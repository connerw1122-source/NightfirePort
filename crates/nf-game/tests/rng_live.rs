//! The port's `Rng` against the game's own `Rand_FRand` (0x1e3780) and `Rand_Rand` (0x1e3738), run in the R5900
//! interpreter over the user's ACTION.ELF from the same `SEED_X` / `SEED_Y` (0x30d0a0 / 0x30d0a4). Opt-in: `NF_ISO`.

use nf_formats::ee::Machine;
use nf_formats::iso9660::IsoImage;
use nf_game::drone::Rng;

#[test]
fn rng_matches_the_game_bit_for_bit() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let mut m = Machine::from_elf(&elf).unwrap();
    for (x, y) in [(0x0659_5adbu32, 0x208a_5de8u32), (0xffff_ffff, 0x8000_0001), (1, 2)] {
        m.poke(0x30d0a0, &x.to_le_bytes()).unwrap();
        m.poke(0x30d0a4, &y.to_le_bytes()).unwrap();
        let mut ours = Rng { x, y };
        for k in 0..2000u32 {
            if k % 3 == 0 {
                m.call(0x1e3738, 37, 0, 1 << 40).unwrap();
                assert_eq!(m.gpr(2), ours.rand(37), "Rand_Rand(37), call {k}");
            } else {
                let max = 1.0 + (k % 7) as f32 * 13.37;
                m.set_fpr(12, max);
                m.call(0x1e3780, 0, 0, 1 << 40).unwrap();
                let mine = ours.frand(max);
                assert_eq!(m.fpr(0).to_bits(), mine.to_bits(), "Rand_FRand({max}), call {k}: game {} port {mine}", m.fpr(0));
            }
        }
        let seeds = (u32::from_le_bytes(m.read(0x30d0a0, 4).unwrap().try_into().unwrap()), u32::from_le_bytes(m.read(0x30d0a4, 4).unwrap().try_into().unwrap()));
        assert_eq!(seeds, (ours.x, ours.y));
    }
    // Sister: a different multiplier would not match.
    let mut wrong = Rng { x: 1, y: 2 };
    wrong.x = 18001u32.wrapping_mul(wrong.x & 0xffff).wrapping_add(wrong.x >> 16);
    assert_ne!(wrong.x, { let mut r = Rng { x: 1, y: 2 }; r.frand(1.0); r.x });
}
