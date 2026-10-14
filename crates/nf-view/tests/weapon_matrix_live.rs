//! The weapon matrix test (task 100d): every on-foot level × every weapon the dev tools hand Bond.
//!
//! One #[test] per level (the suite runs binaries with `--test-threads=1` and a per-binary wall
//! clock, so the level tests interleave in the output and one slow level cannot eat the rest).
//! Each test runs `nf_view::matrix::run_level`: load the level (the normal path), then per weapon
//! the `--give` path, draw, fire ×3 (hold-to-use gadgets hold), reload, switch away and back —
//! recording the view model, the sequences played, the muzzle flash, the sound ids requested and
//! started, the effects spawned and any panic. The printed line per cell is the matrix's raw data;
//! `nf-extract weapon-matrix <iso>` prints the same run as a table.
//!
//! weapon-matrix4b (105a2): the binary must fit the suite's 1200 s per-binary limit. The
//! weapon-matrix4 harness measures weapons only (god refill, script-derived switch/reload budgets,
//! the idle-view re-sync, the twins' base-then-mode route) at honest budgets — ~60-90 s a story
//! level — so the old "every level when NF_MATRIX is unset" default (27 levels, >1200 s; held at
//! merge, `matrix_0700000d` last started) cannot fit. The in-suite set is a fixed sample:
//! 07000005 (the give/gadget harness level), 0700000c (mid-story) and 0700001b (a late story
//! level) — the same sample weapon-matrix3b measured at 270 s for the whole binary — plus the
//! gadget cells, the upgraded-runner checks and the dev-menu row test, with the three
//! 07000005 consumers sharing one computed row. EVERY other level keeps its test, `#[ignore]`d
//! with the reason: run the full 27-level sweep with
//! `NF_MATRIX=all cargo test -p nf-view --test weapon_matrix_live -- --ignored --test-threads=1`
//! (or an explicit `NF_MATRIX=07000001,...` list), or through `nf-extract weapon-matrix <iso>`.
//!
//! Any panic is a bug (the task's rule): the test fails the level when a cell errored, even though
//! the matrix keeps going to measure the rest. Red cells (`I`/`F`/`R`/`W`/`B`/`S` marks) are the
//! report's business (docs/reports/weapon-matrix.md) — they are *measurements*, and the test only
//! fails on panics and hard errors.

use nf_view::matrix;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The matrix levels. weapon-matrix4b: unset = the SUITE'S SAMPLE — 07000005, 0700000c, 0700001b —
/// because the per-level honest budgets (~60-90 s) push the old 27-level default past the runner's
/// 1200 s per-binary limit. `NF_MATRIX=all` (or an explicit `NF_MATRIX=07000001,...` list, `.bin`
/// optional) selects levels again; the sweep levels are `#[ignore]`d so the suite skips them and
/// `-- --ignored` runs them.
fn levels() -> Option<Vec<String>> {
    const SAMPLE: [&str; 3] = ["07000005", "0700000c", "0700001b"];
    match std::env::var("NF_MATRIX") {
        Ok(v) => {
            if v.trim().eq_ignore_ascii_case("all") { return None; }
            Some(v.split(',').map(|s| {
                let s = s.trim();
                if s.contains('.') { s.to_string() } else { format!("{s}.bin") }
            }).collect())
        }
        Err(_) => Some(SAMPLE.iter().map(|s| format!("{s}.bin")).collect()),
    }
}

/// The shared 07000005 row: three tests consume the same measurement (the level test, the
/// upgraded-runner checks, the gadget cells) and a full 05 run costs ~90 s — compute it once per
/// process (the suite runs `--test-threads=1`; the row is read-only afterwards). weapon-matrix3b
/// introduced this share at 270 s for the whole binary; the same arithmetic holds on matrix4's
/// harness.
fn row05(iso: &str) -> &'static matrix::LevelRow {
    static ROW: std::sync::OnceLock<matrix::LevelRow> = std::sync::OnceLock::new();
    ROW.get_or_init(|| {
        let mut line = String::new();
        let mut log = |s: &str| { line.push_str(s); line.push('\n'); };
        let row = matrix::run_level(iso, "07000005", &mut log).expect("07000005 loads");
        eprint!("{line}");
        eprint!("{}", matrix::mark_table(std::slice::from_ref(&row)));
        row
    })
}

