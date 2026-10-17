//! Scratch probe (not committed): debug the pane shot and the corridor shot in 07000008.

use nf_game::player::Input;

fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let mut scene = nf_view::load_weapons(&iso_path, "07000008.bin", false, &[0x00]).expect("load 08");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
        println!("gun weapon {} clip {} spare {} request {:?}", sim.gun.weapon, sim.gun.clip, sim.gun.spare, sim.gun.switch_request);
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(2).max(1);
        sim.drones.inventory.equip_weapon(&wd, 2, rounds, true);
        sim.drones.inventory.equip_ammo(&wd, 2, rounds * 5);
        sim.gun.switch_request = 2;
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        println!("gun weapon {} clip {} spare {} rounds {}", sim.gun.weapon, sim.gun.clip, sim.gun.spare, rounds);
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [107.25003, 35.3124, -118.49846], 0.0);
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        let eye = [sim.bond.pos[0], sim.bond.pos[1] + sim.bond.eye - sim.bond.crouch_drop, sim.bond.pos[2]];
        println!("stand pos {:?} eye {eye:?} yaw {} pitch {}", sim.bond.pos, sim.bond.yaw, sim.bond.pitch);
    }
    // Aim at the front pane centre and fire once.
    let at = [107.25008, 35.3124, -121.99846];
    let mut fired_tick = None;
    for t in 0..600u32 {
        let (input, trigger) = {
            let sim = scene.sim.as_ref().unwrap();
            let (bp, dp) = (sim.bond.pos, at);
            let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
            let dx = dp[0] - eye[0]; let dy = dp[1] - eye[1]; let dz = dp[2] - eye[2];
            let dxz = (dx * dx + dz * dz).sqrt().max(0.5);
            let desired = dx.atan2(dz);
            let err = { let mut e = desired - sim.bond.yaw; while e > std::f32::consts::PI { e -= 2.0 * std::f32::consts::PI; } while e < -std::f32::consts::PI { e += 2.0 * std::f32::consts::PI; } e };
            let want_pitch = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            (Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Default::default() },
             err.abs() < 0.01 && perr.abs() < 0.02 && t > 30)
        };
        scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if trigger { fired_tick = Some(t); println!("t{t} TAP gun {} clip {}", sim.gun.weapon, sim.gun.clip); }
        if !sim.drones.player_bullets.is_empty() {
            for b in &sim.drones.player_bullets {
                println!("t{t} bullet w{} pos {:?} dir {:?} dmg {} travelled {}", b.weapon, b.pos, b.dir, b.damage, b.travelled);
            }
        }
        let brk: Vec<usize> = sim.drones.breaks.list.iter().filter(|b| b.dead).map(|b| b.placement).collect();
        if !brk.is_empty() { println!("t{t} BROKEN {brk:?}"); break; }
        if fired_tick.is_some() && t > fired_tick.unwrap() + 80 { println!("t{t} no break within 80 ticks; bullets {:?}", sim.drones.player_bullets.len()); break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    println!("final broken: {:?}", sim.drones.breaks.list.iter().filter(|b| b.dead).map(|b| (b.placement, b.health)).collect::<Vec<_>>());
    println!("impacts: {:?}", sim.drones.player_impacts.len());
    let e = sim.drones.audio.live_ids();
    println!("0x2e live: {}", e.iter().filter(|&&i| i == 0x2e).count());
}
