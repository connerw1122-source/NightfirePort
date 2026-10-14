//! fix-context-hints2 probe: the icons/channels at the two driver spots + the ctx textures as PNGs.
//! Kept runnable with --nocapture (a scratch probe, not an assertion test).

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let idle = Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    scene
}

fn png(path: &str, w: u32, h: u32, rgba: &[u8]) {
    // Minimal PNG writer: RGBA8, no filtering.
    let mut body: Vec<u8> = Vec::new();
    for y in 0..h {
        body.push(0u8); // filter none
        let row = y as usize * w as usize * 4;
        body.extend_from_slice(&rgba[row..row + w as usize * 4]);
    }
    let mut chunks: Vec<u8> = Vec::new();
    let mut chunk = |tag: [u8; 4], data: &[u8], out: &mut Vec<u8>| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&tag);
        out.extend_from_slice(data);
        let mut c = crc32fast_lite(tag.as_slice(), data);
        out.extend_from_slice(&c.to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(*b"IHDR", &ihdr, &mut chunks);
    chunk(*b"IDAT", &zlib_store(&body), &mut chunks);
    chunk(*b"IEND", &[], &mut chunks);
    std::fs::write(path, [b"\x89PNG\r\n\x1a\n".as_slice(), chunks.as_slice()].concat()).unwrap();
}

fn crc32fast_lite(tag: &[u8], data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 { c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 }; }
        *t = c;
    }
    let mut crc = 0xFFFFFFFFu32;
    for b in tag.iter().chain(data.iter()) {
        crc = table[((crc ^ *b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFFFFFF
}

fn zlib_store(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut i = 0;
    while i < raw.len() {
        let n = (raw.len() - i).min(65535);
        let last = i + n >= raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!n as u16).to_le_bytes());
        out.extend_from_slice(&raw[i..i + n]);
        i += n;
    }
    let adler: u32 = {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in raw { a = (a + byte as u32) % 65521; b = (b + a) % 65521; }
        (b << 16) | a
    };
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

#[test]
#[ignore = "probe: dumps disc textures to crates/ctx2-dump (gitignored); run by hand"]
fn probe_context_hints2() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    std::fs::create_dir_all("../ctx2-dump").unwrap();
    for (level, spots) in [
        ("07000008.bin", vec![("safe", [155.19f32, 43.12, -113.38])]),
        ("07000005.bin", vec![("grapple", [16.5f32, 29.6, 58.6])]),
    ] {
        let scene = load(&iso, level);
        let sim = scene.sim.as_ref().unwrap();
        println!("== {level} ==");
        for (name, s) in &spots {
            println!(" spot {name}: {:?}", s);
            for (i, ic) in sim.icons.iter().enumerate() {
                let d = dist(ic.centre, *s);
                if d < 40.0 {
                    println!("  icon[{i}] ctx={} on={} at ({:.2},{:.2},{:.2}) r {:.3} DIST {:.2} cels {:?}",
                        ic.ctx, ic.on_channel, ic.centre[0], ic.centre[1], ic.centre[2], ic.radius, d, sim.icon_cels[i]);
                }
            }
            for (i, h) in sim.hints.iter().enumerate() {
                let d = dist(h.centre, *s);
                if d < 40.0 {
                    println!("  hint[{i}] label={:08x} sound={} on={} off={} at ({:.2},{:.2},{:.2}) r {:.3} DIST {:.2}",
                        h.label, h.sound, h.on_channel, h.off_channel, h.centre[0], h.centre[1], h.centre[2], h.radius, d);
                }
            }
        }
        println!("  all icons: {}  all hints: {}", sim.icons.len(), sim.hints.len());
        for (i, ic) in sim.icons.iter().enumerate() {
            println!("    icon[{i}] ctx={} on={} at ({:.2},{:.2},{:.2}) r {:.3}", ic.ctx, ic.on_channel, ic.centre[0], ic.centre[1], ic.centre[2], ic.radius);
        }
        // The ctx textures 0x03000064..6a + the HUD sheet ring region.
        for h in [0x0300_0064u32, 0x0300_0065, 0x0300_0066, 0x0300_0067, 0x0300_0068, 0x0300_0069, 0x0300_006a, 0x0300_007a] {
            if let Some((_, t)) = scene.hud_textures.iter().find(|(k, _)| *k == h) {
                png(&format!("../ctx2-dump/{}_tex{:08x}.png", &level[..8], h), t.width, t.height, &t.rgba);
                println!("  tex {:08x} {}x{}", h, t.width, t.height);
            } else {
                println!("  tex {:08x} MISSING", h);
            }
        }
        // The HUD sheet's ring region (uv 102,113 85x86) upscaled view.
        if let Some((_, t)) = scene.hud_textures.iter().find(|(k, _)| *k == 0x0300_007a) {
            let (u0, v0, uw, vh) = (102usize, 113usize, 85usize, 86usize);
            let mut crop = Vec::with_capacity(uw * vh * 4);
            for y in v0..v0 + vh {
                for x in u0..u0 + uw {
                    let o = (y * t.width as usize + x) * 4;
                    crop.extend_from_slice(&t.rgba[o..o + 4]);
                }
            }
            png(&format!("../ctx2-dump/{}_ring_region.png", &level[..8]), uw as u32, vh as u32, &crop);
        }
    }
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0]-b[0]).powi(2) + (a[1]-b[1]).powi(2) + (a[2]-b[2]).powi(2)).sqrt() }
