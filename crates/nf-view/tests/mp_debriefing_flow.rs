//! mp-debriefing-data2: the debriefing page's front-end flow (headless) and the disc-pinned
//! labels/tables (NF_ISO).
//!
//! The flow is C_MPDBG 0x217860's two arms [M]: Confirm (~A, msg 0x4b) → the scenario page;
//! Extra (~Y, msg 0x5d) → the arena re-requested. The rows/result come from
//! `MpMatch::debriefing` (P_MPDEBRIEFING 0x212198 msg 0x4c). Each step keeps a sister
//! assertion that can fail. Adapted from 621's flow test (the handover now goes through
//! master's `enter_page`).

use nf_game::drone::Rng;
use nf_game::mp::debrief::{MpDebriefName, RESULT_DRAW_LABEL, RESULT_WINS_LABEL, TEAM_MI6_LABEL};
use nf_game::mp::{MpMatch, MpSettings, scenario};
use nf_view::menu::{self,
    Action, FrontEnd, Page, MP_BOTS_ROWS, MP_CHARACTERS_SMALL, MP_DEBRIEFING_COLS,
    MP_DEBRIEFING_HEADER, MP_DEBRIEFING_HINT,
};

/// A finished 2-human + 2-bot Arena match with known numbers.
fn finished_match() -> MpMatch {
    let mut s = MpSettings::deathmatch(0x0700_0024);
    s.scenario = scenario::DEATHMATCH;
    s.gametype_mask = scenario::DEATHMATCH;
    s.human_count = 2;
    s.bot_count = 2;
    s.total_players = 4;
    s.frag_limit = 10;
    s.time_limit = 600;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(7));
    m.settings.players[0].codename = "Bond".into();
    m.settings.players[1].codename = "Drake".into();
    m.settings.players[4].skin = 6; // Snow Guard preset
    m.settings.players[5].skin = 2; // Rook preset
    m.game.slots[0].kills = 9;
    m.game.slots[0].score = 9.0;
    m.game.slots[1].kills = 8;
    m.game.slots[1].score = 8.0;
    m.game.slots[4].kills = 8;
    m.game.slots[4].score = 8.0;
    m.game.slots[5].kills = 2;
    m.game.slots[5].score = 2.0;
    m.game.slots[1].deaths = 3;
    m.game.slots[4].deaths = 5;
    m.game.slots[5].deaths = 7;
    m
}

#[test]
fn the_handover_opens_the_page_with_the_rows() {
    let m = finished_match();
    let mut fe = FrontEnd::new();
    fe.page = Page::MpScenario; // anywhere in the MP flow; the handover sets the page itself
    fe.open_mp_debriefing(&m);
    // G-048(3): the match end only QUEUES the page [M MP_Update 0x184bd0:62-78]; the load tail's
    // MenuManager_Create lands it [M 0x1d6360:160-163].
    assert_eq!(fe.flow.page, menu::PAGE_MP_DEBRIEFING, "case 4 queues +0x1c = 0x40000033 [M 0x184bd0:63]");
    assert_eq!(fe.up_page(), None, "nothing is up over the paused arena");
    assert!(fe.reopen_queued_page(), "the load tail's create opens the debriefing");
    assert_eq!(fe.page, Page::MpDebriefing, "the case-4 navigation lands on 0x40000033 [M 0x184bd0:63]");
    assert_eq!(fe.page.id(), 0x4000_0033);
    let rows = fe.mp_debriefing_rows();
    // Sort by TOTAL (kills in Arena): Bond 9, then the two 8s (slot 1 built before slot 4 —
    // the game's stable bubble sort [M 0x212198:112-130]), then Rook-bot 2.
    assert_eq!(rows[0], (0x0100_0297, MpDebriefName::Human("Bond".into()), 0x0300_0154, 9, 9, 0, 9), "rank 1st + Bond's small-table portrait [L 0x2df8f8 row 0]");
    assert_eq!(rows[1].0, 0x0100_0298, "the 8s start the \"2nd\" rank [M 0x212198:135-137]");
    assert_eq!(rows[1].1, MpDebriefName::Human("Drake".into()));
    assert_eq!(rows[2].1, MpDebriefName::Bot(6), "a bot's short-name character [M 0x212198:173-177]");
    assert_eq!(rows[2], (0x0100_0298, MpDebriefName::Bot(6), 0x0300_0167, 8, 8, 5, 8), "the 8-tie shares \"2nd\"; Snow Guard's portrait [L]");
    assert_eq!(rows[3].0, 0x0100_029a, "the distinct 2 drops to \"4th\" [M 0x212198:135-137]");
    assert_eq!(rows[3].2, 0x0300_0165, "Rook's portrait [L]");
    // The result: the top two tie at 8? No — Bond leads 9 vs 8: a win, not a draw.
    let (label, who) = fe.mp_debriefing_result();
    assert_eq!(label, RESULT_WINS_LABEL, "\"%s wins the match.\" [L 0x0100029f]");
    assert_eq!(who, "Bond");
    // Sister: a tied top pair flips the line to the draw text.
    let mut m2 = finished_match();
    m2.game.slots[0].kills = 8;
    m2.game.slots[0].score = 8.0;
    let mut fe2 = FrontEnd::new();
    fe2.open_mp_debriefing(&m2);
    assert_eq!(fe2.mp_debriefing_result().0, RESULT_DRAW_LABEL, "\"Match was a draw\" [L 0x010002a0]");
    // The team line rides the label ids [M 0x212198:233-249].
    let mut m3 = finished_match();
    m3.settings.scenario = scenario::TEAM_DEATHMATCH;
    m3.game.team_scores = [1.0, 4.0];
    let mut fe3 = FrontEnd::new();
    fe3.open_mp_debriefing(&m3);
    let (l3, w3) = fe3.mp_debriefing_result();
    assert_eq!((l3, w3.as_str()), (TEAM_MI6_LABEL, ""), "team0 behind -> MI6's label, no %s arg");
}

