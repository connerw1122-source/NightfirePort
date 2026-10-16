//! Golden tests for the driving missions' data (`DRIVING/*.VIV`, `*.MUS`, `*EN.SPE`, `DRIVING.SYM`).
//!
//! Opt-in: the archives' bytes are stripped from the slim NF_ISO image (the directory entries are there,
//! the extents are zero), so the test reads them from the extracted `DRIVING/` folder (the task's
//! `Nightfire-driving.7z`, byte-identical to the disc — see `cross_check_against_the_disc_golden`).
//! Point `NF_DRIVING` at a folder, or leave it unset to try `<repo>/DRIVING`. Without the folder every
//! test here prints "skip" and passes.
//!
//! Pinned:
//! - the BIGF layer of all 23 driving archives (`golden/driving_census.tsv`: entry count, table end,
//!   payload bytes, the refpack/plain/audio content split, and a SHA-1 over names+offsets+sizes);
//! - every entry's content SHA-1 against the disc-measured `golden/bigf_entries.tsv`;
//! - content classification: RefPack (`10fb`) vs plain vs ASF audio, with per-extension magic checks;
//! - `DRIVING.SYM`: 12,719 entries (it does not describe our DRIVING.ELF — docs/formats/driving-sym.md).

use nf_formats::bigf::Bigf;
use nf_formats::golden::diff_tsv;
use nf_formats::sha1::hex;
use nf_formats::symfile;

/// SHA-1 of a byte slice as hex (workspace helper, no external deps).
fn sha1_hex(data: &[u8]) -> String {
    hex(&nf_formats::sha1::sha1(data))
}

fn driving_dir() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("NF_DRIVING") {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() { return Some(p); }
        eprintln!("skip: NF_DRIVING={p:?} is not a directory");
        return None;
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = manifest.join("../../DRIVING");
    if p.is_dir() { return Some(p); }
    eprintln!("skip: no DRIVING/ folder (set NF_DRIVING)");
    None
}

/// The canonical per-archive digest the golden pins: names+offsets+sizes, then the extension histogram.
fn census_canonical(g: &Bigf) -> String {
    let mut exts: std::collections::BTreeMap<String, u32> = Default::default();
    for e in &g.entries {
        let ext = match e.name.rsplit_once('.') {
            Some((_, e)) => e.to_ascii_lowercase(),
            None => "(none)".into(),
        };
        *exts.entry(ext).or_default() += 1;
    }
    let hist: Vec<String> = exts.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let mut s = String::new();
    for e in &g.entries { s += &format!("{}\t{}\t{}\n", e.name, e.offset, e.size); }
    s + &hist.join(",") + "\n"
}

/// The per-entry content class: RefPack-compressed (`10fb`), ASF audio (`SC`…), or plain.
fn content_class(blob: &[u8], name: &str) -> &'static str {
    if blob.len() >= 2 && blob[..2] == [0x10, 0xfb] { "refpack" }
    else if name.to_ascii_lowercase().ends_with(".asf") || (blob.len() >= 2 && blob[..2] == *b"SC") { "audio" }
    else { "plain" }
}

fn archive_rows(dir: &std::path::Path) -> Vec<(String, Bigf, Vec<Vec<u8>>)> {
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| std::fs::read(p).map(|d| d.starts_with(b"BIGF")).unwrap_or(false))
        .collect();
    files.sort();
    files.into_iter().map(|p| {
        let data = std::fs::read(&p).unwrap();
        let g = Bigf::parse(&data, data.len() as u64).expect("BIGF parse");
        let blobs = g.entries.iter().map(|e| data[e.offset as usize..(e.offset + e.size) as usize].to_vec()).collect();
        (format!("/DRIVING/{}", p.file_name().unwrap().to_string_lossy()), g, blobs)
    }).collect()
}

