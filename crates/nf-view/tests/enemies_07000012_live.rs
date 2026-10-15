//! fix-12-enemies repro: on 07000012 the user saw "killable MP, unkillable NPCs, like enemies"
//! (playtest 2026-10-15). Fire at every guard through the real input path (`nf_view::load` +
//! `Sim::step_bond`) and find which ones never take damage. Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Tick the sim idle.
fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Stand armed Bond at `at` aiming the eye at `to` (chest height).
fn stand_armed_aiming(sim: &mut nf_view::scene::Sim, at: [f32; 3], to: [f32; 3]) {
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
    sim.gun.weapon = 2;
    sim.gun.switch_request = 2;
    sim.gun.switch_state = 0;
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    sim.bond.yaw = d[0].atan2(d[2]);
    let h = (d[0] * d[0] + d[2] * d[2]).sqrt();
    sim.bond.pitch = d[1].atan2(h) / std::f32::consts::FRAC_PI_2;
}

/// Probe 1: dump every drone's state at load.
#[test]
fn probe_dump_drones_at_load() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 10);
    eprintln!("npc_enabled={} spawners={}", sim.drones.npc_enabled, sim.drones.spawners.len());
    for (i, dr) in sim.drones.list.iter().enumerate() {
        eprintln!(
            "drone {:2}: skin {:08x} kind {} group {} present {} enabled {} corpse {} state {:#04x} pos ({:7.2},{:6.2},{:7.2}) hp {:5.1} dmgble {} dmg_taken {:.2} ch {}",
            i, dr.skin, dr.kind, dr.group, dr.present, dr.enabled, dr.corpse, dr.state,
            dr.pos[0], dr.pos[1], dr.pos[2], dr.health, dr.damageable, dr.damage_taken, dr.switch_channel);
    }
    let boxes: Vec<usize> = (0..sim.drones.list.len())
        .filter(|&i| sim.drones.list[i].present && !sim.drones.list[i].corpse)
        .filter(|&i| {
            let data = sim.data();
            sim.drones.list[i].hit_boxes(&data).is_empty()
        })
        .collect();
    eprintln!("present drones with EMPTY hit boxes: {boxes:?}");
}

/// Probe 2: stand Bond next to each present, enabled, living drone and fire at its chest through
/// the real input path; report whether it takes damage.
#[test]
fn probe_fire_at_every_present_drone() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 10);
    let n = sim.drones.list.len();
    let mut report = Vec::new();
    for i in 0..n {
        let (present, enabled, corpse, hp0) = {
            let dr = &sim.drones.list[i];
            (dr.present, dr.enabled, dr.corpse, dr.health)
        };
        if !present || corpse { report.push((i, format!("present={present} corpse={corpse} — skipped"))); continue; }
        // Park Bond 3 units to the drone's −x, aim at its chest.
        let p = sim.drones.list[i].pos;
        let at = [p[0] - 3.0, p[1], p[2]];
        stand_armed_aiming(sim, at, [p[0], p[1] + 1.0, p[2]]);
        tick_idle(sim, 2);
        // Fire held for 6 ticks (the P2K's semi-auto fires on the press; the hold re-fires).
        for _ in 0..6 {
            sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        }
        tick_idle(sim, 6);
        let dr = &sim.drones.list[i];
        let dmg = hp0 - dr.health;
        report.push((i, format!(
            "state {:#04x} enabled {} hp {:.1}→{:.1} (dmg {:.2}) hit_by_player {} hits {}",
            dr.state, dr.enabled, hp0, dr.health, dmg, dr.hit_by_player, dr.hits_taken)));
    }
    for (i, line) in &report { eprintln!("drone {:2}: {}", i, line); }
}
/// scratch probe: does Bond fire at all on 07000012?
#[test]
fn probe3_bullet_spawns() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("no NF_ISO"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    let p = sim.drones.list[9].pos;
    let at = [p[0] - 3.0, p[1], p[2]];
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, at, 0.0);
    sim.gun.weapon = 2;
    sim.gun.switch_request = 2;
    sim.gun.switch_state = 0;
    let e = sim.bond.head();
    let d = [p[0] - e[0], p[1] + 1.0 - e[1], p[2] - e[2]];
    sim.bond.yaw = d[0].atan2(d[2]);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    eprintln!("gun.weapon={} switch_state={} bond.pos=({:.2},{:.2},{:.2})", sim.gun.weapon, sim.gun.switch_state, sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]);
    let b0 = sim.drones.player_bullets.len();
    sim.step_bond(nf_game::player::Input { fire: true, ..Default::default() }, true, false, false);
    eprintln!("after 1 fire tick: player_bullets {} -> {}", b0, sim.drones.player_bullets.len());
    for b in &sim.drones.player_bullets { eprintln!("  bullet pos ({:.2},{:.2},{:.2}) weapon {} dmg {:.2}", b.pos[0], b.pos[1], b.pos[2], b.weapon, b.damage); }
    for _ in 0..4 { sim.step_bond(nf_game::player::Input { fire: true, ..Default::default() }, true, false, false); }
    let dr = &sim.drones.list[9];
    eprintln!("drone9: state {:#x} hp {} hits_taken {} hit_by_player {}", dr.state, dr.health, dr.hits_taken, dr.hit_by_player);
    eprintln!("impacts this tick: {:?}", sim.drones.impacts.len());
}

/// Probe 4: the touch plates and their channels/boxes.
#[test]
fn probe4_triggers() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("no NF_ISO"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for (k, t) in sim.drones.triggers.iter().enumerate() {
        if t.placement < 20 || t.placement > 45 { continue; }
        eprintln!("trigger {k}: placement {} kind {} out {:#x} mask {:#x} lock {} list {:?}", t.placement, t.kind, t.out, t.mask, t.lock, t.list);
    }
    for (k, t) in sim.drones.triggers.iter().enumerate() {
        if t.kind == 1 && t.mask & 8 != 0 && t.out >= 0x01 && t.out <= 0x09 {
            eprintln!("waking touch {k}: placement {} out {:#x} pos {:?}", t.placement, t.out, sim.drones.triggers[k].list);
        }
    }
}
