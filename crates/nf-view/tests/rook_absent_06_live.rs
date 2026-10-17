//! rook-door-06: every 07000006 drone — mode/team/start_state/props 0x24/0x25 — who is the lead?
//! Opt-in via NF_ISO.

#[test]
fn probe_06_all_drones() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, None).expect("load 07000006");
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let d = nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
        skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
        hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
        surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
    };
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if dr.corpse { continue; }
        let st = dr.start_state(&d);
        eprintln!("[drone #{i}] skin {:#x} kind {} mode {} team {} present {} enabled {} state {:#x} switch_ch {} alt_state {:#x} start_state {:#x} pos ({:.1},{:.1},{:.1}) route {}",
            dr.skin, dr.kind, dr.mode, dr.team, dr.present as u8, dr.enabled as u8, dr.state, dr.switch_channel, dr.alt_state, st,
            dr.pos[0], dr.pos[1], dr.pos[2],
            dr.route.as_ref().map(|r| format!("path {} {} nodes", r.path, r.nodes.len())).unwrap_or("-".into()));
    }
}