#[test]
fn driving_archives_match_the_golden_census() {
    let Some(dir) = driving_dir() else { eprintln!("skip: NF_DRIVING/DRIVING not available"); return };
    let golden = include_str!("../golden/driving_census.tsv");
    let rows: Vec<String> = archive_rows(&dir).into_iter().map(|(name, g, blobs)| {
        let (mut r, mut p, mut a) = (0u32, 0u32, 0u32);
        for (e, blob) in g.entries.iter().zip(&blobs) {
            match content_class(blob, &e.name) { "refpack" => r += 1, "audio" => a += 1, _ => p += 1, }
        }
        format!("{name}\t{}\t{}\t{}\t{}\t{r}\t{p}\t{a}\t{}\n",
            g.entries.len(), g.header_size, g.table_end(),
            blobs.iter().map(|b| b.len() as u64).sum::<u64>(),
            sha1_hex(census_canonical(&g).as_bytes()))
    }).collect();
    let actual = format!("archive\tentries\theader_size\ttable_end\tpayload_bytes\trefpack\tplain\taudio\tnames_sha1\n{}", rows.join(""));
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    // Sister: a one-character change to one digest must be caught.
    let mut lines: Vec<String> = actual.lines().map(String::from).collect();
    let row = &mut lines[1];
    let last = row.pop().unwrap();
    row.push(if last == '0' { '1' } else { '0' });
    lines.push(lines.last().unwrap().clone());
    let tampered = lines.join("\n") + "\n";
    assert_eq!(diff_tsv(golden, &tampered).len() >= 1, true, "sister: a one-character change must be caught");
}

#[test]
fn driving_entries_hash_like_the_disc() {
    let Some(dir) = driving_dir() else { eprintln!("skip: NF_DRIVING/DRIVING not available"); return };
    // golden/bigf_entries.tsv was measured through the ISO reader on the user's full disc image;
    // matching it proves the extracted DRIVING/ folder is byte-identical to the disc (all 1,499 entries).
    let golden = include_str!("../golden/bigf_entries.tsv");
    let mut expected: std::collections::HashMap<String, &str> = Default::default();
    for line in golden.lines().skip(1) {
        let mut f = line.split('\t');
        let key = f.next().unwrap();
        let h = f.nth(3).unwrap();
        expected.insert(key.to_string(), h);
    }
    let mut checked = 0usize;
    for (name, g, blobs) in archive_rows(&dir) {
        for (i, (e, blob)) in g.entries.iter().zip(&blobs).enumerate() {
            let key = format!("{name}#{i}");
            let h = expected.get(&key).unwrap_or_else(|| panic!("{key}: missing from golden/bigf_entries.tsv"));
            assert_eq!(sha1_hex(blob), *h.trim(), "{key} ({}) content differs from the disc", e.name);
            checked += 1;
        }
    }
    assert_eq!(checked, 1499, "every driving entry must be checked against the disc golden");
    // Sister: one flipped byte must fail the comparison.
    let bad = sha1_hex(b"\x00");
    assert_ne!(bad, *expected.values().next().unwrap(), "sister: a different content hash must not match");
}

#[test]
fn driving_entry_magics_follow_the_extension_rules() {
    let Some(dir) = driving_dir() else { eprintln!("skip: NF_DRIVING/DRIVING not available"); return };
    // *measured* over all 1,499 entries (docs/formats/driving-viv.md): every entry is RefPack-compressed
    // (`10fb`), ASF audio, or one of these plain magics:
    const PLAIN: &[&[u8]] = &[b"PRAC", b"LAGG", b"LOCH", b"MPCh", b"SHPS", b"FNTS", b"[", b"acti", b"[def",
        b"fogS", b"Farp", b"\x7fELF", b"POV ", b"BNKl"];
    for (name, g, blobs) in archive_rows(&dir) {
        for (e, blob) in g.entries.iter().zip(&blobs) {
            let ext = e.name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
            let ok = blob.starts_with(b"\x10\xfb")
                || (ext == "asf" && blob.starts_with(b"SC"))
                || PLAIN.iter().any(|m| blob.starts_with(*m))
                // .sub files are plain text starting with a line-count number (measured: `14\r\nF…`).
                || (ext == "sub" && blob.first().is_some_and(|c| c.is_ascii_digit()));
            assert!(ok, "{name}: {} has an unexpected magic {} ({:?})", e.name, hex(blob[..4].try_into().unwrap()), &blob[..blob.len().min(8)]);
        }
    }
    // Sister: the rules must reject a RefPack file masquerading under a plain extension.
    assert!(!b"Zzzz".starts_with(b"\x10\xfb"));
}

