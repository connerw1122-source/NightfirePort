//! helicopter probe 2: the 0x19 path-data blocks of the copter levels — the game's framelist table
//! (`parsemap_block_path_data` 0x1d24d0 stores each block's payload pointer; `Copter_Create`
//! 0x17d340 indexes it by the placement's extra[8] low u16). Opt-in via NF_ISO.

use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
fn probe_path_blocks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for archive in ["07000005.bin", "07000008.bin", "0700000b.bin", "07000012.bin"] {
        let data = read_archive(&iso, archive);
        let lb = nf_formats::levelbin::LevelBin::parse(&data).expect("levelbin");
        let mut ordinal = 0usize;
        eprintln!("=== {archive} ===");
        for (entry, e) in lb.entries.iter().enumerate() {
            if !matches!(e.kind, 0 | 1 | 2 | 11) { continue }
            let mdata = lb.data(&data, e);
            let Ok(mf) = nf_formats::map::MapFile::parse(mdata) else { continue };
            for blk in &mf.blocks {
                if blk.id == 0x19 {
                    let pl = blk.payload(mdata);
                    let keys = pl.len() / 28;
                    let k0: Vec<f32> = if pl.len() >= 12 {
                        (0..3).map(|i| f32::from_le_bytes(pl[i * 4..i * 4 + 4].try_into().unwrap())).collect()
                    } else { vec![] };
                    eprintln!("  0x19 ordinal {ordinal}: map entry {entry} ({}) payload {} bytes = {} keys, k0 pos = {k0:?}",
                        e.name, pl.len(), keys);
                    ordinal += 1;
                }
            }
        }
    }
}

#[test]
fn probe_0b_parts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = nf_view::load(&iso, "0700000b.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let ci = sim.drones.copters.iter().position(|c| c.placement == 328).expect("copter");
    sim.drones.channels.set(42, 1, sim.drones.frame);
    sim.bond.pos = [16.7, 55.0, 24.4];
    for _ in 0..300 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
    let c = &sim.drones.copters[ci];
    eprintln!("copter state {} t {} started {} flight {} path {:?}", c.state, c.t, c.started, c.flight_t, c.path.as_ref().map(|p| (p.count, p.dt)));
    eprintln!("part_hashes {:?}", c.part_hashes);
    eprintln!("part_radii {:?}", c.part_radii);
    for o in sim.drones.scripts.objects.list.iter().flatten() {
        if let nf_game::nis::Obj::Entity { hash, matrix, src, .. } = o {
            eprintln!("entity hash={hash:#x} src=({:#x},{}) pos={:?}", src.0, src.1, &[matrix[3][0], matrix[3][1], matrix[3][2]]);
        }
    }
}
