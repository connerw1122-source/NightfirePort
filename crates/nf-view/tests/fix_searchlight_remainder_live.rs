//! fix-searchlight-remainder: the spot light's static housing upload, the broken-head smoke's game
//! stream, the tracking/lost-limit machine, and the corona alpha byte — through the real path
//! (`nf_view::load` + `Sim::step_bond`) on 07000006 (all four courtyard heads carry linked spots;
//! 07000005's data has only 2 spots for 7 heads — the game's own design, docs/WORKLOG.md this task).
//!
//! Opt-in via NF_ISO. Fail-first notes:
//! - `the_spot_light_shines_from_the_housing`: pre-fix `psiLight_SetLights`' port uploaded the LIVE
//!   spot position (a halo at the ground circle); the game uploads the node's static +0x10 (the
//!   housing) [M 0x10e588:34-38] and only measures the intensity from the live spot [M 0x1cb2e0].
//! - `the_broken_head_smoke_…`: pre-fix the puff's life was a guessed 24 ticks and NO shared draw
//!   was consumed [M the dropped 11th arg = 0x80 = 128 at 0x1cbce0 + the 6 draws].
//! - `…_tracks_…`: re-adds the removed searchlight-tracks-bond coverage (G-036 closed as a
//!   misdiagnosis — 07000006's four heads all track).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Wait out the level-start NIS, then the 0x06000066 boot's channel staging (64 @ 1, 63 @ 59,
/// 61 @ 116, 62 @ 167 [D stream 0]) until at least one head sweeps with a linked spot.
fn load_and_light(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..300 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 0 && p.links[2].is_some()) { break; }
    }
    scene
}

#[test]
fn the_spot_light_shines_from_the_housing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_and_light(&iso);
    let sim = scene.sim.as_mut().unwrap();
    // Walk the sweep until some linked spot lands (drawn) — its light record must upload the
    // HOUSING position while measuring the intensity at the LIVE spot.
    let mut found = false;
    for _ in 0..600 {
        sim.step_bond(Default::default(), false, false, false);
        let (idx, sp) = match sim.drones.searchlights.parts.iter().enumerate()
            .find(|(_, p)| p.kind == 2 && p.drawn) { Some(x) => x, None => continue };
        let housing = sim.drones.searchlights.parts[idx].spawn_pos;
        if (housing[0] - sp.pos[0]).powi(2) + (housing[2] - sp.pos[2]).powi(2) < 1.0 { continue; } // need a landed spot clearly off the housing
        let lights = sim.drones.lights.resolve(&|i: u32| {
            sim.drones.searchlights.parts.get(i as usize).map(|p| p.pos).unwrap_or([0.0; 3])
        });
        // The light record of THIS part (the records were pushed per kind-2 part, track = the part index).
        let k = lights.lights.iter().position(|l| l.track == Some(idx as u32)).expect("the kind-2 light record exists");
        let l = &lights.lights[k];
        assert_eq!(l.pos, [housing[0], housing[1], housing[2], 1.0],
            "psiLight_SetLights uploads the node's static +0x10 (the housing) [M 0x10e588:34-38]");
        assert_eq!(l.track_pos, [sp.pos[0], sp.pos[1], sp.pos[2], 1.0],
            "the intensity measurement follows the live spot [M 0x1cb2e0:41-44]");
        // An object at the spot: accepted (2D d ≤ 4) and lit FROM the housing — unless the game's own
        // 30-accept cap crowds the spot records out (they sit last in list order; the courtyard is
        // dense with kind-0 radiators, and the game fills the cap in the same list order [M 0x1cb2e0]).
        let accepted = lights.closest([sp.pos[0], sp.pos[1], sp.pos[2], 1.0], 0x80);
        if let Some((_, _f)) = accepted.iter().find(|(i, _)| lights.lights[*i].track == Some(idx as u32)) {
            let (lp, lc) = lights.pick2([sp.pos[0], sp.pos[1], sp.pos[2], 1.0], 0x80);
            let slot = (0..2).find(|&s| lp[s] == [housing[0], housing[1], housing[2], 1.0])
                .expect("an accepted spot light uploads the housing position");
            assert!(lc[slot][0] > 0.0, "the spot light reaches the object standing at the spot");
        } else {
            assert!(accepted.len() >= 30, "the spot light was crowded out only by the 30-accept cap");
        }
        found = true;
        break;
    }
    assert!(found, "a courtyard spot lands during the sweep (pre-fix the light rode the spot itself)");
}

