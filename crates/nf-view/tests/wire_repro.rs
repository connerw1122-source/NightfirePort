//! fix-wire repro (pre-fix evidence): Bond standing next to 07000005's cable, pressing E. This probe
//! only uses the pre-fix API (load + step_bond), so it runs against both the old and the new code.
//! Old code: no wire substate exists — the substate stays 0 (walk) and nothing grabs (the user's
//! "pressing E plays the falling-to-death animation" — he walks off the cliff instead of crossing).
//! New code: the grab scan puts him on the wire (substate 6, the grab pose).
//! Run with: `NF_ISO=… cargo test -p nf-view --test wire_repro -- --nocapture`

use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn repro_e_near_the_07000005_cable() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).expect("level");
    let _data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().expect("sim");
    // The wire: placement (−23.4855, 34.4676, 6.2629) [D placements probe]. The grab zone is the wire's
    // world sphere (centre ± 4.04); put Bond 1.1 under the wire's centre — inside it, standing.
    let (wx, wy, wz) = (-23.4855, 34.4676, 6.2629);
    sim.bond.pos = [wx, wy - 1.1, wz];
    sim.bond.yaw = 0.0;
    let mut grabbed = false;
    for t in 0..30 {
        sim.step_bond(Default::default(), false, false, true); // the E key held this tick
        if t == 0 || t == 29 { eprintln!("tick {t}: substate {} pos {:?}", sim.bond.substate, sim.bond.pos); }
        if sim.bond.substate != 0 { grabbed = true; break; }
    }
    eprintln!("grabbed: {grabbed}");
    assert!(grabbed, "pressing E next to the cable must attach Bond to the wire");
}
