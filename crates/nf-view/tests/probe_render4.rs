//! fix-render4 probe 4: name the objects that draw the big shadow-decal quads (tex 002f:003e-like
//! black blobs) on 07000007, list every list-1 (alpha list, flags & 1) object with its flag bits,
//! and dump the level's animated textures (disco/party lights animate).
//! Opt-in via NF_ISO.

use nf_formats::{disc, iso9660::IsoImage, level::Level};
use nf_view::scene::{Layer, Scene};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn parse_level(iso_path: &str, archive: &str) -> Level {
    let img = IsoImage::open(iso_path).expect("open ISO");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    Level::parse(&data).expect("parse level")
}

#[test]
fn probe_decal_objects_07000007() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let archive_env = std::env::var("NF_PROBE_LEVEL").unwrap_or_else(|_| "07000007.bin".into());
    let archive = archive_env.as_str();
    let scene = nf_view::load(&iso_path, archive, false).expect("load");
    let level = parse_level(&iso_path, archive);
    let (instances, _) = level.instances(false);

    // Every uniform-colour black-textured World group (shadow decals) with the nearest object names.
    let mut rows: Vec<(f32, String)> = Vec::new();
    for m in scene.materials.iter() {
        if m.layer != Layer::World { continue }
        let Some((tmi, tti)) = m.key else { continue };
        let Some(tex) = scene.textures.iter().find(|t| t.key == (tmi, tti)) else { continue };
        // Shadow-decal texture: dark RGB, alpha varying.
        let (mut s, mut s2, mut amin, mut amax) = (0u64, 0u64, 255u64, 0u64);
        let n = (tex.rgba.len() / 4).max(1) as u64;
        for px in tex.rgba.chunks(4) {
            let l = (px[0] as u64 + px[1] as u64 + px[2] as u64) / 3;
            s += l; s2 += l * l; amin = amin.min(px[3] as u64); amax = amax.max(px[3] as u64);
        }
        let mean = s / n;
        let sd = ((s2 / n).saturating_sub(mean * mean)) as f64;
        if mean > 12 || (sd.sqrt() > 2.0) || amax == amin { continue }
        let tris = &scene.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
        for &i in tris { let v = &scene.vertices[i as usize]; for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } }
        let area = ((hi[0] - lo[0]) * (hi[2] - lo[2])).max((hi[0] - lo[0]) * (hi[1] - lo[1])).max((hi[1] - lo[1]) * (hi[2] - lo[2]));
        if area < 3.0 { continue }
        // The instance whose glist contains this material: match by centre proximity.
        let (mut best, mut name) = (f32::MAX, String::new());
        for inst in &instances {
            let o = &level.maps[inst.map].objects[inst.object];
            let d = (o.centre[0] - m.centre[0]).powi(2) + (o.centre[1] - m.centre[1]).powi(2) + (o.centre[2] - m.centre[2]).powi(2);
            if d < best { best = d; name = format!("\"{}\" flags {:#x} map {}", o.name.trim(), o.flags, inst.map); }
        }
        rows.push((area, format!("area {area:8.1} at ({:8.1},{:7.1},{:8.1}) yext {:5.2} draw {:?} tex {tmi:04x}:{tti:04x} {best:.1}u from {name}", m.centre[0], m.centre[1], m.centre[2], hi[1] - lo[1], m.draw)));
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    eprintln!("=== {} dark-textured large World groups (shadow decals) ===", rows.len());
    for (_, s) in rows.iter().take(25) { eprintln!("{s}"); }

    // Animated textures (party lights).
    for (mi, map) in level.maps.iter().enumerate() {
        for t in map.textures.iter().filter(|t| t.frames > 1) {
            eprintln!("animated texture map {mi} {:04x}: {:04x}:{:04x} {}x{} {} frames rate {}", mi, mi, t.index, t.width, t.height, t.frames, t.rate);
        }
    }
}
