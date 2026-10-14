//! fix-save-profile NF_ISO tests: the codename profile save through the front end's real flow.
//!
//! The game's model ([M] addresses in the cites): the memory-card file IS the codename
//! (Menu_UpdateMessageBox 0x1f7de8:19/28 passes the name buffer 0x2e1ae8 to LS_Load/LS_Save);
//! the chain is PLRS/MSSN/MPSG/GSET/CHET/BNUS [L SaveIFFBlocks 0x2c74d8] and the game's own
//! loaded chain sits in InBufData 0x3416e0 of the ee_start snapshots [L] — the tests decode it
//! through the same path the port's files take. The port's card = the profiles dir, one file
//! per codename.

use nf_view::menu::{Action, FrontEnd, Outcome, Page, Stats};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-fix-save-profile-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// The game's own chain [L InBufData 0x3416e0] + the CRC trailer, or None without the snapshot.
fn game_chain() -> Option<Vec<u8>> {
    let snap = std::fs::read("../work/ee_snapshot.bin").or_else(|_| std::fs::read("../../work/ee_snapshot.bin")).ok()?;
    let chain = &snap[0x3416e0..0x3416e0 + 1022];
    let mut file = chain.to_vec();
    file.extend_from_slice(&nf_formats::png::crc32(0, chain).to_le_bytes());
    Some(file)
}

/// ── 1. Creating a codename ──────────────────────────────────────────────────────────────────
/// The game's flow: the keyboard validates through Menu_IsValidCodename 0x1f9930 (trim, upper,
/// empty 0x3bc, a default-codename substring 0x2c0), Menu_MapDefaultCodename 0x1f7fa8 maps the
/// default template (nightfire 0 → 0x3 [M Menu_SetNightfireStatus 0x2023a0]), and
/// Menu_UpdateMessageBox 0x1f7de8 case 1 writes the chain under the codename.
#[test]
fn create_codename_saves_the_profile() {
    let dir = temp_dir("create");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.page = Page::Codename;
    assert_eq!(fe.rows(), 2, "a fresh card lists only the two defaults [M 0x1f7b78]");
    assert_eq!(fe.row_label(None, 0), "<0x29b>", "no language bank headless — the id placeholder");
    assert_eq!(fe.row_label(None, 1), "<0x10002ff>");

    // The validator [M 0x1f9930]: trim + uppercase; reserved substrings; empty.
    assert_eq!(fe.create_codename("  agent 007 ", &["BOND", "MONEYPENNY"]), Ok("AGENT 007".into()));
    assert_eq!(fe.create_codename("AGENT 007", &[]), Err(nf_game::profile::CodenameError::Exists));
    assert_eq!(fe.create_codename("MY BOND PLAN", &["BOND"]), Err(nf_game::profile::CodenameError::Reserved));
    assert_eq!(fe.create_codename("   ", &[]), Err(nf_game::profile::CodenameError::Empty));
    assert_eq!(
        fe.create_codename("NEW CODENAME", &["New Codename"]),
        Err(nf_game::profile::CodenameError::Reserved),
        "the game's own row label is a reserved name [M def_codename[0] = 0x29b]"
    );

    // The card now lists the codename and the page rows follow [M Menu_EnumSaves's appends].
    assert_eq!(fe.rows(), 3);
    assert_eq!(fe.row_label(None, 2), "AGENT 007");
    assert_eq!(fe.active_profile.as_deref(), Some("AGENT 007"));
    let path = nf_game::profile::Profile::path_in(&dir, "AGENT 007");
    assert!(path.exists(), "one file per codename, named by it [M 0x1f7de8]");

    // The file carries the game's layout: the six blocks at the game's own sizes [L InBufData],
    // and a fresh profile's fields (the template's values [L 0x2e1b18 + M 0x2023a0's 0x3]).
    let bytes = std::fs::read(&path).unwrap();
    let chain = &bytes[..bytes.len() - 4];
    let mut off = 0;
    let mut sizes = Vec::new();
    while off + 8 <= chain.len() {
        let size = u32::from_le_bytes(chain[off + 4..off + 8].try_into().unwrap()) as usize;
        sizes.push(size);
        off += size;
    }
    assert_eq!(sizes, vec![11, 68, 13, 905, 9, 16], "the game's own block sizes [L]");
    assert_eq!(off, 1022, "the game's own chain length [L]");
    let p = nf_game::profile::Profile::load_file(&path).unwrap();
    assert_eq!(p.progress.nightfire_status(), 0x3, "a fresh codename unlocks rows 0 and 1 [M 0x2023a0]");
    assert_eq!(p.plr.controller_style, 7, "the template's style byte [L 0x2e1b18+0x20]");
    assert_eq!(p.gset.music_volume, 70, "the template's music volume [L 0x2e1b18+0x10]");
    assert_eq!(p.gset.sfx_volume, 100, "the template's sfx volume [L 0x2e1b18+0x14]");
    assert!(!p.chet.cheat_info && !p.chet.cheat_ammo && !p.chet.cheat_fire, "[M 0x1f7fa8 resets the cheats]");
    let _ = std::fs::remove_dir_all(&dir);
}

