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

#[test]
fn level_bins_match_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/level_bins.tsv");
    let actual = disc::level_bin_digest(&disc::level_bin_entries(&iso).unwrap());
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

#[test]
fn textures_match_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/textures.tsv");
    let actual = disc::texture_digest(&disc::texture_entries(&iso).unwrap());
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

#[test]
fn geometry_matches_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/geometry.tsv");
    let actual = disc::geometry_digest(&disc::geometry_entries(&iso).unwrap());
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

#[test]
fn level_assembly_matches_the_golden_summary() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/levels.tsv");
    let actual = disc::level_summary(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
    assert!(actual.lines().skip(1).all(|l| l.split('\t').nth(6) == Some("0")), "every placement resolves");
}

#[test]
fn sequences_match_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/sequences.tsv");
    let actual = disc::sequence_digest(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

#[test]
fn scripts_match_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/scripts.tsv");
    let actual = disc::script_digest(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}

/// Every drone gets a start script and sequence except the five whose parameter 1 is 0x06000892 (anim set 0xF has no
/// stand idle; docs/systems/drone-animation.md).
#[test]
fn drone_start_animations_match_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/drones.tsv");
    let actual = disc::drone_start_digest(&iso).unwrap();
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
    let sum = |col: usize| actual.lines().skip(1).map(|l| l.split('\t').nth(col).unwrap().parse::<u32>().unwrap()).sum::<u32>();
    assert_eq!((sum(1), sum(2), sum(3)), (555, 550, 550));
    // Drones present at level start, as counted live in PCSX2 (WORKLOG 0018: tools/trace/goto_level.py snapshots).
    let at_start = |lvl: &str| actual.lines().find(|l| l.starts_with(lvl)).and_then(|l| l.split('\t').nth(4)).unwrap().to_string();
    assert_eq!((at_start("07000005"), at_start("07000001"), at_start("0700000a")), ("13".into(), "15".into(), "0".into()));
}

/// `weapon_data` built by running ACTION.ELF's static initialiser in `ee::Machine`. Cross-check (WORKLOG 0017): equal
/// to PCSX2's live copy in 30,819 of 30,820 bytes; the other is record 1 +0xDC, a skin hashcode the game writes later.
#[test]
fn weapon_data_matches_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let wd = nf_formats::weapon::WeaponData::from_action_elf(&elf).unwrap();
    let armed = (0..wd.records.len()).filter(|&w| wd.third_person(w as u8).is_some()).count();
    let actual = format!("records\tsha1\tthird_person\n{}\t{}\t{armed}\n", wd.records.len(), nf_formats::sha1::hex(&nf_formats::sha1::sha1(&wd.bytes())));
    let golden = "records\tsha1\tthird_person\n115\t2fd48d6d24bbc4f63b8337ded7bd4116d4e3df40\t80\n";
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{}", d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
    assert_eq!(wd.third_person(0x16), Some((0x0200_0b44, 8)));
    assert_eq!(wd.third_person(0x24), Some((0x0200_08ac, 16)));
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

/// Every AI path block (map block 0x05) parses to its exact length, with every link inside its record's nodes
/// (docs/formats/ai-path.md).
#[test]
fn ai_paths_match_the_golden_digest() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let golden = include_str!("../golden/ai_paths.tsv");
    let actual = disc::ai_path_digest(&iso).unwrap();
    assert!(actual.lines().skip(1).all(|l| l.split('\t').nth(5) == Some("0")), "links out of range");
    let d = diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    assert_eq!(diff_tsv(golden, &tamper(&actual)).len(), 1, "sister: a one-character change must be caught");
}
