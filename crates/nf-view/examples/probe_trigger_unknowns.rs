//! U-032 probe: the 07000005 placements behind spawner 268 and the 0x22–0x25 / 0x44 channels.
//! Run: NF_ISO=<iso> cargo --release run -p nf-view --example probe_trigger_unknowns

use nf_formats::level::Level;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;

    let param = |p: &nf_formats::placement::Placement, k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);

    // The named placements.
    for idx in [268usize, 305, 306, 316, 318] {
        let Some(p) = ps.get(idx) else { continue };
        eprintln!("placement {idx}: flags {:#04x} at {:?} params {:?}", p.flags & 0xff, p.translation, p.params);
    }

    eprintln!("--- every trigger whose out/list touches 0x22..=0x25 / 0x44, and every 0xF1 spawner ---");
    for (idx, p) in ps.iter().enumerate() {
        let t = p.flags & 0xff;
        let params = |k: u32| param(p, k);
        let touches = |v: u32| (0x22..=0x25).contains(&v) || v == 0x44;
        match t {
            // 0xDC Trigger (type = p0), 0xEA TouchOnce, 0xEB Touch, 0xEC AND, 0xED SEQ-AND, 0xEE Out, 0xEF OR
            0xdc | 0xea | 0xeb | 0xec | 0xed | 0xee | 0xef => {
                let kind = if t == 0xdc { params(0) } else { t - 0xe9 };
                let (out, list): (u32, Vec<u32>) = match kind {
                    1 | 2 | 5 | 0xd => (params(0), (2..=8).map(|k| params(k)).collect()),
                    3 | 4 | 0xc => (params(0), (1..=8).map(|k| params(k)).collect()),
                    10 => (params(4), vec![]),
                    _ => (params(0), vec![]),
                };
                let hit = touches(out) || list.iter().any(|&c| touches(c));
                if hit || idx == 305 || idx == 306 {
                    eprintln!("trigger #{idx}: type {t:#04x} kind {kind} out {out:#04x} list {list:02x?} mask p9 {:#x} params {:?}", params(9), p.params);
                }
            }
            0xf1 => {
                // Spawner: p4 start, p5 stop, p6 done, p7 kill; report when any touches the channels.
                let gates = [params(4), params(5), params(6), params(7)];
                if gates.iter().any(|&g| touches(g)) {
                    eprintln!("spawner #{idx}: mode {} group {} start {:#04x} stop {:#04x} done {:#04x} kill {:#04x} p3 {} params {:?}",
                        params(0), params(1), params(4), params(5), params(6), params(7), params(3), p.params);
                }
            }
            _ => {}
        }
    }
    eprintln!("--- script events that write the channels (event 0xd set / 0x11 set) ---");
    // The NIS streams: scan every script of the level for Cmd::Event kinds 0xd/0x11 whose params touch the channels.
    let touches = |v: u32| (0x22..=0x25).contains(&v) || v == 0x44;
    for (h, nis) in level.nis.iter() {
        for st in nis.streams.iter() {
            for cmd in st.decode() {
                if let nf_formats::nis::Cmd::Event { kind, params } = cmd {
                    if kind == 0xd || kind == 0x11 {
                        let hit = params.iter().any(|&v| touches(v));
                        if hit { eprintln!("script {h:08x}: event {kind:#04x} params {params:02x?}"); }
                    }
                }
            }
        }
    }
}
