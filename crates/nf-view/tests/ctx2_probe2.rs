//! probe2: dump arbitrary level textures by hash (beyond hud_textures' fixed list).
use nf_game::player::Input;
#[test]
#[ignore = "probe: dumps disc textures to crates/ctx2-dump (gitignored); run by hand"]
fn probe2() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    std::fs::create_dir_all("../ctx2-dump").unwrap();
    // Re-open the archive for the raw texture list (scene.textures only carries the draw list).
    use nf_formats::level::Level;
    use nf_formats::iso9660::IsoImage;
    let iso_img = IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso_img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    for h in [0x0300_005bu32, 0x0300_005c, 0x0300_005d, 0x0300_005e, 0x0300_005f, 0x0300_0060, 0x0300_0061, 0x0300_0062, 0x0300_0063, 0x0300_0064, 0x0300_0065] {
        match level.maps.iter().enumerate().find_map(|(mi, m)| m.textures.iter().find(|t| t.hashcode == h).map(|t| (mi, t))) {
            Some((mi, t)) => {
                let img = nf_view::scene::TextureImage::from_texture(mi, t);
                std::fs::write(format!("../ctx2-dump/tex{:08x}.png", h), png(img.width, img.height, &img.rgba)).unwrap();
                println!("tex {:08x} {}x{} (map {mi})", h, img.width, img.height);
            }
            None => println!("tex {:08x} MISSING", h),
        }
    }
}
fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    for y in 0..h { body.push(0u8); let row = y as usize*w as usize*4; body.extend_from_slice(&rgba[row..row+w as usize*4]); }
    let mut out = vec![0x89u8,0x50,0x4e,0x47,0x0d,0x0a,0x1a,0x0a];
    let mut chunk = |tag:[u8;4], d:&[u8], out:&mut Vec<u8>| {
        out.extend_from_slice(&(d.len() as u32).to_be_bytes()); out.extend_from_slice(&tag); out.extend_from_slice(d);
        let mut table=[0u32;256];
        for (i,t) in table.iter_mut().enumerate(){let mut c=i as u32;for _ in 0..8{c=if c&1!=0{0xEDB88320^(c>>1)}else{c>>1};}*t=c;}
        let mut crc=0xFFFFFFFFu32; for b in tag.iter().chain(d.iter()){crc=table[((crc^*b as u32)&0xff) as usize]^(crc>>8);}
        out.extend_from_slice(&(crc^0xFFFFFFFF).to_be_bytes());
    };
    let mut ihdr=Vec::new(); ihdr.extend_from_slice(&w.to_be_bytes()); ihdr.extend_from_slice(&h.to_be_bytes()); ihdr.extend_from_slice(&[8,6,0,0,0]);
    chunk(*b"IHDR",&ihdr,&mut out); chunk(*b"IDAT",&zlib(&body),&mut out); chunk(*b"IEND",&[],&mut out);
    out
}
fn zlib(raw:&[u8])->Vec<u8>{
    let mut out=vec![0x78,0x01]; let mut i=0;
    while i<raw.len(){ let n=(raw.len()-i).min(65535); let last=i+n>=raw.len();
        out.push(if last{1}else{0}); out.extend_from_slice(&(n as u16).to_le_bytes()); out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&raw[i..i+n]); i+=n; }
    let (mut a,mut b)=(1u32,0u32); for &by in raw {a=(a+by as u32)%65521;b=(b+a)%65521;}
    out.extend_from_slice(&((b<<16)|a).to_be_bytes()); out
}