#[test]
fn confirm_returns_to_the_scenario_page() {
    let m = finished_match();
    let mut fe = FrontEnd::new();
    fe.open_mp_debriefing(&m);
    assert!(fe.reopen_queued_page()); // the load tail's create [G-048(3), M 0x1d6360:160-163]
    assert!(fe.action(Action::Confirm), "Confirm (~A) is handled [M C_MPDBG 0x4b]");
    assert_eq!(fe.page, Page::MpScenario, "C_MPDBG msg 0x44 -> 0x4000001a [M 0x217860:8-11]");
    assert_eq!(fe.page.id(), 0x4000_001a);
    // Sister: other pages do not react to the debriefing's Confirm this way.
    let mut fe2 = FrontEnd::new();
    fe2.page = Page::Main;
    assert!(!fe2.action(Action::Confirm) || fe2.page != Page::MpScenario);
}

#[test]
fn extra_rematch_rerequests_the_arena() {
    let m = finished_match();
    let mut fe = FrontEnd::new();
    fe.open_mp_debriefing(&m);
    assert!(fe.reopen_queued_page()); // the load tail's create [G-048(3), M 0x1d6360:160-163]
    fe.mp_level = 0x0700_0027; // Fort Knox was the arena [L mp_level row 1]
    fe.mp_match_settings = Some(nf_game::mp::MpSettings::deathmatch(0x0700_0027));
    fe.tick(); // the flow machine must be live for the load push [M the confirm-start's pattern]
    assert!(fe.action(Action::Extra), "the ~Y arm is handled [M C_MPDBG 0x5d]");
    assert_eq!(fe.requested_level, Some(0x0700_0027), "ResetMap_LevelToLoad(MPSettings+0x1A8) [M 0x217860:12-14]");
    assert!(fe.mp_match_settings.is_some(), "the config stays for the host (the rematch)");
    // Sister: the confirm arm does NOT touch the level request.
    let mut fe2 = FrontEnd::new();
    fe2.open_mp_debriefing(&finished_match());
    fe2.tick();
    assert!(fe2.action(Action::Confirm));
    assert_eq!(fe2.requested_level, None, "the restore arm only changes the page");
}