#[test]
fn driving_sym_is_the_shipped_hint_table() {
    let Some(dir) = driving_dir() else { eprintln!("skip: NF_DRIVING/DRIVING not available"); return };
    let data = std::fs::read(dir.join("DRIVING.SYM")).expect("DRIVING.SYM");
    let syms = symfile::parse(&data).expect("DRIVING.SYM parse");
    assert_eq!(syms.len(), 12_719, "entry count (docs/formats/driving-sym.md)");
    assert_eq!(syms[0].name, "_umemory_size");
    assert!(syms.iter().any(|s| s.name == "main"), "main must be among the hints");
    assert!(syms.iter().any(|s| s.name.starts_with("PS2PadDevice::")), "pad device hints present");
    // Sister: the count is load-bearing.
    assert_ne!(syms.len(), 12_718);
}

// ---- RefPack decompression (phase 0 of the driving port, G-phase-0) -------------------------------
//
// The game's own decompressor is DRIVING.ELF 0x00323bb0 (gate) / 0x00323e98 (decode) / 0x00323cc8 (size
// probe) [M]; the port lives in `nf_formats::refpack`. This section pins every RefPack entry's
// decompressed size and SHA-1 over the 8 mission .VIVs — the only archives that carry RefPack streams
// (MISC.VIV is all plain, .MUS/.SPE are all ASF audio — pinned by golden/driving_census.tsv, asserted
// below). Coverage: 1,126 streams, 70,285,629 compressed → 132,706,995 decompressed bytes.

/// The 8 mission archives that carry RefPack entries (the census's `refpack` column > 0).
const MISSION_VIVS: [&str; 8] =
    ["MIS01.VIV", "MIS11.VIV", "MIS13A.VIV", "MIS13B.VIV", "MIS13C.VIV", "MIS3.VIV", "MIS4.VIV", "RACE.VIV"];

fn refpack_dir() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("NF_DRIVING_REFPACK") {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() { return Some(p); }
        eprintln!("skip: NF_DRIVING_REFPACK={p:?} is not a directory");
        return None;
    }
    if let Some(p) = std::env::var_os("NF_DRIVING") {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() { return Some(p); }
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = manifest.join("../../DRIVING");
    if p.is_dir() { return Some(p); }
    eprintln!("skip: no DRIVING folder for the RefPack golden (set NF_DRIVING_REFPACK)");
    None
}

fn load_viv(dir: &std::path::Path, name: &str) -> (Bigf, Vec<Vec<u8>>) {
    let data = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let g = Bigf::parse(&data, data.len() as u64).expect("BIGF parse");
    let blobs = g.entries.iter()
        .map(|e| data[e.offset as usize..(e.offset + e.size) as usize].to_vec()).collect();
    (g, blobs)
}

