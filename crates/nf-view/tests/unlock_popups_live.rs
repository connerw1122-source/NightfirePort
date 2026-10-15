//! unlock-popups: the post-results unlock pages — P_NFBONUS 0x40000038, P_WINGAME 0x40000053,
//! P_CHEATMEDAL 0x4000004c.
//!
//! Evidence: the handlers (`work/decomp/ACTION.ELF/00209670_P_NFBONUS_Handler*.c`,
//! `00215100_P_WINGAME_Handler*.c`, `002086b0_P_CHEATMEDAL_Handler*.c`), the page table
//! (`002029e0_Handler_HandleMessage*.c`: 0x40000036 results, 0x40000038 bonus, 0x4000004c
//! cheat-medal, 0x40000053 win), `Menu_GetLevelBonuses` 0x1ffe98, `Menu_SetLevelBonus` 0x1ffa10,
//! `PlrStarts_ProcessRewardCounter` 0x199210, `Menu_HasMedal` 0x202580 and the disc's menu-def
//! (kind-8 entry 08000002 in FILES.BIN's 07000048.bin; PAGE −0x10 {u32 id, u32 hash, u32 aux,
//! u8 kind, u32 size}; CONTROL −0x07 {u32 name, u8, u8 type, u16 x, y, w, h, u16 f1, u32 d1,
//! u16 f2, u32 d2, u32 d3}; MSG −0x0a {u8 msg, u32 a, u32 b}) walked like `MenuManager_Create`
//! 0x21b908. ISO-gated: skips without NF_ISO.

use nf_formats::iso9660::IsoImage;
use nf_formats::levelbin::LevelBin;

fn iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_ISO")?;
    Some(IsoImage::open(path).unwrap())
}

