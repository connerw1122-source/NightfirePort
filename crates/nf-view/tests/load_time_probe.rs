//! The level build-time probe (level-load-time).
//!
//! Times `nf_view::load` for every single-player (on-foot) level on the disc, cold (first load in
//! the process — cold OS file cache + cold OnceLock tables) and warm (the second load of the same
//! level in the same process), broken down by phase. The phase marks ride `load_impl` and
//! `Scene::build` through `nf_view::phase_mark` — the same functions the game path runs (the
//! async-level-load worker executes `load_with_carry` → `load_impl`). The table goes to stdout;
//! the assertions only pin that the timing harness measures something real (a nonzero total, a
//! nonzero scene) so the probe can't pass vacuously.
//!
//! Opt-in via `NF_ISO`; release-only (debug builds are ~10× off the shipped profile).

use std::time::Instant;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Every level archive in FILES.BIN whose name is 0700xxxx.bin, minus the driving missions
/// (07000023-29, out of scope per docs/systems/levels.md §3) and the shared packs (07000500 the
/// global script pack, 07000999). The same definition `levels_sweep_live.rs` uses.
fn on_foot_levels(img: &nf_formats::iso9660::IsoImage) -> Vec<String> {
    let (entries, _) = nf_formats::disc::files_bin_entries(img).unwrap();
    let mut names: Vec<String> = entries.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .filter(|n| {
            let code = u32::from_str_radix(&n[..8], 16).unwrap_or(0) & 0xff;
            !(0x23..=0x29).contains(&code) && code != 0x00 && n != "07000500.bin" && n != "07000999.bin"
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn load_time_probe_all_levels_cold_and_warm_by_phase() {
    let Some(iso) = iso() else { return };
    if cfg!(debug_assertions) { eprintln!("load_time_probe: release-only, skipped on a debug build"); return; }
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open the ISO");
    let levels = on_foot_levels(&img);
    assert!(!levels.is_empty(), "the disc must list the on-foot level archives");
    eprintln!("\n=== load-time probe: {} on-foot levels ===", levels.len());
    eprintln!("{:<10} {:>9} {:>9} | {}", "level", "cold s", "warm s", "warm phases (s)");
    let mut rows: Vec<(String, f64, f64, Vec<(&'static str, f64)>)> = Vec::new();
    for level in &levels {
        // Cold: first build of this level in this process.
        nf_view::phases_reset();
        let t0 = Instant::now();
        let scene = nf_view::load(&iso, level, false).unwrap_or_else(|e| panic!("{level}: {e}"));
        let cold = t0.elapsed().as_secs_f64();
        assert!(scene.vertices.len() > 0, "{level}: the probe's scene must be non-vacuous");
        // Warm: the same level again — the page cache and the process-level tables are hot.
        nf_view::phases_reset();
        let t1 = Instant::now();
        let warm_scene = nf_view::load(&iso, level, false).unwrap_or_else(|e| panic!("{level} warm: {e}"));
        let warm = t1.elapsed().as_secs_f64();
        let phases = nf_view::phases_take();
        assert_eq!(scene.vertices.len(), warm_scene.vertices.len(), "{level}: warm must build the same scene");
        eprintln!("{:<10} {:>9.3} {:>9.3} | {}", level, cold, warm,
            phases.iter().map(|(n, s)| format!("{n}={s:.3}")).collect::<Vec<_>>().join(" "));
        rows.push((level.clone(), cold, warm, phases));
    }
    // The summary table the report quotes (levels sorted by warm total, worst first).
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    eprintln!("\n=== warm total, worst first ===");
    for (level, cold, warm, _) in &rows { eprintln!("{level:>10}: cold {cold:.3}s warm {warm:.3}s"); }
    // Phase totals across all levels (the "top costs" list for the report).
    let mut totals: Vec<(&'static str, f64)> = Vec::new();
    for (_, _, _, phases) in &rows {
        for (n, s) in phases {
            match totals.iter_mut().find(|(m, _)| m == n) { Some((_, v)) => *v += *s, None => totals.push((n, *s)) }
        }
    }
    totals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    eprintln!("\n=== phase totals over all levels (warm runs) ===");
    for (n, s) in &totals { eprintln!("{s:>9.3}s  {n}"); }
    assert!(totals.iter().any(|(n, _)| n.starts_with("07")), "the scene-build phase must be measured");
}
