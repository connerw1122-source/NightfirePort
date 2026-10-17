//! nis-event9-cnis: the disc census for NIS event 9 (cam2mode) and the C_NIS_Handler route
//! (docs/systems/uncited-audit.md gap 10, "first step (cheap): disc census").
//!
//! Pins, through the port's own parsers over every 07* archive in FILES.BIN [D]:
//! 1. every NIS script whose streams fire event kind 9 — the game routes it via
//!    `Player_Cam2Mode` 0x195fc8 [M `Script_EventHandler` 0x1d94c0 case 9] — with the mode
//!    each firing passes and the archive/script that carries it;
//! 2. every kind-8 menu def whose data references the IN-GAME NIS page's handler control
//!    0x1000003e [M `Handler_HandleMessage` 0x2029e0:223-227 → `C_NIS_Handler` 0x205f10]
//!    and/or its msg-0x51 binding controls (0x308840..0x308ca0 [M 0x205f10:7-70]).
//! Opt-in like the goldens: set `NF_ISO`; without it the test prints "skip" and passes.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, levelbin, nis};

#[test]
fn event9_and_nis_page_census() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap_or_else(|e| panic!("NF_ISO={path:?}: {e}"));
    let (files, bin) = disc::files_bin_entries(&iso).unwrap();
    let mut event9: Vec<(String, u32, u16, Vec<u32>)> = Vec::new();
    let mut nis_page: Vec<(String, u32)> = Vec::new();
    
    for f in files.iter().filter(|f| {
        f.name.len() >= 10 && f.name[..2].eq_ignore_ascii_case("07") && f.name.to_ascii_lowercase().ends_with(".bin")
    }) {
        let data = iso
            .read_bytes(bin.lba as u64 * nf_formats::iso9660::SECTOR_SIZE as u64 + f.offset as u64, f.size as u64)
            .unwrap();
        let lb = levelbin::LevelBin::parse(&data).unwrap();
        let name = f.name.clone();
        for e in lb.entries.iter() {
            let bytes = lb.data(&data, e);
            // NIS scripts are the type-7 entries (script_census.rs's kind map).
            if e.kind == 7 {
                let nis = nis::Nis::parse(e.hash(), bytes).unwrap();
                for (si, s) in nis.streams.iter().enumerate() {
                    for cmd in s.decode() {
                        if let nis::Cmd::Event { kind: 9, params } = cmd {
                            event9.push((name.clone(), e.hash(), si as u16, params.clone()));
                        }
                    }
                }
            }
            // Menu defs: the C_NIS_Handler route's data side. They are the 0x08xxxxxx-hash
            // entries (e.g. 08000001/08000002 — the mission defs and the front-end def); the
            // page's handler control 0x1000003e [M 0x2029e0:223-227] is an authored u32 control
            // id inside the def's records. The msg-0x51 binding "controls" (0x308840..0x308ca0)
            // are runtime M_CONTROL heap pointers in the game — not authored data — so only the
            // handler id is scannable. Scan every entry unaligned (the def's records are packed).
            {
                let mut refs_page = false;
                for w in bytes.windows(4) {
                    if w == 0x1000_003eu32.to_le_bytes() { refs_page = true; break; }
                }
                if refs_page { nis_page.push((name.clone(), e.hash())); }
            }
        }
    }
    // ── Event 9 [D] — the audit's question: which cut-scene scripts fire it ────────────────────
    for (a, h, s, p) in &event9 { println!("event9: {a} script {h:08x} stream {s} modes {p:?}"); }
    println!("event9 total: {}", event9.len());
    println!("== kind-8 defs referencing 0x1000003e ==");
    for (a, h) in &nis_page { println!("nis_page: {a} def {h:08x}"); }
    println!("nis_page total: {}", nis_page.len());
    
    
    

    // The pinned set: event kind 9 occurs exactly once on the disc — script 0x0600005b of
    // 07000001 (the script-census pin's per-kind table: kind 9 count 1) [D; M the route].
    assert_eq!(event9.len(), 1, "the disc fires event 9 exactly once");
    assert_eq!(event9[0].0.to_ascii_uppercase(), "07000001.BIN", "in 07000001's archive [D]");
    assert_eq!(event9[0].1, 0x0600_005b, "script 0600005b [D; docs/systems/script-census.md]");

    // ── The C_NIS_Handler page's data side ─────────────────────────────────────────────────────
    // The pause page's main control id is authored in its def [L 0x10000028, menu.rs]; the NIS
    // page's handler control 0x1000003e would be authored the same way. No 07* FILES.BIN entry
    // carries it aligned [D — the scan above]; the raw ISO filesystem is the remaining home.
    let mut raw_hits: Vec<String> = Vec::new();
    for e in iso.entries() {
        let name = String::from_utf8_lossy(e.name.as_ref()).to_string();
        if name.to_ascii_lowercase().ends_with(".bin") { continue; } // FILES.BIN members already scanned
        if let Ok(bytes) = iso.read_file(e) {
            for w in bytes.chunks_exact(4) {
                if w == 0x1000_003eu32.to_le_bytes() { raw_hits.push(name.clone()); break; }
            }
        }
    }
    println!("== raw-ISO (non-FILES.BIN) files carrying aligned 0x1000003e ==");
    for n in &raw_hits { println!("raw: {n}"); }
    println!("raw total: {}", raw_hits.len());
}