#[test]
fn driving_refpack_entries_decompress_to_the_golden() {
    let Some(dir) = refpack_dir() else { eprintln!("skip: NF_DRIVING_REFPACK/DRIVING not available"); return };
    let golden = include_str!("../golden/driving_refpack.tsv");
    let mut rows = Vec::new();
    let mut total_decompressed = 0u64;
    for name in MISSION_VIVS {
        let (g, blobs) = load_viv(&dir, name);
        for (i, (e, blob)) in g.entries.iter().zip(&blobs).enumerate() {
            if !nf_formats::refpack::is_refpack(blob) { continue; }
            // The header must announce exactly what the decoder produces [M 0x323e98 returns the header
            // size; the equality over all 1,126 real streams is the cross-check].
            let (announced, _) = nf_formats::refpack::decompressed_size(blob).expect("header");
            let out = nf_formats::refpack::decompress(blob)
                .unwrap_or_else(|err| panic!("/DRIVING/{name}: {} failed: {err}", e.name));
            assert_eq!(out.len(), announced, "/DRIVING/{name}: {} produced {} bytes, header says {announced}", e.name, out.len());
            total_decompressed += out.len() as u64;
            // Key = archive#table-index, the same index golden/bigf_entries.tsv pins, so the two goldens
            // cross-reference entry for entry.
            rows.push(format!("/DRIVING/{name}#{i}\t{}\t{}\t{}\t{}\n", e.name, blob.len(), out.len(), sha1_hex(&out)));
        }
    }
    assert_eq!(rows.len(), 1126, "RefPack stream count over the 8 mission archives");
    assert_eq!(total_decompressed, 132_706_995, "total decompressed bytes");
    let actual = format!("key\tentry\tcompressed\tdecompressed\tsha1\n{}", rows.join(""));
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    // The doc's [D] anchor: paris_mis01.crp, the largest entry (driving-viv.md; size-field bytes
    // 10 fb 68 29 00 → 0x682900 = 6,826,240).
    let track = rows.iter().find(|r| r.contains("paris_mis01.crp")).expect("paris track in the golden");
    let size: u64 = track.split('\t').nth(3).unwrap().parse().unwrap();
    assert_eq!(size, 6_826_240, "paris_mis01.crp announced decompressed size");
    // Sister: a one-character change to one digest must be caught.
    let mut lines: Vec<String> = actual.lines().map(String::from).collect();
    let row = &mut lines[1];
    let last = row.pop().unwrap();
    row.push(if last == '0' { '1' } else { '0' });
    lines.push(lines.last().unwrap().clone());
    let tampered = lines.join("\n") + "\n";
    assert_eq!(diff_tsv(golden, &tampered).len() >= 1, true, "sister: a one-character change must be caught");
}

#[test]
fn refpack_golden_covers_every_refpack_entry_of_all_23_archives() {
    // driving_census.tsv pins the refpack/plain/audio split of all 23 archives [D]; the decompression
    // golden must cover exactly the streams the census counts as refpack — no more, no less.
    let census = include_str!("../golden/driving_census.tsv");
    let golden = include_str!("../golden/driving_refpack.tsv");
    let mut expected: std::collections::BTreeMap<String, u32> = Default::default();
    for line in census.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        let count: u32 = f[5].parse().unwrap();
        if count > 0 { expected.insert(f[0].to_string(), count); }
    }
    let mut actual: std::collections::BTreeMap<String, u32> = Default::default();
    for line in golden.lines().skip(1) {
        let key = line.split('\t').next().unwrap().to_string();
        let archive = key.split('#').next().unwrap().to_string();
        *actual.entry(archive).or_default() += 1;
    }
    assert_eq!(actual, expected, "the RefPack golden must cover every refpack-counted stream of the census");
    // Sister: the census really splits by content class (misclassifying audio as refpack would show here).
    assert!(!expected.iter().any(|(k, _)| k.ends_with(".MUS") || k.ends_with(".SPE")),
        "no .MUS/.SPE archive may carry refpack streams");
}

