//! The weapon-matrix5 sweep runner: runs `matrix::run` over the levels in PROBE_LEVEL (a comma
//! list, default the whole on-foot set) and prints the mark table, the red cells with their
//! measurements, and the per-weapon aggregate — the report's raw data.

fn main() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let levels: Vec<String> = std::env::var("PROBE_LEVEL").ok().map(|v| {
        v.split(',').map(|s| { let s = s.trim(); if s.contains('.') { s.to_string() } else { format!("{s}.bin") } }).collect()
    }).unwrap_or_default();
    let wanted: Option<Vec<String>> = if levels.is_empty() { None } else { Some(levels) };

    let mut lines = String::new();
    let mut log = |s: &str| { lines.push_str(s); lines.push('\n'); };
    let rows = nf_view::matrix::run(&iso, wanted.as_deref(), &mut log).expect("matrix run");
    eprint!("{lines}");
    let total: usize = rows.iter().map(|r| r.cells.len()).sum();
    let reds: usize = rows.iter().map(|r| r.cells.iter().filter(|c| { let m = c.mark(); m != '-' && m != 'n' }).count()).sum();
    let panics: usize = rows.iter().map(|r| r.cells.iter().filter(|c| c.error.is_some()).count()).sum();
    eprintln!("cells: {total} levels: {} reds: {reds} panics: {panics}", rows.len());
    eprint!("{}", nf_view::matrix::mark_table(&rows));
    eprintln!("=== RED CELLS ===");
    eprint!("{}", nf_view::matrix::red_cells(&rows));
    eprintln!("=== AGGREGATE ===");
    eprint!("{}", nf_view::matrix::aggregate_table(&rows));
}
