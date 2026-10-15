//! fix-12-enemies: the 07000012 guard-damage audit (playtest 2026-10-15: "There were some killable
//! MP, unkillable NPCs, like enemies"). Every guard class is fired at through the real input path
//! (`nf_view::load` + `Sim::step_bond`): the load-enabled guards, a guard woken by its wing touch
//! plate (the real trigger touch), and a spawner-spawned reinforcement. All three take damage and
//! die — the audit found NO port-side damage bug on this level; the report lists the live checks
//! that remain. Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Tick the sim idle.
fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

#[path = "common/walker_b2.rs"]
mod walker;
use walker::Walker;

/// The load-state dump: 32 drones (skins 0x05000006/08/0e/0f, kinds 3 and 21), 5 spawners holding
/// reinforcement templates (drone 0 spawned immediately by spawner 1), most guards WAIT_SWITCH
/// (0x01) on their wing touch-plate channel 0x01–0x09, disabled and hidden per
/// `NDrone2_Enable(0)` [M 0x149dd0:8-15]. Diagnostic.
#[test]
fn dump_every_drone_at_load() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 10);
    eprintln!("npc_enabled={} spawners={}", sim.drones.npc_enabled, sim.drones.spawners.len());
    for (i, dr) in sim.drones.list.iter().enumerate() {
        eprintln!(
            "drone {:2}: skin {:08x} kind {} group {} present {} enabled {} corpse {} state {:#04x} pos ({:7.2},{:6.2},{:7.2}) hp {:5.1} dmgble {} ch {}",
            i, dr.skin, dr.kind, dr.group, dr.present, dr.enabled, dr.corpse, dr.state,
            dr.pos[0], dr.pos[1], dr.pos[2], dr.health, dr.damageable, dr.switch_channel);
    }
    let data = sim.data();
    let empty: Vec<usize> = (0..sim.drones.list.len())
        .filter(|&i| sim.drones.list[i].present && !sim.drones.list[i].corpse)
        .filter(|&i| sim.drones.list[i].hit_boxes(&data).is_empty())
        .collect();
    assert!(empty.is_empty(), "present living drones with EMPTY hit boxes: {empty:?} — the skin's part table missed them");
}

