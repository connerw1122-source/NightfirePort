//! fix-facial probe: the skin facial count (disc +0x12 → runtime AnimSkin+0x2e [M
//! AnimProcessSkinData 0x1bd500]) and where the morph targets live. `AnimObjectNew` 0x1be910
//! mallocs skin+0x2e × f32 for the weights (animData+0xb0); `psiDrawSkinObjectMatrix` 0x111828
//! passes gfx-runtime+0x14 to `SkinIt` 0x10bf30 as the target base, and `SkinIt` fetches
//! `base + (vertex_morph_index + Morph_Index[m]) * 0xc` per morph m. This probe reads the disc
//! skin-info words of skinned GFX blocks and dumps the +0xC descriptor region — decoding where
//! the targets live. Opt-in via `NF_ISO`.

use nf_formats::{character, disc, iso9660::IsoImage, levelbin::LevelBin};

#[test]
fn probe_facial_target_region() {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let iso = match IsoImage::open(&iso) { Ok(i) => i, Err(e) => { eprintln!("iso: {e}"); return } };
    let (entries, bin) = match disc::files_bin_entries(&iso) { Ok(x) => x, Err(e) => { eprintln!("FILES.BIN: {e}"); return } };
    let mut total = 0usize;
    let mut printed = 0usize;
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = match iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64) { Ok(d) => d, Err(_) => continue };
        let lb = match LevelBin::parse(&data) { Ok(l) => l, Err(_) => continue };
        let skels: std::collections::HashMap<u8, u8> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| character::parse_skeleton(lb.data(&data, e)).ok())
            .map(|s| (s.id as u8, s.bones)).collect();
        let mut dbg = (0usize, 0usize, 0usize, 0usize);
        let mut gfx_blocks: std::collections::HashMap<u32, (usize, usize)> = Default::default();
        for e in lb.entries.iter() {
            if printed >= 6 { break }
            let d = lb.data(&data, e);
            let Ok(mf) = nf_formats::map::MapFile::parse(d) else { continue };
            let mut pending: Option<(usize, usize)> = None;
            for blk in &mf.blocks {
                match blk.id {
                    nf_formats::map::id::PS2_GFX => pending = Some((blk.offset as usize, blk.size as usize)),
                    nf_formats::map::id::ENTITY_PARAMS => {
                        if blk.offset + 4 > d.len() as u32 { pending = None; continue }
                        let o = blk.offset as usize;
                        let Some(bytes) = d.get(o..o + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()) else { pending = None; continue };
                        let hash = u32::from_le_bytes(bytes);
                        let Some((off, size)) = pending.take() else { continue };
                        if printed >= 6 { break }
                        let Some(g) = d.get(off..off + size) else { continue };
                        if g.len() < 16 { continue }
                        let footer = match u32::from_le_bytes(g[4..8].try_into().unwrap()) as usize { f if f != 0x10 => f & !3, _ => continue };
                        if footer + 16 > g.len() { continue }
                        let skin_info = (u32::from_le_bytes(g[footer + 8..footer + 12].try_into().unwrap()) as usize) & !3;
                        if skin_info == 0 || skin_info + 16 > g.len() { continue }
                        let src = (u32::from_le_bytes(g[skin_info..skin_info + 4].try_into().unwrap()) as usize) & !3;
                        let src_size = u32::from_le_bytes(g[skin_info + 4..skin_info + 8].try_into().unwrap()) as usize;
                        let w3 = (u32::from_le_bytes(g[skin_info + 12..skin_info + 16].try_into().unwrap()) as usize) & !3;
                        dbg.0 += 1;
                        // The morph-vertex census of the source buffer (2 qwords per vertex; the
                        // morph index = the short at +0x1E) and the region between the skin-info
                        // +8 list's terminator and the +0xC target (the slot-bone list).
                        let nv = src_size / 32;
                        let mut morph_verts = 0usize;
                        let mut max_idx: i32 = -1;
                        for v in 0..nv {
                            let vo = src + v * 32;
                            if vo + 0x20 > g.len() { break }
                            let mi = i16::from_le_bytes(g[vo + 0x1e..vo + 0x20].try_into().unwrap()) as i32;
                            if mi != -1 { morph_verts += 1; max_idx = max_idx.max(mi); }
                        }
                        let w2 = (u32::from_le_bytes(g[skin_info + 8..skin_info + 12].try_into().unwrap()) as usize) & !3;
                        // Walk the -1-terminated u32 list at w2.
                        let mut q = w2;
                        let mut refs = 0usize;
                        while q + 4 <= g.len() && refs < 4096 {
                            let w = u32::from_le_bytes(g[q..q + 4].try_into().unwrap());
                            if w == 0xffffffff { q += 4; break }
                            refs += 1;
                            q += 4;
                        }
                        let w3 = (u32::from_le_bytes(g[skin_info + 12..skin_info + 16].try_into().unwrap()) as usize) & !3;
                        if printed < 4 && morph_verts > 0 {
                            let gap = w3.saturating_sub(q);
                            eprintln!("[facial] gfx {hash:08x}: {nv} verts, morph verts {morph_verts} (max idx {max_idx}), refs {refs}, REF-end {q:#x} -> slot bones {w3:#x} gap {gap} = {:.2} x (K*12) = {:.2} x ((K+21)*12)",
                                gap as f32 / (morph_verts as f32 * 12.0), gap as f32 / ((morph_verts + 21) as f32 * 12.0));
                        }
                        if printed < 4 {
                            let w3r = u32::from_le_bytes(g[skin_info + 12..skin_info + 16].try_into().unwrap()) as usize & !3;
                            // The +0xC target region (0x30 bytes before the skin info, measured).
                            let base = w3r;
                            let bytes: Vec<String> = (0..0x30).map(|k| format!("{:02x}", g[base + k])).collect();
                            eprintln!("[facial] gfx {hash:08x} skin_info {skin_info:#x}: +0xC target {base:#x} (skin_info-{:#x}):", skin_info - base);
                            for row in bytes.chunks(16) { eprintln!("[facial]   {}", row.join(" ")); }
                            let f: Vec<f32> = (0..4).map(|k| f32::from_le_bytes(g[base..base+12].try_into().unwrap_or_default())).collect();
                            let _ = f;
                        }
                        if src_size == 0 { continue }
                        dbg.1 += 1;
                        if w3 == 0 { continue }
                        dbg.2 += 1;
                        if w3 + 16 > g.len() { dbg.3 += 1; continue }
                        let mut desc = [0u32; 4];
                        let mut ok = true;
                        for k in 0..4 {
                            match g.get(w3 + 4 * k..w3 + 8 + 4 * k).and_then(|b| <[u8; 4]>::try_from(b).ok()) {
                                Some(b) => desc[k] = u32::from_le_bytes(b),
                                None => { ok = false; break }
                            }
                        }
                        if !ok { continue }
                        eprintln!("[facial] {} gfx {hash:08x}: src {src:#x} {} verts, +0xC {w3:#x} -> desc {:08x} {:08x} {:08x} {:08x}",
                            fe.name, src_size / 16, desc[0], desc[1], desc[2], desc[3]);
                        let p0 = (desc[0] & !3) as usize;
                        if p0 != 0 && p0 + 32 <= g.len() {
                            let mut f = [0f32; 8];
                            for k in 0..8 { if let Some(b) = g.get(p0 + 4 * k..p0 + 4 * k + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()) { f[k] = f32::from_le_bytes(b); } }
                            eprintln!("[facial]   desc[0] target floats: {:.4} {:.4} {:.4} | {:.4} {:.4} {:.4} | {:.4} {:.4}", f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7]);
                        }
                        printed += 1;
                    }
                    _ => {}
                }
            }
        }
        eprintln!("[facial] {} skinned {} with-verts {} with-w3 {} w3-oob (printed {printed})", dbg.0, dbg.1, dbg.2, dbg.3);
        // The facial skins of this archive: resolve their main meshes and census the morph data.
        for e in lb.entries.iter().filter(|e| e.kind == 3) {
            let d = lb.data(&data, e);
            if d.len() >= 0x15 && d[0x12] != 0 {
                let Ok(skin) = character::parse_skin(d, |id| skels.get(&id).copied()) else { continue };
                let hash = u32::from_le_bytes(d[0..4].try_into().unwrap());
                let main = skin.main.iter().flatten().count();
                total += main;
                if printed >= 14 || hash != 0x0500_0007 { continue }
                for mh in skin.main.iter().flatten() {
                    let Some((off, size)) = gfx_blocks.get(mh) else { eprintln!("[facial] skin {hash:08x} main {mh:08x}: no gfx block"); continue };
                    let g = &d[*off..*off + *size];
                    let footer = (u32::from_le_bytes(g[4..8].try_into().unwrap()) as usize) & !3;
                    if footer == 0x10 || footer + 16 > g.len() { continue }
                    let skin_info = (u32::from_le_bytes(g[footer + 8..footer + 12].try_into().unwrap()) as usize) & !3;
                    if skin_info == 0 || skin_info + 16 > g.len() { continue }
                    let src = (u32::from_le_bytes(g[skin_info..skin_info + 4].try_into().unwrap()) as usize) & !3;
                    let src_size = u32::from_le_bytes(g[skin_info + 4..skin_info + 8].try_into().unwrap()) as usize;
                    let w2 = (u32::from_le_bytes(g[skin_info + 8..skin_info + 12].try_into().unwrap()) as usize) & !3;
                    let w3 = (u32::from_le_bytes(g[skin_info + 12..skin_info + 16].try_into().unwrap()) as usize) & !3;
                    let nv = src_size / 32;
                    let mut morph_verts = 0usize;
                    let mut max_idx: i32 = -1;
                    let mut idx_hist: std::collections::BTreeMap<i32, usize> = Default::default();
                    for v in 0..nv {
                        let vo = src + v * 32;
                        if vo + 0x20 > g.len() { break }
                        let mi = i16::from_le_bytes(g[vo + 0x1e..vo + 0x20].try_into().unwrap()) as i32;
                        if mi != -1 { morph_verts += 1; max_idx = max_idx.max(mi); *idx_hist.entry(mi).or_default() += 1; }
                    }
                    let mut q = w2;
                    let mut refs = 0usize;
                    while q + 4 <= g.len() && refs < 8192 {
                        let w = u32::from_le_bytes(g[q..q + 4].try_into().unwrap());
                        if w == 0xffffffff { q += 4; break }
                        refs += 1; q += 4;
                    }
                    let gap = w3.saturating_sub(q);
                    eprintln!("[facial] skin {hash:08x} (facial {}) main {mh:08x}: {nv} verts, morph verts {morph_verts} (max idx {max_idx}), refs {refs}, gap {gap} = {:.2}x(K*12), {:.2}x((K+21)*12)",
                        d[0x12], gap as f32 / (morph_verts as f32 * 12.0).max(1.0), gap as f32 / ((morph_verts + 21) as f32 * 12.0).max(1.0));
                    if morph_verts > 1 {
                        // Dump the words right after the REF terminator and locate the next one.
                        let words: Vec<String> = (0..9).filter_map(|k| {
                            let o = q + 4 * k;
                            if o + 4 > g.len() { None } else { Some(format!("{:08x}", u32::from_le_bytes(g[o..o + 4].try_into().unwrap()))) }
                        }).collect();
                        eprintln!("[facial]   post-REF words at {:#x}: {}", q, words.join(" "));
                        let f: Vec<f32> = (0..6).filter_map(|k| {
                            let o = q + 4 * k;
                            if o + 4 > g.len() { None } else { Some(f32::from_le_bytes(g[o..o + 4].try_into().unwrap())) }
                        }).collect();
                        if f.len() == 6 { eprintln!("[facial]   as floats: {:.4} {:.4} {:.4} | {:.4} {:.4} {:.4}", f[0], f[1], f[2], f[3], f[4], f[5]); }
                        let mut t = q;
                        let mut span = 0usize;
                        while t + 4 <= g.len() && span < 200000 {
                            if u32::from_le_bytes(g[t..t + 4].try_into().unwrap()) == 0xffffffff { break }
                            span += 4; t += 4;
                        }
                        eprintln!("[facial]   post-REF span to next 0xffffffff: {span} bytes = {} vec3s (K*12={} (K+21)*12={})", span / 12, (morph_verts) * 12, (morph_verts + 21) * 12);
                    }
                    let mut top: Vec<_> = idx_hist.iter().take(6).collect();
                    top.sort_by_key(|(k, _)| **k);
                    for (mi, n) in top { eprintln!("[facial]   idx {mi}: {n} verts"); }
                }
                printed += 1;
            }
        }
    }
    eprintln!("[facial] {total} morph-bearing main meshes across the disc");
}