/// The body one level test shares. `row` is precomputed for 07000005 (the OnceLock share); every
/// other level runs its own measurement (which streams its cell lines through the log).
fn level_body(iso: &str, archive: &str, shared: Option<&matrix::LevelRow>) {
    let owned;
    let row = match shared {
        Some(r) => r,
        None => {
            let mut line = String::new();
            let mut log = |s: &str| { line.push_str(s); line.push('\n'); };
            owned = matrix::run_level(iso, archive, &mut log).expect("level loads");
            eprint!("{line}");
            &owned
        }
    };
    let panics: Vec<String> = row.cells.iter().filter(|c| c.error.is_some())
        .map(|c| format!("{:#04x} {}", c.weapon, c.error.clone().unwrap_or_default())).collect();
    assert!(panics.is_empty(), "weapon matrix: {} cells panicked on {}: {}",
        panics.len(), archive, panics.join("; "));
    eprint!("{}", matrix::mark_table(std::slice::from_ref(row)));
}

macro_rules! matrix_level {
    // The in-suite sample: runs whenever NF_ISO is set and NF_MATRIX (if set) includes the level.
    (sample, $name:ident, $archive:literal) => {
        #[test]
        fn $name() {
            let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
            let wanted = levels();
            if let Some(w) = &wanted {
                if !w.iter().any(|l| l.starts_with($archive)) { return; }
            }
            if $archive == "07000005" {
                level_body(&iso, $archive, Some(row05(&iso)));
            } else {
                level_body(&iso, $archive, None);
            }
        }
    };
    // The sweep level: #[ignore]d with the reason — the suite skips it, `-- --ignored` (with
    // NF_MATRIX=all or a list) runs it, and `nf-extract weapon-matrix <iso>` covers the same cells.
    (sweep, $name:ident, $archive:literal) => {
        #[test]
        #[ignore = "the full 27-level sweep does not fit the suite's 1200 s per-binary limit (~60-90 s a level on weapon-matrix4's harness); in-suite sample: 07000005/0700000c/0700001b. Run this level with NF_MATRIX=all cargo test -p nf-view --test weapon_matrix_live -- --ignored --test-threads=1 (or NF_MATRIX=<level>), or the whole sweep through nf-extract weapon-matrix <iso>"]
        fn $name() {
            let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
            let wanted = levels();
            if let Some(w) = &wanted {
                if !w.iter().any(|l| l.starts_with($archive)) { return; }
            }
            level_body(&iso, $archive, None);
        }
    };
}

// The sweep's on-foot set (levels_sweep_live.rs): story 01-0d, 11-16, 1b, then the wrappers
// 41/43/46/48/49/4a/4b. 07000048 is the front-end map — it stays in the set (it is an on-foot
// archive), its cells measure what the front-end carries. In-suite: 05, 0c, 1b (the sample);
// every other level is the #[ignore]d sweep.
matrix_level!(sweep, matrix_07000001, "07000001");
matrix_level!(sweep, matrix_07000002, "07000002");
matrix_level!(sweep, matrix_07000003, "07000003");
matrix_level!(sweep, matrix_07000004, "07000004");
matrix_level!(sample, matrix_07000005, "07000005");
matrix_level!(sweep, matrix_07000006, "07000006");
matrix_level!(sweep, matrix_07000007, "07000007");
matrix_level!(sweep, matrix_07000008, "07000008");
matrix_level!(sweep, matrix_07000009, "07000009");
matrix_level!(sweep, matrix_0700000a, "0700000a");
matrix_level!(sweep, matrix_0700000b, "0700000b");
matrix_level!(sample, matrix_0700000c, "0700000c");
matrix_level!(sweep, matrix_0700000d, "0700000d");
matrix_level!(sweep, matrix_07000011, "07000011");
matrix_level!(sweep, matrix_07000012, "07000012");
matrix_level!(sweep, matrix_07000013, "07000013");
matrix_level!(sweep, matrix_07000014, "07000014");
matrix_level!(sweep, matrix_07000015, "07000015");
matrix_level!(sweep, matrix_07000016, "07000016");
matrix_level!(sample, matrix_0700001b, "0700001b");
matrix_level!(sweep, matrix_07000041, "07000041");
matrix_level!(sweep, matrix_07000043, "07000043");
matrix_level!(sweep, matrix_07000046, "07000046");
matrix_level!(sweep, matrix_07000048, "07000048");
matrix_level!(sweep, matrix_07000049, "07000049");
matrix_level!(sweep, matrix_0700004a, "0700004a");
matrix_level!(sweep, matrix_0700004b, "0700004b");

