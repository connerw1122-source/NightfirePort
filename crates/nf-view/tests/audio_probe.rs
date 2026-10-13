//! fix-audio2 probe (not a pass/fail test): what the cut-scene scripts ask for, and what the bank has.
//!
//! Dumps every op-0x16 `Script_SoundStart` in every script of the level lib (type-7 entries + the loadable
//! cut-scene packs), then resolves each id against the level's bank (SB_x): the effect's SFXParameters
//! (`pause_in_nis` included) and its sub-track FileRefs (negative = a STREAMS.BIN entry).
//! Opt-in via NF_ISO: `NF_ISO=... cargo test --release -p nf-view --test audio_probe -- --nocapture`.

use nf_formats::nis::Cmd;

fn scripts_report(tag: &str, scene: &nf_view::scene::Scene) {
    let sim = scene.sim.as_ref().unwrap();
    let bank = scene.sound_bank.as_ref().map(|(_, b, _)| b);
    let lut_len = scene.streams.as_ref().map(|(l, _)| l.len()).unwrap_or(0);
    println!("== {tag}: {} scripts, bank {}, STREAMS.LUT {} entries", sim.drones.scripts.lib.len(),
        if bank.is_some() { "loaded" } else { "MISSING" }, lut_len);
    let mut hashes: Vec<_> = sim.drones.scripts.lib.keys().copied().collect();
    hashes.sort();
    println!("all hashes: {:08X?}", hashes);
    // The referenced STREAMS.BIN entries: header + end marker → duration.
    if let Some((lut, bin)) = scene.streams.as_ref() {
        let mut refs = std::collections::BTreeSet::new();
        for h in &hashes {
            for s in &sim.drones.scripts.lib[h].streams {
                for c in s.decode() { if let Cmd::Sound { id, .. } = c {
                    if let Some(Ok(e)) = bank.and_then(|b| b.effect(id)).map(nf_formats::soundbank::Effect::parse) {
                        for st in &e.subs[..e.count] { if st.file < 0 { refs.insert((-1 - st.file) as usize); } }
                    }
                } }
            }
        }
        for k in refs {
            let Some(&[ho, hs, do_, ds]) = lut.get(k) else { println!("  LUT[{k}] OUT OF RANGE"); continue };
            // End marker: header word at +0x48..+0x50 is marker 1 {id −1, pos, type 9, …} (speech.md §4.2).
            let h = bin.get(ho as usize..ho as usize + 0x70).map(|b| b.to_vec());
            let dur = h.map(|b| {
                let pos = u32::from_le_bytes([b[0x4c], b[0x4d], b[0x4e], b[0x4f]]) as usize;
                let typ = u32::from_le_bytes([b[0x50], b[0x51], b[0x52], b[0x53]]);
                let delivered = pos & !0xff;
                format!("end marker pos {pos} type {typ} → {:.2} s (LUT dataSize {ds})", delivered as f64 * 7.0 / 8.0 / 22050.0)
            });
            println!("  LUT[{k}]: header@{ho} size {hs} data@{do_} size {ds} {}", dur.unwrap_or("header unreadable".into()));
        }
    }
    for h in hashes {
        let nis = &sim.drones.scripts.lib[&h];
        let mut lines = Vec::new();
        // The anim scripts this NIS starts (`Script_AnimStart`), with their op-1 sounds / op-4 events.
        let mut anim_hashes = std::collections::BTreeSet::new();
        for (si, s) in nis.streams.iter().enumerate() {
            for c in s.decode() {
                if let Cmd::Anim { script, .. } = c { anim_hashes.insert(script); }
                if let Cmd::Sound { id, flat, has_obj, stream } = c {
                    // Resolve: the bank's effect block for the id.
                    let res = bank.and_then(|b| b.effect(id).map(nf_formats::soundbank::Effect::parse));
                    match res {
                        Some(Ok(e)) => {
                            let files: Vec<String> = e.subs[..e.count].iter().map(|s| {
                                if s.file < 0 { format!("stream LUT[{}]", -1 - s.file) } else { format!("sample {}", s.file) }
                            }).collect();
                            lines.push(format!("  st{si}: id {id} (flat {flat} obj {has_obj} streamarg {stream}) track {} inner {} outer {} maxv {} pause_nis {} loop {} → {}",
                                e.tracking, e.inner, e.outer, e.max_voices, e.pause_in_nis, e.looped, files.join("|")));
                        }
                        Some(Err(e)) => lines.push(format!("  st{si}: id {id} → effect PARSE ERROR {e}")),
                        None => lines.push(format!("  st{si}: id {id} → NOT IN BANK")),
                    }
                }
            }
        }
        if !lines.is_empty() || !anim_hashes.is_empty() {
            println!("script {:08X} len {} streams {}", h, nis.length, nis.streams.len());
            for l in lines { println!("{l}"); }
            for a in anim_hashes {
                match sim.scripts.get(&a) {
                    Some(asx) => {
                        let snds: Vec<String> = asx.commands.iter().filter(|c| c.op == 1 || c.op == 4).map(|c| {
                            if c.op == 1 {
                                let sfx = *c.args.get(1).unwrap_or(&0) as u32;
                                let res = bank.and_then(|b| b.effect(sfx).map(nf_formats::soundbank::Effect::parse));
                                let what = match res {
                                    Some(Ok(e)) => e.subs[..e.count].iter().map(|s| if s.file < 0 { format!("stream LUT[{}]", -1 - s.file) } else { format!("sample {}", s.file) }).collect::<Vec<_>>().join("|"),
                                    Some(Err(_)) => "PARSE ERR".into(),
                                    None => "NOT IN BANK".into(),
                                };
                                format!("op1 f{} sfx {sfx} → {what}", c.args.first().unwrap_or(&0))
                            } else {
                                format!("op4 {:?} @f{}", c.args.get(1..), c.args.first().unwrap_or(&0))
                            }
                        }).collect();
                        println!("  anim {:08X} len {} cmds {}: {}", a, asx.length, asx.commands.len(),
                            if snds.is_empty() { "no op1/op4".into() } else { snds.join("; ") });
                    }
                    None => println!("  anim {:08X} MISSING from the anim set", a),
                }
            }
        }
    }
}