/// The menu-def's raw bytes: the kind-8 entry `08000002` inside FILES.BIN's `07000048.bin`.
fn menu_def(iso: &IsoImage) -> Vec<u8> {
    let (entries, bin) = nf_formats::disc::files_bin_entries(iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000048.BIN")).expect("07000048.bin in FILES.BIN");
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let lb = LevelBin::parse(&data).unwrap();
    let e = lb.entries.iter().find(|e| e.kind == 8 && e.name == "08000002").expect("the 08000002 menu-def entry");
    lb.data(&data, e).to_vec()
}

/// One CONTROL of a PAGE record with the MSG/KMSG/SET stream the game replays at the enter msgs.
#[derive(Debug, Clone)]
pub struct Ctrl {
    pub name: u32,
    pub b: u8,
    pub typ: u8,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub f1: u16,
    pub d1: u32,
    pub f2: u16,
    pub d2: u32,
    pub d3: u32,
    pub msgs: Vec<(u8, u32, u32)>,
    pub strings: Vec<String>,
    /// MGRMSG (−0x0d) records on the control: (msg, a, b) — the data-driven page changes.
    pub mgr: Vec<(u8, u32, u32)>,
    /// KMSG (−0x0c): (time, u8, u32, u32).
    pub kmsg: Vec<(u32, u8, u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct Pg {
    pub id: u32,
    pub hash: u32,
    pub aux: u32,
    pub kind: u8,
    pub ctrls: Vec<Ctrl>,
}

/// Walks the menu-def's tag stream exactly like `MenuManager_Create` 0x21b908 and returns every
/// PAGE record with its CONTROLs. Per-record sizes follow the decompile's reads; PAGE bodies are
/// skipped by their size field (the decompile's own skip for non-matching files, `puStack_d0 +=
/// iVar8` — the body walk and the skip land on the same next tag).
pub fn pages(def: &[u8]) -> Vec<Pg> {
    let mut p = 0usize;
    let mut rd = |n: usize, p: &mut usize| -> u32 {
        let mut v = 0u32;
        for k in 0..n { v |= (def[*p + k] as u32) << (8 * k); }
        *p += n;
        v
    };
    let _version = rd(4, &mut p);
    let mut out = Vec::new();
    loop {
        let tag = rd(4, &mut p);
        match tag {
            0xfffffff0 => {
                let id = rd(4, &mut p);
                let hash = rd(4, &mut p);
                let aux = rd(4, &mut p);
                let kind = rd(1, &mut p) as u8;
                let size = rd(4, &mut p) as usize;
                let mut pg = Pg { id, hash, aux, kind, ctrls: Vec::new() };
                let end = p + size;
                while p < end {
                    let t2 = rd(4, &mut p);
                    match t2 {
                        0xfffffff9 => {
                            let (name, b, typ) = (rd(4, &mut p), rd(1, &mut p) as u8, rd(1, &mut p) as u8);
                            let (x, y, w, h) = (rd(2, &mut p) as u16, rd(2, &mut p) as u16, rd(2, &mut p) as u16, rd(2, &mut p) as u16);
                            let (f1, d1, f2, d2, d3) = (rd(2, &mut p) as u16, rd(4, &mut p), rd(2, &mut p) as u16, rd(4, &mut p), rd(4, &mut p));
                            pg.ctrls.push(Ctrl { name, b, typ, x, y, w, h, f1, d1, f2, d2, d3, msgs: Vec::new(), strings: Vec::new(), mgr: Vec::new(), kmsg: Vec::new() });
                        }
                        0xfffffff6 => {
                            let (msg, a, b) = (rd(1, &mut p) as u8, rd(4, &mut p), rd(4, &mut p));
                            if let Some(c) = pg.ctrls.last_mut() { c.msgs.push((msg, a, b)); }
                        }
                        0xfffffff8 => {
                            let s = def[p..p + 0x20].iter().take_while(|&&c| c != 0).map(|&c| c as char).collect::<String>();
                            if let Some(c) = pg.ctrls.last_mut() { c.strings.push(s); }
                            p += 0x20;
                        }
                        0xfffffff7 => { p += 8; }
                        0xfffffff5 => { p += 14; }
                        0xfffffff4 => {
                            let (t, m, a, b) = (rd(4, &mut p), rd(1, &mut p) as u8, rd(4, &mut p), rd(4, &mut p));
                            if let Some(c) = pg.ctrls.last_mut() { c.kmsg.push((t, m, a, b)); }
                        }
                        0xfffffff3 => {
                            let (m, a, b) = (rd(1, &mut p) as u8, rd(4, &mut p), rd(4, &mut p));
                            if let Some(c) = pg.ctrls.last_mut() { c.mgr.push((m, a, b)); }
                        }
                        0xfffffffb => { p += 32; }
                        0xfffffffc => { p += 14; }
                        0xfffffffd => { p += 2; }
                        0xfffffffe => { p += 4; }
                        other => panic!("unknown in-page tag {other:08x} in page {id:08x}"),
                    }
                }
                p = end;
                out.push(pg);
            }
            0xfffffff9 => { p += 30; }
            0xfffffff8 => { p += 0x20; }
            0xfffffff7 => { p += 8; }
            0xfffffff6 => { p += 9; }
            0xfffffff5 => { p += 14; }
            0xfffffff4 => { p += 13; }
            0xfffffff3 => { p += 9; }
            0xfffffffb => { p += 32; }
            0xfffffffc => { p += 14; }
            0xfffffffd => { p += 2; }
            0xfffffffe => { p += 4; }
            0xfffffff2 => break, // END −0x0e
            other => panic!("unknown outer tag {other:08x}"),
        }
    }
    out
}

/// Prints the three pages' full control layout (a dump aid; assertions live below).
#[test]
fn dump_unlock_pages() {
    let Some(iso) = iso() else { eprintln!("skip: no NF_ISO"); return };
    let def = menu_def(&iso);
    let all = pages(&def);
    for id in [0x4000_0038u32, 0x4000_0053, 0x4000_004c] {
        match all.iter().find(|p| p.id == id) {
            Some(pg) => {
                println!("=== PAGE {:08x} hash={:08x} aux={:08x} kind={} ===", pg.id, pg.hash, pg.aux, pg.kind);
                for c in &pg.ctrls {
                    println!(
                        "  ctrl {:08x} b={} typ={} ({},{},{},{}) f1={} d1={:08x} f2={} d2={:08x} d3={:08x} msgs={:?} mgr={:?} kmsg={:?} names={:?}",
                        c.name, c.b, c.typ, c.x, c.y, c.w, c.h, c.f1, c.d1, c.f2, c.d2, c.d3, c.msgs, c.mgr, c.kmsg, c.strings
                    );
                }
            }
            None => println!("=== PAGE {id:08x} NOT IN THE MENU-DEF ==="),
        }
    }
    // every data-driven route INTO the three pages: MGRMSG/KMSG carrying msg 0x44 with these ids
    println!("\n=== data-driven routes (MGRMSG/KMSG msg 0x44) into 0x40000038/0x40000053/0x4000004c ===");
    for pg in &all {
        for c in &pg.ctrls {
            for (m, a, b) in c.mgr.iter().copied().chain(c.kmsg.iter().map(|(t, m, a, b)| (*m, *a, *b))) {
                if m == 0x44 && matches!(a, 0x4000_0038 | 0x4000_0053 | 0x4000_004c) {
                    println!("  page {:08x} ctrl {:08x}: msg 0x44 -> {:08x} (b={b})", pg.id, c.name, a);
                }
            }
        }
    }
    println!("\ntotal pages: {}", all.len());
}