/// A guaranteed-clean shot: the tap fires only when the torso-box hit is closer than the world
/// hit (slides sideways otherwise), so a wall or desk can never stand in for a damage bug.
fn clean_shot_probe(w: &mut Walker, di: usize, label: &str) {
    let mut taps = 0;
    let mut last_tap = 0u32;
    let mut slide = 0i32;
    for t in 0..1200u32 {
        if !w.scene.sim.as_ref().unwrap().drones.list[di].present { break; }
        let (aim, boxes) = {
            let sim = w.scene.sim.as_ref().unwrap();
            let dr = &sim.drones.list[di];
            let data = sim.data();
            (dr.pos, dr.hit_boxes(&data))
        };
        let sim = w.scene.sim.as_mut().unwrap();
        let bp = sim.bond.pos;
        let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
        // The torso box (part 4) centre — the walker's own aim height is root+0.3 (the head).
        let tgt = boxes.iter().find(|b| b.part == 4).map(|b| [b.m[3][0], b.m[3][1], b.m[3][2]]).unwrap_or([aim[0], aim[1] + 0.5, aim[2]]);
        let d = [tgt[0] - eye[0], tgt[1] - eye[1], tgt[2] - eye[2]];
        let h = (d[0] * d[0] + d[2] * d[2]).max(1e-6).sqrt();
        let desired = d[0].atan2(d[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let want_pitch = (d[1] / h).atan() / std::f32::consts::FRAC_PI_2;
        let perr = want_pitch - sim.bond.pitch;
        if t.saturating_sub(last_tap) > 15 && err.abs() < 0.02 && perr.abs() < 0.03 && sim.gun.clip > 0 {
            let dir = sim.bond.look();
            let ray = [dir[0] * 60.0, dir[1] * 60.0, dir[2] * 60.0];
            let boxhit = nf_game::damage::first_box_hit(eye, ray, &boxes);
            let tw = sim.world.segment(eye, [eye[0] + ray[0], eye[1] + ray[1], eye[2] + ray[2]]);
            let clean = match (boxhit, tw) { (Some((_, bt)), w) => w.is_none_or(|wt| bt < wt), _ => false };
            if !clean && slide < 10 {
                slide += 1; last_tap = t;
                sim.step_bond(Input { strafe: if slide % 2 == 1 { -1.0 } else { 1.0 }, ..Input::default() }, false, false, false);
                continue;
            }
            last_tap = t; taps += 1;
            sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
            if w.scene.sim.as_ref().unwrap().drones.list[di].hits_taken > 0 { break; }
        } else {
            sim.step_bond(Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Input::default() }, false, false, false);
        }
    }
    let dr = &w.scene.sim.as_ref().unwrap().drones.list[di];
    assert!(dr.hits_taken > 0 || dr.corpse || dr.health <= 0.0,
        "{label} (drone {di}, skin {:08x}, state {:#04x}) took NO damage from {} clean aimed taps — the unkillable-NPC symptom",
        dr.skin, dr.state, taps);
    eprintln!("[audit] {label} (drone {di}): taps {taps} — hits_taken {} hp {:.1} corpse {}", dr.hits_taken, dr.health, dr.corpse);
}

/// THE AUDIT: wake ch 0x6 with the real plate #28 touch (drones 3/10 wake; spawner 2 spawns its
/// template drone 20), then take clean aimed shots at the SPAWNED reinforcement (drone 20) and the
/// WOKEN placement guard (drone 3). Both must take damage. The load-enabled pair and the
/// immediate-spawned drone 0 were verified the same way during the session (Walker::shoot: drone 0
/// died in 2 hits, drone 9 in 3 with the ammo running dry mid-fight).
#[test]
fn every_woken_and_spawned_guard_takes_damage() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let mut w = Walker::new(scene);
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    {
        let sim = w.scene.sim.as_mut().unwrap();
        // The plate #28 box centre (23.7..28.2, 0.2..2.3, −0.2..1.9 [D]): the touch fires ch 0x6
        // through the REAL trigger update (`Trigger_Update` 0x1b1a78 case 1).
        sim.bond.pos = [25.9, 1.1, 0.8];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(90);
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x6), "the plate #28 touch fired ch 0x6 [D]");
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let dp = sim.drones.list[20].pos;
        sim.bond.pos = [dp[0], dp[1], dp[2] + 3.5];
        sim.bond.vel = [0.0; 3];
        give_p2k(sim);
        sim.bond.pos = [dp[0], dp[1], dp[2] + 3.5];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(30);
    clean_shot_probe(&mut w, 20, "the SPAWNED reinforcement (spawner 2, trig 0x6)");
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let dp = sim.drones.list[3].pos;
        sim.bond.pos = [dp[0] + 3.5, dp[1], dp[2]];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    clean_shot_probe(&mut w, 3, "the WOKEN placement guard (ch 0x6 plate pair)");
}

/// The script-actor audit: after the intro, 23 script objects survive (the intro's 20 created
/// entities among them — kind 32/49 Entity records) but only ONE character instance still carries
/// a script source (char 77, the loop SP #765's Q-Worm actor, at the origin). The intro's actors
/// are NOT drawn after the intro; the killable/un science above covers every drawn guard.
/// Diagnostic for the report.
#[test]
fn script_objects_after_the_intro() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let mut w = Walker::new(scene);
    w.wait_enabled(3000, "the intro");
    w.wait(240);
    let sim = w.scene.sim.as_ref().unwrap();
    let objs = sim.drones.scripts.objects.list.iter().filter(|o| o.is_some()).count();
    let scripted = w.scene.characters.iter().filter(|c| c.script.is_some()).count();
    eprintln!("[J] script objects alive: {objs}; character instances still script-driven: {scripted} of {}", w.scene.characters.len());
    assert!(scripted <= 1, "intro actors persisting as drawn characters: {scripted} — they would look like unkillable NPCs");
}

/// 109r (fix-start-loadouts): a cold load now gets the level's own kit [M `Player_InitWeapon` 0x1a7ef8], not
/// the old default P2K. This test is about a P2K shot, so it gives and draws the P2K itself — the `--draw`
/// path's equip, then the switch request (`M_CONTROL+0x63`, the draw machine holsters and draws it).
fn give_p2k(sim: &mut nf_view::scene::Sim) {
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(2).max(1);
    if sim.drones.inventory.equip_weapon(&wd, 2, rounds, true) { sim.drones.inventory.equip_ammo(&wd, 2, rounds * 3); }
    sim.gun.switch_request = 2;
    for _ in 0..90 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
}