#[test]
fn the_static_labels_are_the_pages() {
    // The page's own texts [L the menu-def page 0x40000033 — see the NF_ISO test below].
    assert_eq!(MP_DEBRIEFING_HEADER, 0x151, "\"Debriefing\"");
    assert_eq!(MP_DEBRIEFING_HINT, 0x2c1, "\"~A Continue  ~Y Replay\"");
    assert_eq!(MP_DEBRIEFING_COLS, [0x14e, 0x24c, 0x686, 0x2be, 0x163], "Agent/Points/Victories/Deaths/TOTAL");
    assert_eq!(MP_CHARACTERS_SMALL.len(), 29, "29 character rows");
    // Sister: the columns differ from the confirm page's summary labels.
    assert!(!MP_DEBRIEFING_COLS.contains(&0x3d4), "no Map row on the debriefing");
    // MP_BOTS_ROWS is the wheel's own list — the small table is a different table [L].
    assert_eq!(MP_BOTS_ROWS.len() + MP_CHARACTERS_SMALL.len() > 29, true, "both tables exist");
}

// --- The NF_ISO half: the disc's own bytes ---

fn iso_lang() -> Option<nf_formats::hud::Lang> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = nf_formats::iso9660::IsoImage::open(path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

fn action_elf() -> Option<Vec<u8>> {
    let path = std::env::var_os("NF_ISO")?;
    let img = nf_formats::iso9660::IsoImage::open(path).ok()?;
    img.read_file(img.find("/ACTION.ELF")?).ok()
}

fn u32_at(elf: &[u8], va: u32) -> u32 {
    let p = (va - 0x106100) as usize;
    u32::from_le_bytes([elf[p], elf[p + 1], elf[p + 2], elf[p + 3]])
}

#[test]
fn the_labels_bind_to_the_discs_text() {
    let Some(lang) = iso_lang() else { eprintln!("skip: NF_ISO not set"); return };
    let text = |id: u32| lang.bind(id).unwrap_or("<unbound>").to_string();
    assert_eq!(text(MP_DEBRIEFING_HEADER), "Debriefing", "the page header [L USATxt]");
    assert!(text(MP_DEBRIEFING_HINT).contains("~A Continue"), "the hint bar [L]");
    assert_eq!(text(0x14e), "Agent");
    assert_eq!(text(0x24c), "Points");
    assert_eq!(text(0x686), "Victories");
    assert_eq!(text(0x2be), "Deaths");
    assert_eq!(text(0x163), "TOTAL");
    assert_eq!(text(0x0100_0297), "1st", "rank 1 [L 0x309520]");
    assert_eq!(text(0x0100_029a), "4th", "rank 4 [L 0x309520]");
    assert_eq!(text(RESULT_WINS_LABEL), "%s wins the match.");
    assert_eq!(text(RESULT_DRAW_LABEL), "Match was a draw");
    assert_eq!(text(0x1c7), "Phoenix", "the team labels [L]");
    assert_eq!(text(TEAM_MI6_LABEL), "MI6");
    assert_eq!(text(0x0100_02e2), "Bond", "Menu_GetBotShortName's base label [M 0x201918]");
    assert_eq!(text(0x0100_02fd), "Nik Nack", "character 27 [M 0x201918]");
    // Sister: an id the page never uses must not silently equal a used one.
    assert_ne!(text(0x2be), text(0x24c), "Deaths and Points are distinct texts");
}

#[test]
fn the_small_character_table_is_the_discs() {
    let Some(elf) = action_elf() else { eprintln!("skip: NF_ISO not set"); return };
    // mp_characters_small @0x2df8f8: 29 x 0x18 rows, key at +0x0C, preview at +0x00 [L].
    for (i, (tex, name, _desc, key)) in MP_CHARACTERS_SMALL.iter().enumerate() {
        let base = 0x2df8_f8 + (i as u32) * 0x18;
        assert_eq!(u32_at(&elf, base), *tex, "row {i} preview");
        assert_eq!(u32_at(&elf, base + 4), *name, "row {i} name label");
        assert_eq!(u32_at(&elf, base + 0xc), *key, "row {i} key = the character id");
    }
    // Sister: the big wheel table's previews differ (the debriefing uses the SMALL one).
    assert_ne!(MP_CHARACTERS_SMALL[0].0, 0x0300_010a, "small-table Bond is 0x03000154, not the wheel's 0x0300010a [L]");
}