#[test]
fn the_broken_head_smoke_uses_the_game_stream_and_life() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_and_light(&iso);
    let sim = scene.sim.as_mut().unwrap();
    // The real break path is a damaging bullet in obj+0xD0 (U-041's stand-in is the box test); the
    // state poke puts a lit head into the game's state 2 with the same downstream machinery the
    // update runs (the smoke gate + the droop).
    let head = sim.drones.searchlights.parts.iter().position(|p| p.kind == 1 && p.state != 3 && p.links[2].is_some())
        .expect("a lit courtyard head with a linked spot");
    sim.drones.searchlights.parts[head].state = 2;
    let mut spawn_frame = None;
    let mut snap_rng = None;
    for t in 0..40u32 {
        let f = sim.drones.frame;
        if sim.drones.gas.puffs.is_empty() {
            snap_rng = Some(sim.drones.rng.clone());
            sim.step_bond(Default::default(), false, false, false);
            if !sim.drones.gas.puffs.is_empty() {
                spawn_frame = Some((f, t));
                break;
            }
        } else { break; }
    }
    let (f, _t) = spawn_frame.expect("a broken drawn head smokes within one mod-7 window");
    // The step advanced the frame before the ctl ran, so the gate's frame is f+1 (one 60 Hz tick per
    // step_bond call; the bit-exact replay below passing proves the call ran exactly one tick).
    assert_eq!((f + 1 + sim.drones.searchlights.parts[head].created) % 7, 0,
        "the smoke fires on the game's (GameState+52 + obj+0xEC) % 7 phase [M 0x1cb900:375]");
    let puff = &sim.drones.gas.puffs[0];
    assert_eq!(puff.life, 128.0, "the decomp-dropped 11th arg = 0x80 ticks [M 0x1cbce0], not the old guessed 24");
    assert_eq!(puff.kind, 7);
    // The 6 game draws, replayed on the pre-tick snapshot after the k other draws that tick made
    // before the searchlight's ctl, reproduce the puff bit for bit. (Each draw — frand or rand —
    // advances the stream once, so k filler draws land on the same state regardless of their kind.)
    // Pre-fix the fields came from a counter hash: no offset k can reproduce them.
    let snap = snap_rng.expect("snapshot");
    let rec = 1.0f32 / 60.0;
    let expect = |r: &mut nf_game::drone::Rng| {
        let _rot_z = r.frand(std::f32::consts::PI);
        let spin = r.frand(std::f32::consts::FRAC_PI_2 * rec);
        let sign = r.rand(0x100);
        let (rx, ry, rz) = (r.frand(0.2), r.frand(0.2), r.frand(0.2));
        (if sign >= 0x80 { -spin } else { spin },
         [nf_formats::ps2float::mul(rx - 0.1, rec), nf_formats::ps2float::mul(ry, rec), nf_formats::ps2float::mul(rz - 0.1, rec)])
    };
    let mut hit = None;
    for k in 0..1024u32 {
        let mut r = snap.clone();
        for _ in 0..k { r.frand(1.0); }
        let (spin, vel) = expect(&mut r);
        if spin.to_bits() == puff.spin.to_bits() && vel.iter().zip(puff.vel.iter()).all(|(a, b)| a.to_bits() == b.to_bits()) { hit = Some(k); break; }
    }
    let k = hit.expect("the puff's fields = the game's 6 draws at a fixed stream offset (pre-fix: a counter hash, no such k)");
    eprintln!("[smoke] the 6 draws sit {k} draws into the tick");
}

