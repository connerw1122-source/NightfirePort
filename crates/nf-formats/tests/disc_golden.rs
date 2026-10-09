//! Golden tests against the user's own disc image. Opt-in: set `NF_ISO` to the path of the
//! 007: Nightfire (USA) image (docs/build-identity.md); without it every test here prints "skip" and passes.
//!
//! The snapshots in `golden/` hold paths/names, numbers and SHA-1s only. Each golden comparison has a sister
//! assertion proving the same comparison fails when one value changes.
//!
//! Independent cross-check (not a test; needs 7-Zip): `tools/crosscheck/iso-7zip.sh` hashes every file through
//! 7-Zip's own ISO 9660 reader; WORKLOG 0001 records 0 differences against `golden/disc_files.tsv`.

use nf_formats::golden::diff_tsv;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, elf::Elf};

fn iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_ISO")?;
    Some(IsoImage::open(&path).unwrap_or_else(|e| panic!("NF_ISO={path:?}: {e}")))
}

/// Flips one character of the first data row's last column, so a comparison against it must fail.
fn tamper(table: &str) -> String {
    let mut lines: Vec<String> = table.lines().map(String::from).collect();
    let row = &mut lines[1];
    let last = row.pop().unwrap();
    row.push(if last == '0' { '1' } else { '0' });
    lines.join("\n") + "\n"
}

#[test]
fn directory_tree_matches_the_golden_manifest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/disc_files.tsv");
    let actual = disc::manifest(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

#[test]
fn bigf_tables_match_the_golden_manifest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/bigf_entries.tsv");
    let actual = disc::bigf_manifest(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

#[test]
fn files_bin_matches_the_golden_manifest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/files_bin.tsv");
    let actual = disc::files_bin_manifest(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

/// Facts about the BIGF headers recorded in docs/formats/bigf.md: the header-size field is the entry-table end
/// (`.VIV`) or the table end + 8 (`.MUS`/`.SPE`, an `L225` trailer); the LE size field equals the file size except
/// in MISC.VIV (UNKNOWNS U-004).
#[test]
fn bigf_header_fields_behave_as_documented() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut size_mismatch = Vec::new();
    for e in iso.entries().iter().filter(|e| !e.is_directory && e.path.starts_with("/DRIVING/") && !e.path.ends_with(".SYM")) {
        let g = disc::read_bigf(&iso, e).unwrap();
        let trailer = g.header_size - g.table_end();
        if e.path.ends_with(".VIV") {
            assert_eq!(trailer, 0, "{}", e.path);
        } else {
            assert_eq!(trailer, 8, "{}", e.path);
            let t = iso.read_bytes(e.lba as u64 * 2048 + g.table_end() as u64, 8).unwrap();
            assert_eq!(t, b"L225\0\0\0\x02", "{}", e.path);
        }
        if g.size_field != e.size { size_mismatch.push(e.path.as_str()); }
    }
    assert_eq!(size_mismatch, ["/DRIVING/MISC.VIV"]);
}

/// The three executables: SHA-1 (build identity), entry point, single PT_LOAD, and each ELF's own extent equals
/// its directory size (nothing appended).
#[test]
fn executables_match_build_identity() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = "path\tsha1\tentry\tvaddr\tmemsz\textent_eq_size\tsymbols\n\
        /ACTION.ELF\te7ffdcb38ccd309388046ab864ebde5e3414eaab\t2242a8\t107100\t24f52c\ttrue\t5603\n\
        /DRIVING.ELF\td7b8a57f3b8e17e980a6d9228b2ad02ed8eebc5c\t24c0d8\t107100\t3208b4\ttrue\t0\n\
        /SLUS_205.79\td5a9bd3ee0e9f1faf519fbd27138a2cdf4e82efe\ta00008\ta00000\t220c0\ttrue\t0\n";
    let mut actual = String::from("path\tsha1\tentry\tvaddr\tmemsz\textent_eq_size\tsymbols\n");
    for p in ["/ACTION.ELF", "/DRIVING.ELF", "/SLUS_205.79"] {
        let e = iso.find(p).unwrap();
        let data = iso.read_file(e).unwrap();
        let elf = Elf::parse(&data).unwrap();
        assert_eq!(elf.phdrs.len(), 1, "{p}");
        let ph = &elf.phdrs[0];
        actual.push_str(&format!("{p}\t{}\t{:x}\t{:x}\t{:x}\t{}\t{}\n",
            nf_formats::sha1::hex(&nf_formats::sha1::sha1(&data)), elf.entry, ph.vaddr, ph.memsz,
            elf.extent == e.size, elf.symbols(&data).unwrap().len()));
    }
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{}", d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
    assert_eq!(disc::BOOT_ELF_SHA1, "d5a9bd3ee0e9f1faf519fbd27138a2cdf4e82efe");
}
