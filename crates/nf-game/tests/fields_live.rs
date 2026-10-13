//! NF_ISO tests for the fields-world fixes (docs/fields/*.md):
//! - the raw 0xDC trigger param layout (no shift) — `Trigger_Create` 0x1b1720 reads out = p1,
//!   inputs = p2..p8, mask = p9, lock = p10, hide = p11 for the typed wrappers AND the raw type;
//! - the real fusebox placements (0x31) whose param 2 decides the live-wire hurt polarity
//!   (`FuseBox_Update` 0x1adfe8:99-101: hurt iff +0x56 == 0).

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};

fn iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_ISO")?;
    Some(IsoImage::open(&path).unwrap())
}

fn level_bin(iso: &IsoImage, name: &str) -> Option<Level> {
    let (entries, bin) = disc::files_bin_entries(iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Level::parse(&data).ok()
}

/// The raw 0xDC census: every level bin on the disc, the main map's 0xDC placements, and the
/// kind byte the port derives (p0, unshifted). The port's old build() shifted the raw family's
/// params by one (`*suggested*`); `Trigger_Create` 0x1b1720:61-91 disproves that — this test
/// pins the mapping on whatever the disc actually ships.
#[test]
fn raw_0dc_triggers_use_the_unshifted_param_layout() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let (entries, _bin) = disc::files_bin_entries(&iso).unwrap();
    let mut found = 0u32;
    for e in &entries {
        if !e.name.to_ascii_lowercase().ends_with(".bin") || e.name.len() != 12 { continue; }
        let Some(level) = level_bin(&iso, &e.name) else { continue };
        let Some(m) = level.main_map() else { continue };
        for p in &level.maps[m].placements {
            if p.flags & 0xff != 0xdc { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
            let kind = param(0) as u8;
            found += 1;
            eprintln!("  {} p0(kind)={} p1(out)={} p9(mask)={} p10(lock)={}", e.name, kind, param(1), param(9), param(10));
            // The port's build() must produce the same kind from p0 with no shift.
            let built = nf_game::triggers::build(&level);
            let hit = built.iter().find(|t| t.placement == level.maps[m].placements.iter().position(|q| std::ptr::eq(q, p)).unwrap_or(usize::MAX));
            if let Some(t) = hit {
                assert_eq!(t.kind, kind, "raw 0xDC: the port's kind must be unshifted param 0");
            }
        }
    }
    eprintln!("raw 0xDC placements on the disc: {found}");
}

/// 07000014's fuseboxes (0x31 with a script hash in param 0): the placement params feed
/// `FuseBox_Create`'s +0x54/+0x56/+0x5C/+0x5E. The live-wire hurt fires on a group-1 wire death
/// iff +0x56 (param 2) == 0 [M 0x1adfe8:99-101] — the level's own params decide which way the
/// polarity bites.
#[test]
fn fusebox_07000014_param2_decides_the_live_wire_hurt() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let Some(level) = level_bin(&iso, "07000014.bin") else { eprintln!("skip: 07000014.bin not on the disc"); return };
    let m = level.main_map().unwrap();
    let (boxes, _spheres) = nf_game::fusebox::FuseBox::create(&level.maps[m].placements, |p| Some((p.translation, 1.0)));
    eprintln!("07000014 fuseboxes: {} (params [p1,p2,p3,p4])", boxes.len());
    for b in &boxes {
        eprintln!("  placement {} ch = {:?}", b.placement, b.ch);
        // Sister: the port's ch words are the placement params 1-4 (the game's +0x54/+0x56/+0x5C/+0x5E).
        let p = &level.maps[m].placements[b.placement];
        let par = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        assert_eq!(b.ch, [par(1) as u16, par(2) as u16, par(3) as u16, par(4) as u16]);
    }
    if let Some(b) = boxes.iter().find(|b| b.ch[1] == 0) {
        // A box with +0x56 == 0: cutting its live wire MUST hurt (the fixed polarity).
        let mut b2 = nf_game::fusebox::FuseBox {
            placement: b.placement, pos: b.pos, ch: b.ch, wires: [false, true],
            opened: true, open_t: 0, gone: false,
        };
        let mut ch = nf_game::triggers::Channels::default();
        let hurt = b2.player_use([0.0; 3], [0.0; 3], &mut ch, 0);
        assert_eq!(hurt, nf_game::fusebox::LIVE_WIRE_HURT, "param 2 unset → the live wire hurts");
    }
}