#[test]
fn probe() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    for tag in ["07000001.bin", "07000005.bin", "07000007.bin"] {
        let Ok(scene) = nf_view::load(&iso, tag, false) else { println!("== {tag}: LOAD FAILED"); continue };
        scripts_report(tag, &scene);
    }
}

#[test]
fn probe_players() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    for tag in ["07000001.bin", "07000005.bin", "07000007.bin"] {
        let Ok(scene) = nf_view::load(&iso, tag, false) else { continue };
        let sim = scene.sim.as_ref().unwrap();
        println!("== {tag} players:");
        for p in &sim.drones.scripts.players {
            println!("  placement {} state {} scripts {:08X?} gate {:?}", p.placement, p.state,
                p.scripts.iter().map(|h| scene.sim.as_ref().map(|_| *h).unwrap_or(0)).collect::<Vec<_>>(), "");
        }
    }
}

#[test]
fn probe_shot() {
    use nf_game::player::Input;
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    let sim0 = scene.sim.as_ref().unwrap();
    let r = &sim0.weapons.records[2];
    let f32at = |o: usize| f32::from_le_bytes([r[o], r[o+1], r[o+2], r[o+3]]);
    println!("weapon2 range {} speed {} spread {}", f32at(0x1c), f32at(0x20), f32at(0x24));
    let gpos = sim0.drones.list[6].pos;
    for dy in [0.2f32, 0.4, 0.6, 0.8, 1.0, 1.2] {
        let mut parts = Vec::new();
        { let sim = scene.sim.as_mut().unwrap();
          sim.bond.pos = [gpos[0] - 2.5, gpos[1] + dy, gpos[2]];
          sim.bond.eye = 0.0;
          sim.bond.yaw = std::f32::consts::FRAC_PI_2;
          sim.bond.pitch = 0.0;
          for _ in 0..10 {
            sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
            for i in &sim.drones.player_impacts { parts.push(match i {
                nf_game::drone::PlayerImpact::Drone { part, .. } => format!("part{part}"),
                _ => "world".into() }); }
            sim.step_bond(Input::default(), false, false, false);
          }
        }
        println!("dy {dy}: {:?} health {}", parts, scene.sim.as_ref().unwrap().drones.list[6].health);
    }
}
