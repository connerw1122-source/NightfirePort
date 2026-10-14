//! fix-window-view: dump the sky-dome textures of 07000007 (map47) to BMPs for inspection.
//! Writes OUTSIDE the repo (game bytes are never committed).

use nf_formats::disc;
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn save_bmp(path: &str, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let row = (w * 3 + 3) / 4 * 4;
    let size = 54 + row * h;
    let mut b = Vec::with_capacity(size as usize);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&[0u8; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&w.to_le_bytes());
    b.extend_from_slice(&h.to_le_bytes());
    b.extend_from_slice(&[1u8, 0, 24, 0]);
    b.extend_from_slice(&[0u8; 4]); // BI_RGB
    b.extend_from_slice(&(row * h).to_le_bytes());
    b.extend_from_slice(&[0u8; 16]);
    for y in (0..h).rev() {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            b.push(rgba[i + 2]);
            b.push(rgba[i + 1]);
            b.push(rgba[i]);
        }
        while b.len() as u32 % 4 != 0 { b.push(0); }
    }
    std::fs::write(path, b).map_err(|e| e.to_string())
}

#[test]
fn dump_sky_textures() -> Result<(), String> {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set; skip"); return Ok(()) };
    let out_dir = match std::env::var("NF_TEX_OUT") { Ok(d) => d, Err(_) => { eprintln!("NF_TEX_OUT not set; skip"); return Ok(()) } };
    let img = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    let lv = Level::parse(&data).expect("level parse");
    let m = &lv.maps[lv.main_map().unwrap_or(0)];
    for ti in [11usize, 34, 177, 178, 179, 180, 181, 182, 183, 184] {
        let t = &m.textures[ti];
        let w = t.width as u32; let h = t.height as u32;
        let pal = t.palette_rgba8();
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for &i in t.frame(0) { rgba.extend_from_slice(&pal[i as usize]); }
        let path = format!("{out_dir}/tex{ti}.bmp");
        save_bmp(&path, w, h, &rgba).map_err(|e| e.to_string())?;
        println!("tex{ti} {}x{} -> {path}", t.width, t.height);
    }
    Ok(())
}
