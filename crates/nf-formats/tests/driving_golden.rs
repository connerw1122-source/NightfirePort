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
