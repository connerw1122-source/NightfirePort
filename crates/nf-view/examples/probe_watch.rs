//! The weapon-matrix5 watch-pair probe: why does the mode press on the Micro-Camera 0x54 do
//! nothing on 07000009/0a/0b (weapon-matrix4's red #2) while it lands on the other levels?
//! The game's own input for the mode action ORs the FIRE edge in for this pair [M `Player_Weapon`
//! 0x1a24c8: `bVar5 = Input_Action(0xc,4); if (*w - 0x54 < 2) bVar5 |= Input_Action(9,4);`] —
//! this probe measures the cell both ways: the harness's plain press (mode_switch alone, what
//! matrix4 ran) and the OR'd press (mode_switch + the fire edge).

use nf_view::panel;
use nf_view::scene::Scene;

fn rec(sim: &nf_view::scene::Sim, id: usize) -> String {
    let Some(w) = sim.weapons.records.get(id) else { return format!("{id:#04x}: no record") };
    let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
    format!(
        "{id:#04x}: sel(+4)={} twin(+5)={} mcount(+0x2e)={} flags(+0x68)={:08x} atype(+0x90)={} per_shot(+0x91)={} clip(+0x92)={} fp(+0xdc)={:08x}",
        u16::from_le_bytes([w[4], w[5]]), w[5], w[0x2e], u32_at(0x68), w[0x90], w[0x91], u16::from_le_bytes([w[0x92], w[0x93]]), u32_at(0xdc)
    )
}

fn inv(sim: &nf_view::scene::Sim, id: u8) -> String {
    let t54 = sim.weapons.ammo_type(id as usize);
    let own = sim.drones.inventory.owned.get(id as usize).copied().unwrap_or(false);
    let cl = sim.drones.inventory.clip.get(id as usize).copied().unwrap_or(-1);
    let car = sim.drones.inventory.carried.get(t54).copied().unwrap_or(-1);
    let fm = sim.drones.inventory.fire_mode.get(id as usize).copied().unwrap_or(0);
    format!("owned={own} clip[{id:#04x}]={cl} atype={t54} carried[{t54}]={car} fire_mode={fm}")
}

