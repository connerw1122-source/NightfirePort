//! probe_striped_box (task 102k): locate the red/yellow diagonal-striped box the port draws on
//! 07000005 (trench right, against the castle wall) with NF_NOPORTAL=1 — the driver's shot
//! `nf-view <iso> 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30 --shot a.png`.
//! Stripes like that are usually an editor/marker texture the game never draws, so:
//!   1. decode every map texture, flag red+yellow candidates, print an ASCII preview (no GPU here);
//!   2. list the objects referencing them (map, name, hashcode);
//!   3. list every placement resolving to those objects, with kind/flags and which port draw
//!      path reaches it (static cel / MISC_KINDS / 0x3a).
//! Opt-in via NF_ISO. Kept: the census test (fix_striped_box_census) reuses the classifiers.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::texture::Texture;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

/// Fraction of pixels that are "red" (r high, g low, b low) and "yellow" (r and g high, b low).
fn stripe_frac(t: &Texture) -> (f32, f32) {
    let px = t.palette_rgba8();
    let idx = t.frame(0);
    let n = idx.len() as f32;
    let (mut r, mut y) = (0usize, 0usize);
    for &i in idx {
        let c = px[i as usize];
        if c[3] < 8 { continue; }
        if c[0] >= 170 && c[1] < 90 && c[2] < 90 { r += 1; }
        else if c[0] >= 170 && c[1] >= 150 && c[2] < 90 { y += 1; }
    }
    (r as f32 / n, y as f32 / n)
}

/// Coarse ASCII preview: red → R, yellow → Y, else '.'.
fn ascii_preview(t: &Texture) -> String {
    let px = t.palette_rgba8();
    let idx = t.frame(0);
    let (w, h) = (t.width as usize, t.height as usize);
    let (cw, ch) = (32usize, 12usize);
    let mut s = String::new();
    for cy in 0..ch {
        for cx in 0..cw {
            let x = cx * w / cw;
            let y = cy * h / ch;
            let c = px[idx[y * w + x] as usize];
            s.push(if c[3] < 8 { ' ' }
                else if c[0] >= 170 && c[1] < 90 && c[2] < 90 { 'R' }
                else if c[0] >= 170 && c[1] >= 150 && c[2] < 90 { 'Y' }
                else { '.' });
        }
        s.push('\n');
    }
    s
}

const MISC_KINDS: [u8; 15] = [0x2e, 0x30, 0x34, 0x35, 0x3b, 0x71, 0xd5, 0xd8, 0xe1, 0xe3, 0xda, 0xfa, 0xfd, 0xfe, 0x4a];

/// Which port draw path reaches a placement (scene.rs's cels_only build, current master).
fn port_path(p: &nf_formats::placement::Placement) -> &'static str {
    let kind = (p.flags & 0xff) as u8;
    if kind == 0x3a { "0x3a-grapple" }
    else if MISC_KINDS.contains(&kind) { "MISC_KINDS" }
    else if p.creates_cel() { "static-cel" }
    else if matches!(kind, 0x20 | 0xe2) { "ss-addition" }
    else { "none" }
}