#[test]
fn a_courtyard_head_tracks_bond_until_the_lost_limit() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_and_light(&iso);
    // Ride a sweep: teleport Bond onto the first landed linked spot each tick until the head arms
    // (the 0→1 test is spot-only [M 0x1cb900:251-263]).
    let head = loop {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        match sim.drones.searchlights.parts.iter().enumerate()
            .find(|(_, p)| p.kind == 1 && p.state == 0 && p.links[2].is_some()) {
            Some((h, _)) => break h,
            None if sim.drones.frame > 1200 => panic!("no sweeping head with a linked spot"),
            None => continue,
        }
    };
    let mut armed_at = None;
    for t in 0..900u32 {
        let sim = scene.sim.as_mut().unwrap();
        let spot = sim.drones.searchlights.parts[head].links[2].unwrap();
        let sp = sim.drones.searchlights.parts[spot].clone();
        if sp.drawn { sim.bond.pos = sp.pos; sim.bond.yaw = 0.0; }
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts[head].state == 1 { armed_at = Some(t); break; }
    }
    armed_at.expect("riding the spot arms the head (state 1)");

    // The head now steers at Bond; walk him slowly and the spot follows (the searchlight-tracks-bond
    // behaviour, re-added — G-036 closed).
    let mut tracked = 0;
    for _ in 0..120u32 {
        let sim = scene.sim.as_mut().unwrap();
        let spot = sim.drones.searchlights.parts[head].links[2].unwrap();
        {
            let sp = &sim.drones.searchlights.parts[spot];
            let d = ((sp.pos[0] - sim.bond.pos[0]).powi(2) + (sp.pos[2] - sim.bond.pos[2]).powi(2)).sqrt();
            if d > 1.2 { sim.bond.pos = [sp.pos[0], sim.bond.pos[1], sp.pos[2]]; }
        }
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts[head].state == 1 { tracked += 1; }
    }
    assert_eq!(tracked, 120, "the head holds state 1 while Bond walks with the spot (tracking)");

    // The corona alpha byte reaches the colour table: searchlight part vertices stopped sharing one
    // 1.0 slot (pre-fix every part used `col_one`). The expected alpha is read from the part at the
    // same moment the palette is built (the byte updates every drawn tick).
    let (layout, attrs) = scene.skin_layout();
    let sl_cols: Vec<u32> = attrs.iter().filter_map(|a| {
        // The searchlight vertices are the ones whose matrix slot sits in the part region.
        if a.a >= layout.sl_base && a.a < layout.door_base { Some(a.col) } else { None }
    }).collect();
    assert!(sl_cols.iter().any(|&c| c != sl_cols[0]),
        "the parts no longer share one colour slot (the corona's alpha byte has somewhere to live)");
    let (_, cols) = scene.skin_palette(&layout, None, 0.0, Default::default(), None);
    let (alpha, corona_idx) = {
        let sim = scene.sim.as_ref().unwrap();
        let ci = sim.drones.searchlights.parts[head].links[4].unwrap();
        (sim.drones.searchlights.parts[ci].alpha, ci)
    };
    assert!(alpha < 255, "the tracking corona carries the game's faded alpha byte");
    let ca = cols[(layout.col_sl + corona_idx as u32) as usize][3];
    assert!((ca - alpha as f32 / 255.0).abs() < 1e-6, "the corona's colour slot .a = obj+0x106/255 [M 0x1cb900:334-336]");

    // Bond walks away: the spot's lost counter runs to the game's FRAME_RATE·15 = 900 ticks, then
    // the head returns to the sweep [M 0x1cb900:254-263].
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [sim.bond.pos[0] + 80.0, sim.bond.pos[1], sim.bond.pos[2] + 80.0];
    let mut back = None;
    for t in 1..=960u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts[head].state == 0 { back = Some(t); break; }
    }
    let back = back.expect("the head gives up and returns to the sweep");
    assert!((890..=940).contains(&back), "the lost limit is the game's 900 ticks (got {back})");
}
