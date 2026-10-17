//! rook-door-06: the ScriptPlayers of 07000006 — who watches ch 185 (0xB9, the truck-arrival
//! plates) and what their scripts DO (events / channel writes / sounds / subtitles). The lead
//! walk's trigger should be in here. Opt-in via NF_ISO.

#[test]
fn probe_06_script_players() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, None).expect("load 07000006");
    // Let the level settle so the start scripts activate and parse.
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    for (i, sp) in sim.drones.scripts.players.iter().enumerate() {
        let watch185 = (sp.mask >> 0) != 0 && (sp.mask as u32 >> 185 & 1 != 0 || sp.mask as u32 >> (185 - 16) & 1 != 0);
        let gate185 = sp.gate == 185 || sp.out == 185;
        if !watch185 && !gate185 {
            // still print SPs whose script is loaded and whose out/gate near the greeting, terse
            continue;
        }
        eprintln!("[SP #{i}] placement {} scripts {:?} modes {:?} mask {:#x} out {} gate {} stay {}",
            sp.placement, sp.scripts, sp.modes, sp.mask, sp.out, sp.gate, sp.stay as u8);
    }
    // Everything with a loaded script: decode all streams' commands.
    eprintln!("---- all activated SPs ----");
    for (i, sp) in sim.drones.scripts.players.iter().enumerate() {
        let mut any = false;
        for (k, info) in sp.info.iter().enumerate() {
            let Some(s) = info else { continue };
            any = true;
            for (si, st) in s.nis.streams.iter().enumerate() {
                for cmd in st.decode() {
                    match &cmd {
                        nf_formats::nis::Cmd::Event { kind, params } => {
                            eprintln!("[SP #{i} s{k} st{si}] EVENT kind {kind:#04x} params {params:?}");
                        }
                        nf_formats::nis::Cmd::Sound { id, .. } => {
                            eprintln!("[SP #{i} s{k} st{si}] SOUND id {id:#x}");
                        }
                        nf_formats::nis::Cmd::Text { label, time } => {
                            eprintln!("[SP #{i} s{k} st{si}] TEXT label {label:#x} time {time}");
                        }
                        nf_formats::nis::Cmd::Anim { script, skin, start } => {
                            eprintln!("[SP #{i} s{k} st{si}] ANIM script {script:#x} skin {skin:#x} start {start:#x}");
                        }
                        _ => {}
                    }
                }
            }
        }
        if any && (sp.mask != 0 || sp.out != 0 || sp.gate != 0) {
            eprintln!("[SP #{i}] placement {} scripts {:?} modes {:?} mask {:#x} out {} gate {}",
                sp.placement, sp.scripts, sp.modes, sp.mask, sp.out, sp.gate);
        }
    }
}