fn main() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let level = std::env::var("PROBE_LEVEL").unwrap_or_else(|_| "07000009".into());
    let level = if level.contains('.') { level } else { format!("{level}.bin") };

    // The merged load (the same tables the matrix cell runs on).
    let mut scene: Scene = nf_view::load(&iso, &level, false).expect("load");
    let ids: Vec<u8> = panel::catalog_entries(&scene).into_iter().map(|e| e.id).collect();
    scene = nf_view::load_weapons(&iso, &level, false, &ids).expect("merged load");

    // Wait the intro out.
    for _ in 0..7200 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }

    let sim = scene.sim.as_ref().unwrap();
    eprintln!("=== records ===\n{}\n{}", rec(sim, 0x54), rec(sim, 0x55));
    eprintln!("=== inventory 0x54: {} ===", inv(sim, 0x54));
    eprintln!("=== inventory 0x55: {} ===", inv(sim, 0x55));
    eprintln!("held={:02x} disabled={}", sim.gun.weapon, sim.disabled);

    // Cycle to 0x54 the way the cell does (the gadget wheel).
    let mut held54 = false;
    for _ in 0..24 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..50 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            if scene.sim.as_ref().unwrap().gun.weapon == 0x54 { held54 = true; break; }
        }
        if held54 { break; }
    }
    eprintln!("=== cycled to 0x54: {held54} ===");
    if !held54 { return; }
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }

    // Phase 0: the FIRST fire press, A/B by style (PROBE_STYLE=a = the harness's trigger-only
    // press; b = the real-path shape: lib.rs feeds `fire: self.trigger` into Input AND the
    // trigger arg). From the clean idle machine, one press: does the swap land, does the
    // throw spawn, does the machine stick?
    let style = std::env::var("PROBE_STYLE").unwrap_or_else(|_| "b".into());
    let b0 = scene.sim.as_ref().unwrap().drones.player_bullets.len();
    let g0 = scene.sim.as_ref().unwrap().gun.clip;
    let (w0, fm0) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    match style.as_str() {
        "a" => { scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false); }
        _ => { scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { fire: true, ..Default::default() }, true, false, false); }
    }
    for _ in 0..80 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let b1 = scene.sim.as_ref().unwrap().drones.player_bullets.len();
    let g1 = scene.sim.as_ref().unwrap().gun.clip;
    let (w1, fm1) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    {
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("=== FIRST fire press (style {style}): before w={w0:#04x} fm={fm0} bullets={b0} gunclip={g0} → after w={w1:#04x} fm={fm1} bullets={b1} gunclip={g1} → {} / {} ===",
            if w1 != 0x54 || fm1 != fm0 { "SWAP landed" } else { "no swap" },
            if b1 > b0 { "THROW spawned" } else { "no throw" });
        eprintln!("    state: sw={} rel={} fire={} vs={:#x}/{:.1} held={:02x}", sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8, sim.view_script, sim.view_time, sim.gun.weapon);
    }
    // Re-idle with the harness's stale repair (frozen finished script → re-queue the idle).
    for _ in 0..200 {
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state {
            let (s, t) = (sim.view_script, sim.view_time);
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            let mut sim = scene.sim.as_mut().unwrap();
            if sim.view_script == s && sim.view_time == t && t == 0.0 && s != 0 {
                let idle = sim.weapons.idle_script(sim.gun.weapon as usize);
                sim.view_script = idle; sim.view_time = 0.0; sim.view_speed = 1.0;
            }
            continue;
        }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("=== re-idled: held={:02x} sw={} rel={} fire={} vs={:#x}/{:.1} ===", sim.gun.weapon, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8, sim.view_script, sim.view_time);
    }

    // Fire ×2 (throw the camera) then settle — the cell's state before the 4.5 press.
    for _ in 0..2 {
        for _ in 0..45 { scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false); }
        for _ in 0..8 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    }
    // Ride the machine to the IDLE VIEW (the harness's wait_idle shape: machine idle + the view
    // not mid-fire-script) — the mode gate refuses otherwise.
    for _ in 0..400 {
        let sim = scene.sim.as_ref().unwrap();
        let idle = sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state;
        if idle { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("=== after fire+settle: held={:02x} sw={} rel={} fire={} vs={:#x}/{:.1} ===",
            sim.gun.weapon, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8, sim.view_script, sim.view_time);
        eprintln!("    {}", inv(sim, 0x54));
    }

    // Attempt 1: the FIRE-EDGE press alone (the game's action-9 OR arm — scene.rs's `toggled`):
    // mode_switch=false, input.fire=true, trigger=true. The game's mode block reads
    // action12 | action9 for this pair, so a fire press reaches it.
    let (w0, fm0) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { fire: true, ..Default::default() }, true, false, false);
    for _ in 0..80 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (w1, fm1) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    eprintln!("=== FIRE-EDGE press: before w={w0:#04x} fm={fm0} → after w={w1:#04x} fm={fm1} → {} ===",
        if w1 != 0x54 || fm1 != fm0 { "LANDED" } else { "nothing" });
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("    after-edge: sw={} rel={} fire={} vs={:#x}/{:.1}", sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8, sim.view_script, sim.view_time);

    // Bring the watch back if the edge press moved it, then re-idle.
    if w1 != 0x54 {
        for _ in 0..24 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
            for _ in 0..50 {
                scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
                if scene.sim.as_ref().unwrap().gun.weapon == 0x54 { break; }
            }
            if scene.sim.as_ref().unwrap().gun.weapon == 0x54 { break; }
        }
    }
    for _ in 0..400 {
        let sim = scene.sim.as_ref().unwrap();
        let idle = sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state;
        if idle { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }

    // Attempt 2: the harness's plain press (mode_switch alone) — what matrix4 ran.
    let (w0, fm0) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..80 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (w1, fm1) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    eprintln!("=== PLAIN press: before w={w0:#04x} fm={fm0} → after w={w1:#04x} fm={fm1} → {} ===",
        if w1 != 0x54 || fm1 != fm0 { "LANDED" } else { "nothing" });

    // If the plain press moved the weapon away, bring it back before attempt 2.
    if w1 != 0x54 {
        for _ in 0..24 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
            for _ in 0..50 {
                scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
                if scene.sim.as_ref().unwrap().gun.weapon == 0x54 { break; }
            }
            if scene.sim.as_ref().unwrap().gun.weapon == 0x54 { break; }
        }
        for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    }

    // Attempt 2: the game's own OR'd input — mode_switch AND the fire edge on the same tick
    // (the game reads action 12 | action 9 for this pair [M 0x1a24c8]).
    let (w0, fm0) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { mode_switch: true, fire: true, ..Default::default() }, true, false, false);
    for _ in 0..80 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (w1, fm1) = { let sim = scene.sim.as_ref().unwrap();
        (sim.gun.weapon, sim.drones.inventory.fire_mode.get(0x54).copied().unwrap_or(0)) };
    eprintln!("=== OR'D press (mode+fire): before w={w0:#04x} fm={fm0} → after w={w1:#04x} fm={fm1} → {} ===",
        if w1 != 0x54 || fm1 != fm0 { "LANDED" } else { "nothing" });
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("=== final: {} ===", inv(sim, 0x54));
    eprintln!("=== final 0x55: {} ===", inv(sim, 0x55));
}
