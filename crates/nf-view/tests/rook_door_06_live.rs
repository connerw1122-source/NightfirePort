//! rook-door-06: Rook (the ally who leads Bond) stalls in a doorway on 07000006 — playtests
//! 2026-10-15 ("rook gets stuck on a door") and 2026-10-16 ("rook gets stuck in the doorway
//! on 06. again"); the 663 npc-doors merge did not fix it.
//!
//! This probe drives the level's REAL path — the truck-ride carry (start #496), the arrival
//! plates (ch 185 = 0xB9, the driver greeting + the escort's enable channel), Bond following
//! the lead through real inputs — and logs the lead's state/position/route/door per tick so
//! the doorway stall shows itself. Opt-in via NF_ISO.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};

use walker::Walker;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

fn dump_census(sim: &nf_view::scene::Sim, tag: &str) {
    eprintln!("[{tag}] bond {:?}", sim.bond.pos);
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        eprintln!("[{tag}] door #{i} ({:.2},{:.2},{:.2}) kind {} group {} approach {} locked {} ch_open {} ch_lock {} flags {:#x} state {:#x}",
            d.pos[0], d.pos[1], d.pos[2], d.kind, d.group, d.approach, d.locked_flag, d.ch_open, d.ch_lock, d.flags, d.state);
    }
    for (pi, p) in sim.nav.paths.iter().enumerate() {
        let dn: Vec<String> = p.nodes.iter().enumerate().filter(|(_, n)| n.flags & 2 != 0)
            .map(|(i, n)| format!("n{}({:.1},{:.1},{:.1})", i, n.pos[0], n.pos[1], n.pos[2])).collect();
        let all: Vec<String> = p.nodes.iter().enumerate()
            .map(|(i, n)| format!("n{}({:.1},{:.1},{:.1})f{:#x}", i, n.pos[0], n.pos[1], n.pos[2], n.flags)).collect();
        eprintln!("[{tag}] nav path {pi} kind {} {} nodes: {}", p.kind, p.nodes.len(),
            if p.kind & 0xD != 0 { all.join(" ") } else { dn.join(" ") });
    }
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || dr.corpse { continue; }
        let route = dr.route.as_ref().map(|r| format!("path {} idx {} {} nodes", r.path, r.index, r.nodes.len())).unwrap_or("-".into());
        eprintln!("[{tag}] drone #{i} skin {:#x} kind {} state {:#x} enabled {} pos ({:.1},{:.1},{:.1}) route {} switch_ch {}",
            dr.skin, dr.kind, dr.state, dr.enabled, dr.pos[0], dr.pos[1], dr.pos[2], route, dr.switch_channel);
    }
}

/// The full real path: the truck-ride carry → the arrival plates (ch 185) → the escort (Rook)
/// wakes → Bond follows him on real inputs until he stops walking. Per-tick log of the lead.
#[test]
fn probe_rook_leads_and_stalls() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006"));
    w.wait_enabled(4000, "the intro NIS");
    w.wait(300);
    dump_census(w.scene.sim.as_ref().unwrap(), "start");

    // The arrival plates: walk forward from the truck spawn until ch 185 (0xB9) rises.
    let mut input = Input::default();
    input.forward = 1.0;
    let mut ch185_at = None;
    for t in 0..1200u32 {
        w.tick(input, false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xB9) { ch185_at = Some(t); break; }
    }
    eprintln!("[plates] ch 0xB9 at {ch185_at:?} bond {:?}", w.scene.sim.as_ref().unwrap().bond.pos);
    w.wait(60);

    // The lead: the escort that wakes on the plates — snapshot who turned enabled / state-changed.
    let before: Vec<(bool, u8)> = w.scene.sim.as_ref().unwrap().drones.list.iter().map(|d| (d.enabled, d.state)).collect();
    let mut lastpos: Vec<[f32; 3]> = w.scene.sim.as_ref().unwrap().drones.list.iter().map(|d| d.pos).collect();
    for t in 0..70_000u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        // Who woke, ever.
        for (i, dr) in sim.drones.list.iter().enumerate() {
            if !dr.present { continue; }
            if dr.enabled != before[i].0 {
                eprintln!("[wake +{}] drone #{} skin {:#x} kind {} enabled {} state {:#x} pos ({:.1},{:.1},{:.1}) switch_ch {}",
                    t, i, dr.skin, dr.kind, dr.enabled, dr.state, dr.pos[0], dr.pos[1], dr.pos[2], dr.switch_channel);
            }
        }
        // Movers + door-workers: any enabled drone that moved or stands near a door.
        if t % 30 == 0 {
            for (i, dr) in sim.drones.list.iter().enumerate() {
                if !dr.present || dr.corpse || !dr.enabled { continue; }
                let moved = walker::dxz(dr.pos, lastpos[i]);
                let near_door = dr.hit_door.map(|d| format!("door#{} {:#x}", d, sim.drones.doors.list[d].state)).unwrap_or_default();
                if moved > 0.15 || !near_door.is_empty() || matches!(dr.state, 0x1c | 0x1d | 0x20 | 0x21 | 0x25 | 0x26 | 0x81) {
                    eprintln!("[w +{}] drone #{} skin {:#x} state {:#x} pos ({:.2},{:.2},{:.2}) mv {:.2} route {} {}",
                        t, i, dr.skin, dr.state, dr.pos[0], dr.pos[1], dr.pos[2], moved,
                        dr.route.as_ref().map(|r| format!("p{} i{}/{}", r.path, r.index, r.nodes.len())).unwrap_or("-".into()), near_door);
                }
            }
        }
        for (i, dr) in sim.drones.list.iter().enumerate() { lastpos[i] = dr.pos; }
        if t % 3000 == 0 {
            eprintln!("[t +{}] bond ({:.1},{:.1},{:.1}) ch185 {}", t, sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2], sim.drones.channels.on(0xB9) as u8);
        }
        if sim.drones.level_exit.is_some() { eprintln!("[watch] level exit at +{t}"); break; }
        // Bond trails the nearest enabled non-corpse NPC at the 3..8 band (real inputs).
        let mut nearest: Option<(usize, f32)> = None;
        for (i, dr) in sim.drones.list.iter().enumerate() {
            if !dr.present || dr.corpse || !dr.enabled { continue; }
            let d = walker::dxz(sim.bond.pos, dr.pos);
            if nearest.map_or(true, |(_, bd)| d < bd) { nearest = Some((i, d)); }
        }
        if let Some((_, d)) = nearest {
            if d < 2.8 { w.tick(Input::default(), false); } // stand (too close — back out of the PlayerInWay band next ticks)
            else {
                let (target, dd) = { let s = w.scene.sim.as_ref().unwrap(); (s.drones.list[nearest.unwrap().0].pos, d) };
                let desired = (target[0] - sim.bond.pos[0]).atan2(target[2] - sim.bond.pos[2]);
                let err = walker::wrap_pi(desired - sim.bond.yaw);
                let input = Input { turn: err.clamp(-0.07, 0.07), forward: if dd > 8.0 { 1.0 } else if dd > 6.0 { 0.5 } else { 0.0 }, ..Default::default() };
                w.tick(input, false);
            }
        }
    }
    dump_census(w.scene.sim.as_ref().unwrap(), "end");
}