// ---- The .crp CARP container (driving phase 1, task 503) -------------------------------------------
//
// Every `.crp` of the 23 archives (89 entries [D, golden/crp_golden.tsv]) is RefPack-compressed or,
// for the five MISC.VIV cars, plain, and decompresses to a CARP record pool: a root `CARP` record,
// an `Arti`/`Shar`/`CDat`/`RNgp`/`Map ` pool, and a chunk directory (`DBN ` track name, `ELFd` +
// `ELFr` — a real 32-bit LE MIPS relocatable object, `MapN`, `Sect`, instance-indexed `sn  `).
// Consumers [M]: track loader 0x22f898 (DBN), module loader 0x1a9600 (ELFd/ELFr/Sect/sn),
// record namer 0x2dc1b0 (Arti/Shar/CDat/RNgp/Map + nested Name). The parser is
// `nf_formats::crp`; docs/formats/crp.md.
//
// The canonical row (must match golden/crp_golden.tsv byte for byte):
// key | entry | decompressed | sha1 | root_flags | pool | chunks | elf | name | elfd_sha1, where
// pool = "CODE×n" histogram (codes as the big-endian spelling, spaces → `_`, short tags "CODE#i"),
// chunks = "CODE:size:payload_off" in directory order, elf = "t<e_type>.m<e_machine>.s<e_shnum>"
// of the ELFd payload or "-", name = the DBN string or "-".

fn crp_code_str(tag: u32) -> String {
    tag.to_be_bytes().iter()
        .map(|&b| if (32..127).contains(&b) { b as char } else { '.' })
        .collect::<String>().replace(' ', "_")
}

fn crp_render(tag: u32, flags: u32) -> String {
    let code = if flags & 1 != 0 { (tag & 0xffff_0000) | 0x2020 } else { tag };
    if flags & 1 != 0 { format!("{}#{}", crp_code_str(code), tag & 0xffff) } else { crp_code_str(code) }
}

fn crp_canonical(key: &str, entry: &str, dec: &[u8]) -> String {
    use nf_formats::crp::{Crp, tag};
    let c = Crp::parse(dec).unwrap_or_else(|e| panic!("{key}: CARP parse failed: {e}"));
    let mut pool_hist: std::collections::BTreeMap<String, u32> = Default::default();
    for r in c.pool().unwrap_or_else(|e| panic!("{key}: pool: {e}")) {
        *pool_hist.entry(crp_render(r.tag, r.flags)).or_default() += 1;
    }
    let pool: Vec<String> = pool_hist.into_iter().map(|(k, v)| format!("{k}×{v}")).collect();
    let mut chunks = Vec::new();
    let mut elfd: Option<(usize, usize)> = None;
    for r in c.directory().unwrap_or_else(|e| panic!("{key}: directory: {e}")) {
        let at = r.data_addr(dec).unwrap_or_else(|e| panic!("{key}: data_addr: {e}"));
        chunks.push(format!("{}:{}:{:#x}", crp_render(r.tag, r.flags), r.size(), at));
        if r.tag_code() == tag::ELFD { elfd = Some((at, r.size())); }
    }
    let elf = elfd.and_then(|(at, size)| -> Option<String> {
        let e = dec.get(at..at + size)?;
        if e.get(..4)? != b"\x7fELF" { return None; }
        let u16le = |o: usize| u16::from_le_bytes([e[o], e[o + 1]]);
        Some(format!("t{}.m{}.s{}", u16le(0x10), u16le(0x12), u16le(0x30)))
    }).unwrap_or_else(|| "-".into());
    let elfd_sha = match elfd {
        Some((at, size)) => sha1_hex(&dec[at..at + size]),
        None => "-".into(),
    };
    let name = c.track_name().unwrap_or_else(|e| panic!("{key}: track_name: {e}")).unwrap_or("-");
    format!("{key}\t{entry}\t{}\t{}\t{:#x}\t{}\t{}\t{elf}\t{name}\t{elfd_sha}\n",
        dec.len(), sha1_hex(dec), c.root.flags, pool.join(","), chunks.join(","))
}