/// The dev menu's "Test weapon" row (dev.rs): the same machine the App's tick loop drives,
/// headless — apply the row through `apply_menu_action_to_scene`, then per tick
/// `test_inputs` → `step_bond` → `test_observe` (lib.rs's merge, verbatim). The P2K on its own
/// start level must draw, fire 3 shot ticks and reload.
#[test]
fn test_weapon_row_runs_the_sequence() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    if let Some(w) = levels() { if !w.iter().any(|l| l.starts_with("07000005")) { return; } }
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let mut dev = nf_view::dev::Dev::default();
    // The weapon in hand on 07000005's start is the P2K (0x02); give it ammo headroom first.
    let (status, _) = nf_view::panel::apply(&mut scene, &[("do".to_string(), "weapon".to_string()), ("id".to_string(), "0x02".to_string()), ("rounds".to_string(), "8".to_string())]);
    eprintln!("[test-weapon-row] give: {status}");
    let out = nf_view::dev::apply_menu_action_to_scene(&mut dev, &mut scene, nf_view::dev::MenuAction::TestWeapon, [0.0; 3], 0.0, false);
    eprintln!("[test-weapon-row] apply: {}", out.status);
    assert!(dev.test_run.is_some(), "the row must start a run");
    let id = dev.test_run.as_ref().unwrap().id;
    assert_eq!(id, 0x02, "the run tests the weapon in hand");
    // Drive the machine the way App's tick loop does (lib.rs's merge, verbatim).
    for _ in 0..1200 {
        let (tw_switch, tw_fire, tw_reload) = dev.test_inputs();
        let input = nf_game::player::Input { switch_to: tw_switch, fire: tw_fire, ..Default::default() };
        scene.sim.as_mut().unwrap().step_bond(input, tw_fire, tw_reload, false);
        dev.test_observe(scene.sim.as_ref().unwrap());
        if dev.test_run.is_none() { break; }
    }
    assert!(dev.test_run.is_none(), "the run must finish inside 1200 ticks");
    let msg = dev.result_msg.clone().unwrap_or_default();
    eprintln!("[test-weapon-row] result: {msg}");
    assert!(msg.contains("draw ok"), "the draw must complete: {msg}");
    assert!(msg.contains("3 shot ticks"), "the P2K must fire 3 shots: {msg}");
    assert!(msg.contains("reload ok"), "the reload must run: {msg}");
}

/// weapon-matrix2 (103b): the upgraded runner's own checks on one level — the real give path
/// (`nf_view::load_weapons`, 100a's merge) dresses the view models, the mode press lands the twin
/// with a label, the MGL impact twin blasts on contact and the Sentinel's guided camera rides.
/// Sister (the numbers the marks gate on): the old runner's `I`/`.` cells for these ids were the
/// stale give path, and the 0x2a/0x2c behaviours were unmeasured — this asserts the new runner's
/// fields on the same real sequence (`matrix::run_level`, NF_ISO). Shares the OnceLock 07000005
/// row with the level test and the gadget cells (weapon-matrix4b).
#[test]
fn upgraded_runner_checks_the_new_behaviours_on_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let row = row05(&iso);
    let cell = |id: u8| row.cells.iter().find(|c| c.weapon == id).expect("catalog cell");
    // The real give path: the merge dresses the previously-`I` launcher cells.
    for id in [0x2au8, 0x2cu8, 0x34u8] {
        let c = cell(id);
        assert!(c.give_ok, "{id:#04x}: the give must equip (real give path)");
        assert!(c.view_model, "{id:#04x}: the merged tables must build the view model");
    }
    // The MGL impact twin 0x2a blasts on first contact (the +0x70 & 0x200 records tick the fuse
    // instead [M Bullet_DoTrails 0x12d890:262-281]) — the gap is flight ticks, not the 210 fuse.
    let a = cell(0x2a);
    assert!(a.grenades > 0 && a.explosions > 0, "0x2a must spawn and blast its grenade");
    assert!(a.expl_gap > 0 && a.expl_gap <= 90, "0x2a blasts on contact, gap {} ticks", a.expl_gap);
    // The Sentinel 'Guided' 0x2c rides the missile camera (weapon +0x6C & 4 [L ee_start_05.bin]).
    assert!(cell(0x2c).guided, "0x2c: the guided camera must ride a live missile");
    // weapon-matrix5 (the missile-cam ride-out, [M Bullet_Delete 0x12e660:27-29 sets the owner's
    // +0x944 = 0xf; Player_Update 0x193600 case 10 counts it down before the substate restores]):
    // the cell survives its own guided camera — the reload/mode/away phases run after the ride-out,
    // so the away switch and the back land. Fail-first: the pre-fix runner's 0x2c cell here was W
    // (the away switch eaten under the substate-10 player) with the mode-press M hidden under it.
    let sentinel = cell(0x2c);
    assert!(sentinel.away_ok && sentinel.back_ok, "0x2c: away {}/back {} — the cell must ride out the guided camera",
        sentinel.away_ok, sentinel.back_ok);
    // weapon-matrix5 (the watch OR, [M Player_Weapon 0x1a24c8: the mode block reads action 12 |
    // action 9 for 0x54/0x55]): the Micro-Camera's twin chain operates — the plain mode press or
    // the OR'd fire presses land the swap. The OR arm's own fail-first is examples/probe_watch.rs
    // (style b: "no swap" pre-fix, "SWAP landed 0x54→0x55" post-fix — the fire press IS a mode
    // press on this pair now).
    assert!(cell(0x54).mode_ok, "0x54: the watch's twin chain must operate (action 12, or the OR'd fire press)");
    // weapon-matrix5b sister ([M Player_SetFiringAnim 0x1a3c70 tail — the `+0xf4 = 9` store sits
    // inside the `hash != 0` branch; a script-less record never enters state 9]): after the fire
    // settle the machine must be IDLE. The unconditional `fire_state = true` latch wedged the
    // proxy at 9 forever for the watch pair (no fire script ever plays, the clear needs
    // `view_script == +0xB0`) and shut the mode block [M 0x1a24c8:199-201].
    assert!(cell(0x54).machine_idle, "0x54: the state-9 proxy must be honest after the fire settle (a script-less record never enters state 9)");
    // The mode press: a twin-linked record lands its twin (or cycles the mode byte) and both
    // labels resolve [M Player_Weapon 0x1a24c8:204-260; HUD_UpdateAmmoPane 0x19a7f0].
    let two = cell(0x02);
    assert!(two.mode_tried && two.mode_ok, "the P2K's mode press must land its twin");
    assert!(two.mode_label_base != 0, "the base mode label must resolve");
}

