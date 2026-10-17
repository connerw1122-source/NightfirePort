//! The disc's own menu def (kind-8 entry 08000002 in 07000048.bin) through the port's parser
//! and keyframe runtime [D everything below comes off the ISO; the behaviour pins cite the
//! decompile as [M]]. For every page the port transcribes (crates/nf-view/src/menu.rs +
//! docs/systems/frontend.md), the cited controls' rects/labels/textures are compared against
//! the parsed data — every difference is printed and the cited ones assert.

use nf_formats::disc;
use nf_formats::levelbin;
use nf_formats::menudef::*;

/// The LevelBin entry id (kind 8 folded with hash 2 → 0x08000002) and the file hash the PAGE
/// records carry (0x80000002 — bootup's GameState+0x20 [M 0x1c7e78:24; menu.rs PAGE doc]).
const ENTRY_HASH: u32 = 0x0800_0002;
const FILE_HASH: u32 = 0x8000_0002;

fn load_menu_def() -> MenuDef {
    let iso_path = std::env::var("NF_ISO").expect("NF_ISO not set");
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).expect("open ISO");
    let (files, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let fe = files
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case("07000048.BIN"))
        .expect("07000048.bin in FILES.BIN");
    let data = iso
        .read_bytes(bin.lba as u64 * nf_formats::iso9660::SECTOR_SIZE as u64 + fe.offset as u64, fe.size as u64)
        .expect("read 07000048.bin");
    let lb = levelbin::LevelBin::parse(&data).expect("parse 07000048.bin");
    let e = lb
        .entries
        .iter()
        .find(|e| e.hash() == ENTRY_HASH)
        .expect("kind-8 menu def entry 08000002");
    let bytes = lb.data(&data, e);
    MenuDef::parse(bytes, FILE_HASH).expect("parse the menu def")
}