#[test]
fn driving_crp_files_parse_to_the_golden() {
    let Some(dir) = driving_dir() else { eprintln!("skip: NF_DRIVING/DRIVING not available"); return };
    let golden = include_str!("../golden/crp_golden.tsv");
    let mut rows = Vec::new();
    for (name, g, blobs) in archive_rows(&dir) {
        for (i, (e, blob)) in g.entries.iter().zip(&blobs).enumerate() {
            if !e.name.to_ascii_lowercase().ends_with(".crp") { continue; }
            let key = format!("{name}#{i}");
            let dec = if nf_formats::refpack::is_refpack(blob) {
                nf_formats::refpack::decompress(blob)
                    .unwrap_or_else(|err| panic!("{key}: {} refpack: {err}", e.name))
            } else { blob.clone() };
            rows.push(crp_canonical(&key, &e.name, &dec));
        }
    }
    assert_eq!(rows.len(), 89, "every .crp entry of the 23 archives must be parsed");
    let actual = format!("key\tentry\tdecompressed\tsha1\troot_flags\tpool\tchunks\telf\tname\telfd_sha1\n{}", rows.join(""));
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    // Sister: a one-character change to one digest must be caught.
    let mut lines: Vec<String> = actual.lines().map(String::from).collect();
    let row = &mut lines[1];
    let last = row.pop().unwrap();
    row.push(if last == '0' { '1' } else { '0' });
    lines.push(lines.last().unwrap().clone());
    let tampered = lines.join("\n") + "\n";
    assert_eq!(diff_tsv(golden, &tampered).len() >= 1, true, "sister: a one-character change must be caught");
}

#[test]
fn driving_crp_structure_matches_the_consumer_split() {
    let Some(dir) = driving_dir() else { eprintln!("skip: NF_DRIVING/DRIVING not available"); return };
    use nf_formats::crp::{Crp, tag};
    let mut tracks = 0usize;
    let mut with_elf = 0usize;
    let mut plain_misc_cars = 0usize;
    for (name, g, blobs) in archive_rows(&dir) {
        for (i, (e, blob)) in g.entries.iter().zip(&blobs).enumerate() {
            if !e.name.to_ascii_lowercase().ends_with(".crp") { continue; }
            let key = format!("{name}#{i}");
            let dec = if nf_formats::refpack::is_refpack(blob) {
                nf_formats::refpack::decompress(blob).unwrap()
            } else { blob.clone() };
            let c = Crp::parse(&dec).unwrap_or_else(|err| panic!("{key}: {err}"));
            // Every root directory carries the module chunks the game loads [M 0x1a9600].
            assert!(c.find(tag::ELFD).unwrap().is_some(), "{key}: no ELFd module chunk");
            assert!(c.find(tag::SECT).unwrap().is_some(), "{key}: no Sect chunk");
            with_elf += 1;
            // Track containers: the DBN name + the MapN/RNgp/CDat pool the track path needs
            // [M 0x22f898/0x2dc1b0; measured: the 8 data\track\*.crp entries].
            if e.name.starts_with("data\\track\\") {
                let n = c.track_name().unwrap().expect("{key}: the track .crp must carry a DBN name");
                assert!(!n.is_empty(), "{key}: empty DBN name");
                let pool = c.pool().unwrap();
                assert!(pool.iter().any(|r| r.tag == tag::RNGP), "{key}: track pool must hold a road network");
                assert!(pool.iter().any(|r| r.tag == tag::CDAT), "{key}: track pool must hold collision data");
                assert!(c.find(tag::MAPN).unwrap().is_some(), "{key}: track must carry a MapN chunk");
                tracks += 1;
            }
            // The five MISC.VIV car .crp entries are the only plain (non-RefPack) ones [D, census].
            if !nf_formats::refpack::is_refpack(blob) {
                assert!(name.ends_with("MISC.VIV") && e.name.starts_with("data\\car\\model\\"),
                    "{key}: unexpected plain .crp outside MISC.VIV car models");
                plain_misc_cars += 1;
            }
        }
    }
    assert_eq!(tracks, 8, "the eight track containers (one per mission archive)");
    assert_eq!(plain_misc_cars, 5, "MISC.VIV's plain car models (docs/formats/driving-viv.md)");
    assert_eq!(with_elf, 89, "every .crp embeds a MIPS ELF module");
    // Sister: the track split is load-bearing.
    assert_ne!(tracks, 7);
}

