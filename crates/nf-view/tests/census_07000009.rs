//! level-09 census: dump every 07000009 main-map placement by kind, with params. Opt-in NF_ISO; prints only.
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
#[ignore = "assertion-free census dump; run with --ignored"]
fn dump_07000009_placements_by_kind() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set"); return };
    let data = read_archive(&iso, "07000009.bin");
    let lv = Level::parse(&data).expect("parse");
    let mm = lv.main_map().unwrap_or(0);
    println!("maps {} main {mm}", lv.maps.len());
    let mut placements: Vec<(usize, &nf_formats::placement::Placement)> = Vec::new();
    for (mi, m) in lv.maps.iter().enumerate() {
        for p in &m.placements { placements.push((mi, p)); }
    }
    println!("all-map placements: {}", placements.len());
    let mut by_kind: Vec<(u32, Vec<usize>)> = Vec::new();
    for (i, (_, p)) in placements.iter().enumerate() {
        let k = p.flags & 0xff;
        match by_kind.iter_mut().find(|(kk, _)| *kk == k) {
            Some((_, v)) => v.push(i),
            None => by_kind.push((k, vec![i])),
        }
    }
    by_kind.sort();
    for (k, idxs) in &by_kind {
        println!("== KIND {k:#04x}: {} placements ==", idxs.len());
        for &i in idxs {
            let (mi, p) = placements[i];
            let target = if p.by_hash() { lv.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < lv.maps[mi].objects.len() { Some((mi, p.index as usize)) } else { None };
            let name = target.map(|(m, o)| lv.maps[m].objects[o].name.trim().to_string())
                .unwrap_or_else(|| format!("UNRESOLVED hash {:08x}", p.hashcode));
            let t = p.translation;
            let params: Vec<String> = p.params.iter().map(|(a, b)| format!("{a}:{b:#x}")).collect();
            let cel = if p.creates_cel() { " CEL" } else { "" };
            println!("m{mi} #{i} kind {k:#04x}{cel} \"{}\" @({:.2},{:.2},{:.2}) params [{}] hash {:08x} flags {:#x}",
                name, t[0], t[1], t[2], params.join(" "), p.hashcode, p.flags);
        }
    }
}

/// The disc-wide Hurt (0xE4) survey: every hurt placement's damage type / who-mask / gate across
/// the story levels — decides whether the Hurt kind port changes visible play (who 1 = the inert
/// configuration 07000009's #373 carries; who 0x10 = the player class bit that CAN fire
/// [M Hurt_Update 0x17e5d0's class-bit table]). Opt-in NF_ISO; prints only.
#[test]
#[ignore = "assertion-free disc survey; run with --ignored"]
fn survey_0xe4_hurt_placements_across_the_story_levels() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set"); return };
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let levels: &[&str] = &["07000001", "07000002", "07000003", "07000004", "07000005", "07000009",
        "07000007", "07000008", "07000009", "0700000a", "0700000b", "0700000c", "0700000d",
        "07000011", "07000012", "07000013", "07000014", "07000015", "07000016", "0700001b",
        "07000041", "0700004a", "0700004b"];
    for lv in levels {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{lv}.bin"))) else { continue };
        let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
        let Ok(level) = Level::parse(&data) else { eprintln!("{lv}: parse failed"); continue };
        let Some(mm) = level.main_map() else { continue };
        for (i, p) in level.maps[mm].placements.iter().enumerate() {
            if p.flags & 0xff != 0xe4 { continue; }
            let g = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (dtype, dmg, gate, who) = (g(0), g(1), g(2), g(4));
            // The start law [M 0x17e5d0 state 0]: gated → the channel value starts it; no gate +
            // who 0 → starts at once; no gate + who != 0 → fires only when who & collected bits
            // (8/0x10/2) — who 1 collects nothing (inert).
            let live = if gate != 0 { "GATED" } else if who == 0 { "LIVE" } else if who & 0x1a != 0 { "LIVE" } else { "inert" };
            println!("{lv} #{i} type {dtype} dmg {dmg} gate {gate:#x} who {who:#x} -> {live}");
        }
    }
}
