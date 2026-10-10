//! `nf-view`: level viewer for 007: Nightfire (PS2). Binary entry; all logic is in the library crate
//! (`nf_view`) so that headless tests (`crates/nf-view/tests/`) can drive the real Sim through the same path the
//! window uses (docs/tools/dev-mode.md: input record/replay).

fn main() -> anyhow::Result<()> { nf_view::run() }