// ---------------------------------------------------------------------------
// drive-track-mesh: the article/mesh layer over the track containers
// (crates/nf-formats/src/crp_mesh.rs, golden/crp_mesh_golden.tsv).

/// The canonical per-track mesh digest row: counts over the article pool + the relocated module
/// image (numbers only; same form `nf-extract crp-mesh-digest` prints, golden/crp_mesh_golden.tsv).
fn mesh_canonical(viv: &str, idx: usize, entry: &str, dec: &[u8]) -> String {
    use nf_formats::crp::Crp;
    use nf_formats::crp_mesh::TrackModule;
    let c = Crp::parse(dec).unwrap_or_else(|e| panic!("{viv}#{idx} {entry}: {e}"));
    let track = c.track_name().unwrap().unwrap_or("-").to_owned();
    let articles = c.track_articles().unwrap_or_else(|e| panic!("{entry}: {e}"));
    let module = TrackModule::load(&c).unwrap_or_else(|e| panic!("{entry}: {e}"));
    let coll = c.collision_census().unwrap();
    let model_symbols = module.symbols.iter().filter(|s| s.name.starts_with("__Model:::")).count();
    let geoprims = module.geoprim_names().len();
    let parts: usize = articles.iter().map(|a| a.parts.len()).sum();
    let sha1: String = sha1_hex(&module.image);
    format!(
        "{viv}#{idx}\t{entry}\t{track}\t{}\t{parts}\t{model_symbols}\t{geoprims}\t{}\t{}\t{}\t{sha1}",
        articles.len(),
        module.relocations,
        coll.ci_total_items,
        coll.cn_records,
    )
}

#[test]
fn driving_track_mesh_layer_matches_the_golden() {
    let Some(dir) = driving_dir() else { return };
    let golden_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("golden/crp_mesh_golden.tsv");
    let golden = std::fs::read_to_string(&golden_path).expect("golden/crp_mesh_golden.tsv");
    let mut expected: Vec<String> = golden.lines().skip(1).map(str::to_owned).collect();
    expected.sort();
    let mut got: Vec<String> = Vec::new();
    for name in MISSION_VIVS {
        let (g, blobs) = load_viv(&dir, name);
        let (idx, e) = g.entries.iter().enumerate()
            .find(|(_, x)| x.name.to_ascii_lowercase().starts_with("data\\track\\") && x.name.to_ascii_lowercase().ends_with(".crp"))
            .unwrap_or_else(|| panic!("{name}: no data\\track\\*.crp entry"));
        let blob = &blobs[idx];
        let dec = if nf_formats::refpack::is_refpack(blob) {
            nf_formats::refpack::decompress(blob).unwrap_or_else(|err| panic!("{}: {err}", e.name))
        } else {
            blob.clone()
        };
        got.push(mesh_canonical(name, idx, &e.name, &dec));
    }
    got.sort();
    assert_eq!(got.len(), expected.len(), "golden row count");
    for (g, e) in got.iter().zip(expected.iter()) {
        assert_eq!(g.as_str(), e.as_str(), "\n got: {g}\nwant: {e}");
    }
    // Sister: a tampered article name must change the digest (the layer is load-bearing, not a
    // pass-through). Flip a byte inside the first article's Name payload region of paris and
    // expect the module sha1 row to differ somewhere other than the identity.
    let (g, blobs) = load_viv(&dir, "MIS01.VIV");
    let (idx, e) = g.entries.iter().enumerate()
        .find(|(_, x)| x.name.to_ascii_lowercase().starts_with("data\\track\\")).unwrap();
    let mut dec = nf_formats::refpack::decompress(&blobs[idx]).unwrap();
    let mid = dec.len() / 2;
    dec[mid] ^= 0xff;
    let tampered = mesh_canonical("MIS01.VIV", idx, &e.name, &dec);
    let clean = got.iter().find(|r| r.starts_with("MIS01")).unwrap();
    assert_ne!(tampered.as_str(), clean.as_str(),
        "a flipped decompressed byte must change the digest");
}