/// ── 2. Loading on the codename page ─────────────────────────────────────────────────────────
/// The game's own chain (the snapshot's InBufData [L]) round-trips through the page's real
/// action path: the rows follow Menu_EnumSaves's order, Confirm loads (Menu_UpdateMessageBox
/// 0x1f7de8 case 0 → LS_Load 0x1cdfc0 → LS_GetLoadStuff 0x1cd2b0 applies the blocks), the
/// nightfire status becomes the unlock mask [M Menu_SetNightfireStatus 0x2023a0], and the
/// Records page reads the loaded progress [M C_RBDSRECORDS 0x20bf00:26-28].
#[test]
fn codename_page_loads_the_game_chain() {
    let Some(file) = game_chain() else { eprintln!("no ee_snapshot.bin — skipping the [L] flow test"); return };
    let dir = temp_dir("load");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(nf_game::profile::Profile::path_in(&dir, "SNAP"), &file).unwrap();

    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.page = Page::Codename;
    assert_eq!(fe.rows(), 3);
    assert_eq!(fe.row_label(None, 2), "SNAP", "the card's row IS the codename [M 0x1f7b78]");
    // The real action path: Down, Down, Confirm.
    assert!(fe.action(Action::Down));
    assert!(fe.action(Action::Down));
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::Difficulty, "the load continues to the difficulty page [M C_SBNFCN 0x20b390]");
    assert_eq!(fe.cursor, 1, "the wheel starts on Agent [M the centred middle row]");
    assert_eq!(fe.active_profile.as_deref(), Some("SNAP"));
    // The chain's nightfire status 0xfff [L] becomes the unlock mask.
    assert!(fe.unlocked.iter().all(|&u| u), "the snapshot's profile has all 12 missions unlocked [L MSSN ff 0f 00 00]");

    // The Records values come from the loaded profile [M 0x20bf00 reads ScoringTable+0x30 etc.].
    let direct = nf_game::profile::Profile::load_file(&nf_game::profile::Profile::path_in(&dir, "SNAP")).unwrap();
    for row in 0..12 {
        assert_eq!(
            fe.record_row(row),
            (direct.progress.table[row].best, direct.progress.table[row].rows[0].value, {
                let hash = nf_game::progress::SP_LEVEL_HASHES[row];
                (1..=4).filter(|&k| direct.progress.has_medal(hash, k, 0)).count() as u32
            }),
            "the Records page reads the profile's progress for row {row}"
        );
    }
    // And a profile-less front end records zeros (the pre-task behaviour).
    assert_eq!(FrontEnd::new().record_row(0), (0, 0, 0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// ── 3. Saving after a mission: the unlock ───────────────────────────────────────────────────
/// The real path: a level loads through `nf_view::load` and ticks through `Sim::step_bond`
/// (the harness the host drives); the front end's `mission_ended` is the results hook the host
/// calls when the sim's mission status completes. Completing mission 0 must set the next row's
/// unlock [M Menu_SetLevelBonus 0x1ffa10:63-65 stamps sp_level[i]+0x10 for the level and the
/// next one] and write the chain so a reload on the codename page restores it.
#[test]
fn completing_a_mission_unlocks_and_saves() {
    let dir = temp_dir("unlock");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.create_codename("UNLOCKTEST", &["BOND"]).expect("create");

    // The real harness: the mission's own level, warm ticks through step_bond (the sim's
    // progress is the tally the results page reads).
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("no NF_ISO — skipping the sim part"); return };
    let mut scene = match nf_view::load(&iso, "07000005.bin", false) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("07000005.bin load failed ({e}) — skipping the sim part");
            return;
        }
    };
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..30 {
        sim.step_bond(Default::default(), false, false, false);
    }
    // The host's merge: the sim's tally becomes the profile's progress before the save
    // [M the game writes its MSSN/BNUS from the live state the results page reads].
    fe.mission = 0;
    fe.sync_profile_progress(&sim.drones.progress);

    fe.mission_ended(Outcome::Complete, Stats { shots_fired: 0, shots_hit: 0, accuracy_pct: 0, time_s: 1 });
    assert!(fe.unlocked[1], "completing row 0 unlocks row 1 [M 0x1ffa10:63-65]");
    assert!(fe.save_profile(), "the results save wrote the file");

    // A new session loads it back on the codename page and the unlock survives.
    let mut fe2 = FrontEnd::new();
    fe2.set_profile_dir(dir.clone());
    fe2.page = Page::Codename;
    assert_eq!(fe2.rows(), 3);
    assert!(fe2.action(Action::Down));
    assert!(fe2.action(Action::Down));
    assert!(fe2.action(Action::Confirm));
    assert!(fe2.unlocked[1], "the unlock survives the round trip");

    // Sister: a FAILED mission saves nothing new — the fail menu has no save arm [M 0x216618].
    let before = std::fs::read(nf_game::profile::Profile::path_in(&dir, "UNLOCKTEST")).unwrap();
    fe.mission = 2;
    fe.mission_ended(Outcome::Failed, Stats { shots_fired: 0, shots_hit: 0, accuracy_pct: 0, time_s: 1 });
    let after = std::fs::read(nf_game::profile::Profile::path_in(&dir, "UNLOCKTEST")).unwrap();
    assert_eq!(before, after, "the fail path does not write the profile");
    assert!(!fe.unlocked[3], "a failed mission unlocks nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// ── 4. The CRC trailer guards the file ──────────────────────────────────────────────────────
/// A corrupted profile is refused on load (the game's LOADERROR path [M MemCard_LOADERROR
/// 0x10f138 / psiLoadData's checks]; the port's decode verifies the CRC-32 trailer).
#[test]
fn corrupt_profile_is_refused() {
    let dir = temp_dir("crc");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.create_codename("CORRUPT", &[]).unwrap();
    let path = nf_game::profile::Profile::path_in(&dir, "CORRUPT");
    let mut bytes = std::fs::read(&path).unwrap();
    let n = bytes.len();
    bytes[n / 2] ^= 0x08;
    std::fs::write(&path, &bytes).unwrap();
    assert!(nf_game::profile::Profile::load_file(&path).is_err(), "the flipped bit must not load");
    // The card still lists the row (the game shows the row and errors on the load [M
    // MemCard_FILE_CORRUPT 0x10efc0's flow]); the page load reports failure and stays.
    let mut fe2 = FrontEnd::new();
    fe2.set_profile_dir(dir.clone());
    fe2.page = Page::Codename;
    assert_eq!(fe2.rows(), 3);
    assert!(fe2.action(Action::Down));
    assert!(fe2.action(Action::Down));
    fe2.action(Action::Confirm);
    assert!(fe2.active_profile.is_none(), "the corrupt load leaves no active profile");
    let _ = std::fs::remove_dir_all(&dir);
}
