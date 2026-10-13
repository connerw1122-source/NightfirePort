//! fix-final-blockers2 probe 2: the static NIS 0x060006cf — streams 24..27's decoded commands and
//! the key tables (times + positions) for the marker streams, plus the same for the lift script
//! 0x06000571 stream 0 (the cab). Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

use nf_formats::nis::Stream as _;
use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn dump_nis_060006cf_marker_streams() {
    let iso = iso();
    let scene = nf_view::load(&iso, "0700004a.bin", false).unwrap();
    let lib = &scene.sim.as_ref().unwrap().drones.scripts.lib;
    for hash in [0x060006cfu32, 0x06000571] {
        let Some(nis) = lib.get(&hash) else { eprintln!("{hash:08x}: not in lib"); continue };
        let nis = &**nis;
        eprintln!("=== {:08x} len {} loop {} streams {} keys {}", hash, nis.length, nis.loop_start, nis.streams.len(), nis.keys.len());
        for (i, st) in nis.streams.iter().enumerate() {
            if hash == 0x060006cf && ![0, 26].contains(&i) { continue; }
            if hash == 0x06000571 && i > 1 { continue; }
            eprintln!("-- stream {i} fk {} lk {} cmds {}", st.first_key, st.last_key, st.commands.len());
            for c in st.decode() {
                eprintln!("   {c:?}");
            }
            if hash == 0x060006cf && false {
                for (ki, k) in nis.keys[st.first_key as usize..=st.last_key as usize].iter().enumerate() {
                    eprintln!("   key[{}] time {:.2} pos {:?} quat {:?} scale {:.3} w24 {:?} w28 {:.3}", st.first_key as usize + ki, k.time, k.pos, k.quat, k.scale, k.w24, k.w28);
                }
            }
        }
    }
}