/// weapon-matrix3 (103m): the gadget cells are real now. The carried gadgets (0x4A Stunner,
/// 0x4E Laser, 0x50 Q-Claw, 0x54 Micro-Camera — owned at the 07000005 level start [L
/// ee_start_05.bin, the level-start inventory; gadget_select_live pins the same set]) must equip
/// through the gadget cycle (`Player_WeaponChange` 0x1a6f80 mode=1, the port's
/// `Input::gadget_cycle`), fire with observable events (the stunner's taser beam / the laser's
/// parked pointer beam / the claw's bolt / the camera's throw) and sound, and survive the switch
/// away and back. Fail-first: on the pre-change runner these cells were `.` — the give declined
/// an already-owned gadget (`Player_EquipWeapon` 0x1a8a58's owned branch) and the fire checks
/// never ran. Sister: 0x4C (not carried on 07000005) stays dim `n` — the game's own cycle cannot
/// reach it either [M 0x1a6f80 walks owned records only]. Shares the OnceLock 07000005 row
/// (weapon-matrix4b).
#[test]
fn gadget_cells_run_the_cycle_on_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let row = row05(&iso);
    let cell = |id: u8| row.cells.iter().find(|c| c.weapon == id).expect("catalog cell");
    for id in [0x4Au8, 0x4eu8, 0x50u8, 0x54u8] {
        let c = cell(id);
        assert!(c.give_ok, "{id:#04x}: carried at the level start [L] — the cell must run");
        assert!(c.draw_ok, "{id:#04x}: the gadget cycle must equip it");
        // The stunner with no guard in the aim ray spawns nothing by the game's own rule
        // ([M 0x1a4bf0:169-200] — the no-target arm); the rest must show their event.
        assert!(c.fire_events() > 0 || c.no_target, "{id:#04x}: the fire must show (beam/bolt/camera/shot)");
        assert!(!(c.sounds.is_empty() && c.started.is_empty()), "{id:#04x}: the equip/fire must sound");
        assert!(c.away_ok && c.back_ok, "{id:#04x}: away {}/back {}", c.away_ok, c.back_ok);
    }
    // The not-carried id: dim, with the cycle-reach reason.
    let nc = cell(0x4c);
    assert!(!nc.give_ok && nc.mark() == 'n', "0x4C is not carried on 07000005 — dim n, got {}", nc.mark());
    assert!(nc.give_msg.as_deref().unwrap_or("").contains("not carried"), "the reason names the cycle's reach");
}
