//! nis-event9-cnis: the `C_NIS_Handler` route's boot (the IN-GAME NIS page 0x1000003e's msg-0x4b
//! tail [M `C_NIS_Handler` 0x205f10:92-112 — `Script_Load` + `Player_Disable` + `Script_Play` +
//! `Camera_PushStates` + `Camera_Enable(7,0,0,0)`]).
//!
//! The disc census (nf-formats/tests/nis_event9_census.rs) found NO authored menu def for the page
//! anywhere on NTSC-U — no level resolves a binding today — so the route's port is the boot
//! behaviour itself: `Sim::nis_page_boot(hash)` starts the once-mode script player the game's way,
//! gates re-shows on `Script_IsPlaying(FMVScript)` [M 0x205f10:92] and refuses hashes the lib
//! doesn't carry (the game's `hashtable_getitem` arm [M 0x205f10:104-106]). Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn nis_page_boots_the_script_the_games_way_and_gates_reshows() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 01");
    let sim = scene.sim.as_mut().unwrap();

    // A hash the lib doesn't carry: the game's `hashtable_getitem` arm leaves FMVScript alone [M
    // 0x205f10:104-106 — `if (lVar2 != 0)`].
    assert!(!sim.nis_page_boot(0x0600_ffff), "an uncarried hash boots nothing");
    assert!(sim.fmv_script.is_none(), "FMVScript stays 0 [M 0x205f10:71]");

    // The real boot: a script the level's archive carries, started as the page's 0x4b tail does —
    // `Script_Load` + `Player_Disable` + `Script_Play(FMVScript, 0)` [M 0x205f10:99-107].
    let before = sim.drones.scripts.players.len();
    assert!(sim.nis_page_boot(0x0600_005b), "0600005b is in 01's archive [D]");
    assert_eq!(sim.drones.scripts.players.len(), before + 1, "Script_Load made a script player");
    assert_eq!(sim.drones.scripts.players.last().unwrap().scripts[0], 0x0600_005b, "the player is the FMV script");
    assert_eq!(sim.drones.scripts.players.last().unwrap().modes[0], 1, "Script_Play(…, 0) — the once mode");
    assert_eq!(sim.fmv_script, Some(0x0600_005b), "FMVScript = the booted hash [M 0x205f10:87]");
    // The hand-off drains on the tick: the player is disabled for the cut-scene [M 0x205f10:102].
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.disabled, "Player_Disable(glb_players, 0) [M 0x205f10:102]");
    let p = sim.drones.scripts.players.last().unwrap();
    assert!(p.state == 2 && p.info[p.cur].as_ref().is_some_and(nf_game::nis::Script::playing), "the cut-scene runs");

    // The page re-shown while the FMV script plays is a no-op [M 0x205f10:92].
    let count = sim.drones.scripts.players.len();
    assert!(!sim.nis_page_boot(0x0600_005b), "Script_IsPlaying(FMVScript) gates the re-show");
    assert_eq!(sim.drones.scripts.players.len(), count, "no second player");
}
