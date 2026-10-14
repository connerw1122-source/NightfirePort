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
//! Any panic is a bug (the task's rule): the test fails the level when a cell errored, even though
//! the matrix keeps going to measure the rest. Red cells (`I`/`F`/`R`/`W`/`B`/`S` marks) are the
//! report's business (docs/reports/weapon-matrix.md) — they are *measurements*, and the test only
//! fails on panics and hard errors.

use nf_view::matrix;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The matrix levels: `NF_MATRIX=07000005,07000011` selects; unset = all (the sweep's on-foot set).
fn levels() -> Option<Vec<String>> {
    std::env::var("NF_MATRIX").ok().map(|v| v.split(',').map(|s| {
        let s = s.trim();
        if s.contains('.') { s.to_string() } else { format!("{s}.bin") }
    }).collect())
}

macro_rules! matrix_level {
    ($name:ident, $archive:literal) => {
        #[test]
        fn $name() {
            let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
            let wanted = levels();
            if let Some(w) = &wanted {
                if !w.iter().any(|l| l.starts_with($archive)) { return; }
            }
            let mut line = String::new();
            let mut log = |s: &str| { line.push_str(s); line.push('\n'); };
            let row = matrix::run_level(&iso, $archive, &mut log).expect("level loads");
            eprint!("{line}");
            let panics: Vec<String> = row.cells.iter().filter(|c| c.error.is_some())
                .map(|c| format!("{:#04x} {}", c.weapon, c.error.clone().unwrap_or_default())).collect();
            assert!(panics.is_empty(), "weapon matrix: {} cells panicked on {}: {}",
                panics.len(), $archive, panics.join("; "));
            eprint!("{}", matrix::mark_table(std::slice::from_ref(&row)));
        }
    };
}

// The sweep's on-foot set (levels_sweep_live.rs): story 01-0d, 11-16, 1b, then the wrappers
// 41/43/46/48/49/4a/4b. 07000048 is the front-end map — it stays in the set (it is an on-foot
// archive), its cells measure what the front-end carries.
matrix_level!(matrix_07000001, "07000001");
matrix_level!(matrix_07000002, "07000002");
matrix_level!(matrix_07000003, "07000003");
matrix_level!(matrix_07000004, "07000004");
matrix_level!(matrix_07000005, "07000005");
matrix_level!(matrix_07000006, "07000006");
matrix_level!(matrix_07000007, "07000007");
matrix_level!(matrix_07000008, "07000008");
matrix_level!(matrix_07000009, "07000009");
matrix_level!(matrix_0700000a, "0700000a");
matrix_level!(matrix_0700000b, "0700000b");
matrix_level!(matrix_0700000c, "0700000c");
matrix_level!(matrix_0700000d, "0700000d");
matrix_level!(matrix_07000011, "07000011");
matrix_level!(matrix_07000012, "07000012");
matrix_level!(matrix_07000013, "07000013");
matrix_level!(matrix_07000014, "07000014");
matrix_level!(matrix_07000015, "07000015");
matrix_level!(matrix_07000016, "07000016");
matrix_level!(matrix_0700001b, "0700001b");
matrix_level!(matrix_07000041, "07000041");
matrix_level!(matrix_07000043, "07000043");
matrix_level!(matrix_07000046, "07000046");
matrix_level!(matrix_07000048, "07000048");
matrix_level!(matrix_07000049, "07000049");
matrix_level!(matrix_0700004a, "0700004a");
matrix_level!(matrix_0700004b, "0700004b");

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