/// The controls the transcription cites. Control ids REPEAT within a page (0x10000001 is the
/// header AND the panel AND the hint on 0x40000025 — [D the dump]), so each citation is
/// (page, full rect, label, texture, what) matched within that page.
type Cite = (u32, (i16, i16, i16, i16), Option<u32>, Option<u32>, &'static str);

fn cited() -> Vec<Cite> {
    vec![
        // Main menu [M menu-def page 0x40000002; menu.rs rows + description]
        (0x4000_0002, (167, 262, 306, 28), Some(0x28), None, "NightFire row"),
        (0x4000_0002, (167, 294, 306, 28), Some(0x149), None, "Multiplayer row"),
        (0x4000_0002, (167, 326, 306, 28), Some(0x14b), None, "Codenames row"),
        (0x4000_0002, (103, 357, 434, 42), None, None, "description plate"),
        // Title [M menu-def page 0x40000009; menu.rs PAGE_START]. The prompt's DATA label is
        // 457 (0x1c9) [D]; menu.rs cites 0x10001eb — that id is bound at RUNTIME by
        // P_START_Handler 0x20a408:321-323, not authored in the def (a transcription note,
        // reported, not asserted against the data).
        (0x4000_0009, (131, 309, 379, 16), Some(457), None, "Press-START prompt (data label 457)"),
        (0x4000_0009, (50, 332, 540, 108), None, None, "legal plate"),
        // Select Codename [M menu-def page 0x40000025; frontend.md]
        (0x4000_0025, (50, 40, 376, 17), Some(0x1f2), None, "header"),
        (0x4000_0025, (50, 53, 590, 295), None, Some(0x0300_00b3), "panel"),
        // Select Difficulty [M menu-def page 0x40000023; frontend.md emblem 0x030000a5 + ring]
        (0x4000_0023, (50, 40, 376, 17), Some(0x1f8), None, "header"),
        (0x4000_0023, (114, 118, 150, 150), None, Some(0x0300_00a5), "difficulty emblem"),
        // Single Player [M menu-def page 0x4000001c; menu.rs PAGE_MISSION_SELECT]
        (0x4000_001c, (50, 40, 376, 17), Some(0x14c), None, "header"),
        (0x4000_001c, (114, 118, 150, 150), None, Some(0x0300_0084), "mission preview"),
        // Dossier [M menu-def page 0x4000002b; menu.rs PAGE_DOSSIER]
        (0x4000_002b, (50, 40, 376, 17), Some(0x27f), None, "header"),
        // P_LANGUAGE boot page [M menu-def page 0x40000034; frontend.md flags row + logo]
        (0x4000_0034, (282, 109, 64, 32), None, Some(0x0300_00c9), "flag 1"),
        (0x4000_0034, (434, 38, 156, 43), None, Some(0x0300_016d), "logo"),
        // Records [M menu-def PAGE 0x4000003a (menu.rs PAGE_RECORDS doc)]
        (0x4000_003a, (132, 124, 376, 16), None, None, "mission list"),
        (0x4000_003a, (435, 282, 128, 64), None, Some(0x0300_013d), "medal image"),
    ]
}

#[test]
fn menu_def_matches_the_transcription() {
    let Ok(_iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let def = load_menu_def();

    // ── Structure [D the parsed disc bytes; the 2026-10-13 census said 56 pages / 1003 controls] ──
    println!(
        "== menu-def 08000002: {} registered / {} PAGE records, {} controls ==",
        def.pages.len(),
        def.pages_seen,
        def.pages.iter().map(|p| p.controls.len()).sum::<usize>()
    );
    let known: &[u32] = &[
        0x4000_0002, 0x4000_0009, 0x4000_0023, 0x4000_0025, 0x4000_001c, 0x4000_002b,
        0x4000_0034, 0x4000_0037, 0x4000_003a, 0x4000_003b, 0x4000_0043, 0x4000_004a,
        0x4000_0047,
    ];
    for id in known {
        assert!(def.page(*id).is_some(), "page {id:#x} the port transcribes is in the def");
    }

    // ── The transcription comparison: every cited control, every difference printed ──
    let mut diffs = 0usize;
    for (page_id, rect, label, texture, what) in cited() {
        let Some(p) = def.page(page_id) else {
            println!("DIFF page {page_id:#08x} missing ({what})");
            diffs += 1;
            continue;
        };
        // Layered controls share a rect (emblem + ring both (114,118,150,150) [D]) — the cite
        // holds when ANY control at that rect carries the cited label/texture.
        let at_rect: Vec<&ControlDef> =
            p.controls.iter().filter(|c| (c.x, c.y, c.w, c.h) == rect).collect();
        if at_rect.is_empty() {
            println!("DIFF {page_id:#08x} ({what}): rect {rect:?} NOT IN THE DATA");
            diffs += 1;
            continue;
        }
        if let Some(want) = label {
            if !at_rect.iter().any(|c| c.label == Some(want)) {
                println!("DIFF {page_id:#08x} ({what}): labels {:?} vs cited {want:#x}",
                    at_rect.iter().map(|c| c.label).collect::<Vec<_>>());
                diffs += 1;
            }
        }
        if let Some(want) = texture {
            if !at_rect.iter().any(|c| c.texture == Some(want)) {
                println!("DIFF {page_id:#08x} ({what}): textures {:?} vs cited {want:#x}",
                    at_rect.iter().map(|c| c.texture).collect::<Vec<_>>());
                diffs += 1;
            }
        }
    }
    assert_eq!(diffs, 0, "{diffs} transcription difference(s) — see the DIFF lines above");
}

#[test]
fn menu_def_runtime_settles_into_the_authored_layout() {
    let Ok(_iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let def = load_menu_def();

    // ── Keyframe/message census across the whole def [D] ──
    let mut n_scripts = 0usize;
    let mut n_kfs = 0usize;
    let mut n_kmsgs = 0usize;
    let mut kmsg_ids: Vec<u32> = Vec::new();
    let mut n_bind21 = 0usize;
    for p in &def.pages {
        for c in &p.controls {
            n_scripts += c.scripts.len();
            for s in &c.scripts {
                n_kfs += s.keyframes.len();
                for k in &s.keyframes {
                    n_kmsgs += k.msgs.len();
                    for m in &k.msgs { kmsg_ids.push(m.id); }
                }
            }
            n_bind21 += c.msgs.iter().filter(|(id, _, _)| *id == 0x21).count();
        }
    }
    println!("== census: {n_scripts} scripts, {n_kfs} keyframes, {n_kmsgs} keyframe msgs \
              ({n_bind21} msg-0x21 bindings) ==");
    kmsg_ids.sort_unstable();
    kmsg_ids.dedup();
    println!("== keyframe msg ids on the disc: {:02x?} ==", kmsg_ids);

    // ── Every page runs its runtime through page-open + settle without diverging ──
    for p in &def.pages {
        let mut rt = MenuRuntime::new(p);
        rt.page_shown(0x4c);
        let mut moved: Vec<String> = Vec::new();
        for _ in 0..1200 {
            rt.tick();
        }
        for (ix, d) in rt.draw_list().iter().enumerate().skip(1) {
            let c = &p.controls[ix - 1];
            let authored = (c.x, c.y, c.w, c.h);
            if d.rect != authored {
                moved.push(format!("{:08x} authored {:?} -> settled {:?}", c.id, authored, d.rect));
            }
        }
        if !moved.is_empty() {
            println!("== page {:#08x}: {} control(s) moved by their scripts ==", p.id, moved.len());
            for m in &moved { println!("  {m}"); }
        }
        // Hard pin [M Script_RunFrame 0x220580:50-52 + 63-72]: a control with no keyframes at
        // all never animates and holds its authored rect.
        for (ix, c) in p.controls.iter().enumerate() {
            let no_kfs = c.scripts.iter().all(|s| s.keyframes.is_empty());
            if no_kfs {
                let d = &rt.draw_list()[ix + 1];
                assert_eq!(d.rect, (c.x, c.y, c.w, c.h),
                    "page {:#08x} control {:#08x} with no keyframes must hold its authored rect",
                    p.id, c.id);
            }
        }
    }

    // ── The forced control [M MenuManager_Create 0x21b908:305-314 — 0x10000224 → (340,198)] ──
    let mut seen = 0usize;
    for p in &def.pages {
        for c in &p.controls {
            if c.id == 0x1000_0224 {
                assert_eq!((c.x, c.y), (340, 198), "0x10000224 is forced to (340,198) on every page");
                seen += 1;
            }
        }
    }
    println!("== 0x10000224 forced on {seen} page(s) ==");

    // ── The full-screen fixup [M 0x21b908:305-311 — type 5 full-screen → 0x200 × 0x1c0] ──
    let mut fullscreen = 0usize;
    for p in &def.pages {
        for c in &p.controls {
            if c.ctype == 5 && c.w == 0x200 && c.h == 0x1c0 {
                fullscreen += 1;
            }
        }
    }
    println!("== full-screen fixup applied to {fullscreen} control(s) ==");
}
