//! zz_ss_dump (scratch — uncommitted): dump every 0xE2 SS placement's raw params (param 0 = the
//! create state, 1 = rearm, 2 = contact mask, 3 = channel, 5 = duration, 6 = spline, 7 = tail) and
//! the runtime Ss for the levels this task's machine touches. NOT a test — diagnostics with
//! eprintln; run with --nocapture.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

// Scratch diagnostics kept for the SS param citations (109n); run with --ignored --nocapture.
#[ignore]
#[test]
fn dump_ss_params() {
    use nf_formats::level::Level;
    let Some(iso) = iso() else { eprintln!("NF_ISO not set"); return };
    for level in ["0700000c", "07000012", "07000008", "07000005"] {
        let data = {
            use nf_formats::iso9660::IsoImage;
            let img = IsoImage::open(&iso).expect("open iso");
            let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{level}.bin"))).expect("archive");
            img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
        };
        let lvl = Level::parse(&data).unwrap_or_else(|_| panic!("parse {level}"));
        let m = match lvl.main_map() { Some(m) => m, None => continue };
        eprintln!("=== {level} SSs (0xE2 with keys) ===");
        for (i, p) in lvl.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0xe2 { continue; }
            let keys = (p.extra[7] & 0xffff) as usize;
            if keys == 0 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0u32, |&(_, v)| v);
            let name = {
                let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < lvl.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                target.map(|(mp, o)| lvl.maps[mp].objects[o].name.clone()).unwrap_or_default()
            };
            eprintln!(
                "#{i} name={name:?} keys={keys} rate={} params: 0={} (0x{:x}) 1={} 2={} (0x{:x}) 3={} 4={} 5={} 6={} 7={} extra7={:08x}",
                p.extra[7] >> 16,
                param(0), param(0), param(1), param(2), param(2), param(3), param(4), param(5), param(6), param(7), p.extra[7]
            );
        }
        let scene = match nf_view::load(&iso, &format!("{level}.bin"), false) { Ok(s) => s, Err(_) => continue };
        if let Some(sim) = &scene.sim {
            for (k, s) in sim.drones.sss.list.iter().enumerate() {
                eprintln!("  runtime ss[{k}] placement={} state=0x{:x} mask=0x{:x} ch={} rate={} t={} hash={:08x}",
                    s.placement, s.state, s.mask, s.channel, s.rate, s.t, s.hash);
            }
        }
    }
}