#[test]
fn probe_striped_textures_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set; skipping"); return };
    for archive in ["07000005.bin", "0700000c.bin"] {
        let data = read_archive(&iso, archive);
        let level = Level::parse(&data).expect("parse");
        println!("=== {archive}: {} maps ===", level.maps.len());
        for (mi, m) in level.maps.iter().enumerate() {
            // 1. striped texture candidates.
            let mut striped: Vec<usize> = Vec::new();
            for (ti, t) in m.textures.iter().enumerate() {
                let (r, y) = stripe_frac(t);
                if r >= 0.02 && y >= 0.02 {
                    striped.push(ti);
                    println!("[map {mi} {:?}] TEXTURE idx {ti} hash {:08x} {}x{} frames {} red {:.1}% yellow {:.1}%\n{}",
                        m.name, t.hashcode, t.width, t.height, t.frames, r * 100.0, y * 100.0, ascii_preview(t));
                }
            }
            if striped.is_empty() { continue; }
            // 2. objects referencing them.
            for (oi, o) in m.objects.iter().enumerate() {
                let uses: Vec<u32> = o.gfx.leaves.iter().flat_map(|l| l.batches.iter().filter_map(|b| b.texture))
                    .filter(|t| striped.contains(&(*t as usize))).collect();
                if uses.is_empty() { continue; }
                println!("  [map {mi}] OBJECT #{oi} {:?} hash {:08x} flags {:08x} centre {:?} radius {} texture-refs {uses:?}",
                    o.name, o.hashcode, o.flags, o.centre, o.radius);
            }
        }
        // 3. every placement resolving to a striped object, from every map's placement list.
        for (mi, m) in level.maps.iter().enumerate() {
            for (pi, p) in m.placements.iter().enumerate() {
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
                let Some((tmap, tobj)) = target else { continue };
                let o = &level.maps[tmap].objects[tobj];
                let uses: Vec<u32> = o.gfx.leaves.iter().flat_map(|l| l.batches.iter().filter_map(|b| b.texture))
                    .filter(|t| striped_of(&level, tmap).contains(&(*t as usize))).collect();
                if uses.is_empty() { continue; }
                println!("  PLACEMENT map {mi} #{pi} idx {} kind {:02x} hash {:08x} cel {} flags {:08x} at {:?} rot {:?} params {:?} -> obj {:?} (map {tmap} #{tobj}) path {}",
                    p.index, p.flags & 0xff, p.hashcode, p.creates_cel(), p.flags, p.translation, p.rotation, p.params, o.name, port_path(p));
            }
        }
    }
}

fn striped_of(level: &Level, map: usize) -> Vec<usize> {
    level.maps[map].textures.iter().enumerate()
        .filter(|(_, t)| { let (r, y) = stripe_frac(t); r >= 0.02 && y >= 0.02 })
        .map(|(i, _)| i).collect()
}

/// Where do the census's leftover 0x10-flagged World draws come from? The port's World sites the
/// 0x10 gate doesn't cover: the static loop's 0x20/0xe2 additions, coronas 0xfc, wires 0x3e,
/// ladders 0x22. Dump every placement of those kinds whose resolved object carries flags 0x10.
#[test]
fn probe_0x10_reachable_by_ungated_kinds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set; skipping"); return };
    for archive in ["07000001.bin", "07000003.bin", "07000012.bin", "07000013.bin", "07000005.bin"] {
        let level = Level::parse(&read_archive(&iso, archive)).expect("parse");
        println!("=== {archive} ===");
        for (mi, m) in level.maps.iter().enumerate() {
            for (pi, p) in m.placements.iter().enumerate() {
                let kind = (p.flags & 0xff) as u8;
                if p.creates_cel() { continue; }
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
                let Some((tmap, tobj)) = target else { continue };
                let o = &level.maps[tmap].objects[tobj];
                if o.flags & 0x10 == 0 { continue; }
                let has_gfx = o.gfx.leaves.iter().any(|l| !l.batches.is_empty());
                println!("  kind {kind:02x} map {mi} #{pi} at {:?} -> {:?} (map {tmap} #{tobj}) flags {:#x} gfx {has_gfx}", p.translation, o.name, o.flags);
            }
        }
    }
}

#[test]
fn probe_0x0f_nonskinned_0x10() {
    let Some(iso) = iso() else { return };
    for archive in ["07000001.bin", "07000003.bin", "07000012.bin", "07000013.bin"] {
        let level = Level::parse(&read_archive(&iso, archive)).expect("parse");
        println!("=== {archive} ===");
        for (mi, m) in level.maps.iter().enumerate() {
            for (pi, p) in m.placements.iter().enumerate() {
                if (p.flags & 0xff) != 0x0f || p.creates_cel() { continue; }
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
                let Some((tmap, tobj)) = target else { continue };
                let o = &level.maps[tmap].objects[tobj];
                if o.flags & 0x10 == 0 || !o.gfx.leaves.iter().any(|l| !l.batches.is_empty()) || o.gfx.is_skinned() { continue; }
                println!("  0f map {mi} #{pi} at {:?} -> {:?} flags {:#x} centre {:?}", p.translation, o.name, o.flags, o.centre);
            }
        }
        for h in [0x0200_0502u32, 0x0200_0505] {
            if let Some((tm, to)) = level.by_hash.get(&h) {
                let o = &level.maps[*tm].objects[*to];
                println!("  car child {h:08x}: flags {:#x} gfx {}", o.flags, o.gfx.leaves.iter().any(|l| !l.batches.is_empty()));
            }
        }
    }
}
