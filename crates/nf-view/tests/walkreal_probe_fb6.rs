//! fix-final-blockers2 probe 1: cut-scene #289 (0x060006cf) over its whole run (t1580..2260) —
//! per tick: Bond's pose/disabled, the lift SP's live matrix (the platform), and every running
//! 0x060006cf stream binding + its bound object's matrix row3. Plus NF_SCRIPTLOG's event-6/free
//! diagnostics from nis.rs. Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn dump_cutscene289_streams_and_matrices() {
    std::env::set_var("NF_SCRIPTLOG", "1");
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "0700004a.bin", false).unwrap());
    for k in 0..84 {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        let target = if k < 42 { [3.2, 0.0, -2.2] } else { [3.2, 0.0, -3.8] };
        let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        w.tick(nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: 1.0, ..Default::default() }, false);
    }
    w.skip_cutscenes = false;
    for k in 0..2700u32 {
        w.tick(Default::default(), false);
        let t = k + 84;
        if t < 2270 || t > 2700 { if !(2180..=2200).contains(&t) { continue; } }
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        // The lift SP's live matrix (the platform): the SP whose script is 0x06000571.
        let sp = sim.drones.scripts.players.iter().find(|sp| sp.scripts.contains(&0x06000571));
        let sp_y = sp.map(|sp| sp.live_matrix[3][1]).unwrap_or(f32::NAN);
        let sp_xz = sp.map(|sp| (sp.live_matrix[3][0], sp.live_matrix[3][2])).unwrap_or((f32::NAN, f32::NAN));
        // The running cut-scene: find the player whose info slot holds 0x060006cf.
        let mut line = format!("t{t} bond ({:.2},{:.2},{:.2}) disabled {} grounded {}", p[0], p[1], p[2], sim.disabled, sim.bond.grounded);
        if let Some(sp) = sp { line.push_str(&format!(" | platform ({:.2},{:.2},{:.2})", sp_xz.0, sp_y, sp_xz.1)); }
        // fix-final-blockers2: every script player's base + live matrix, and (in the enable window)
        // every running script's stream bindings with the bound objects' matrices.
        if t >= 2180 && t <= 2190 {
            for (pi, pl) in sim.drones.scripts.players.iter().enumerate() {
                if pl.info.iter().all(|x| x.is_none()) { continue; }
                let m = &pl.matrix; let lm = &pl.live_matrix;
                line.push_str(&format!(" || SP pl{pi} placement{} scripts {:08x}/{:08x} base ({:.2},{:.2},{:.2}) live ({:.2},{:.2},{:.2})",
                    pl.placement, pl.scripts[0], pl.scripts[1], m[3][0], m[3][1], m[3][2], lm[3][0], lm[3][1], lm[3][2]));
                for (slot, info) in pl.info.iter().enumerate() {
                    if let Some(s) = info {
                        line.push_str(&format!(" >> slot{slot} {:08x} clock {:.0}", s.nis.hashcode, s.clock));
                        for (si, (kind, obj, fk, lk)) in s.stream_bindings().iter().enumerate() {
                            let om = obj.and_then(|o| sim.drones.scripts.objects.list.get(o)).and_then(|x| x.as_ref()).map(|o| match o {
                                nf_game::nis::Obj::Entity { matrix, hash, .. } => format!("E{:08x}({:.2},{:.2},{:.2})", hash, matrix[3][0], matrix[3][1], matrix[3][2]),
                                nf_game::nis::Obj::Anim { skin, script, frame, .. } => format!("A{:08x}/s{:08x}@f{:.0}", skin, script, frame),
                                nf_game::nis::Obj::Camera { matrix, .. } => format!("C({:.2},{:.2},{:.2})", matrix[3][0], matrix[3][1], matrix[3][2]),
                            });
                            line.push_str(&format!(" | s{si} k{kind} fk{fk} lk{lk} obj {obj:?} {om:?}"));
                        }
                    }
                }
            }
        }
        for (pi, pl) in sim.drones.scripts.players.iter().enumerate() {
            for (slot, info) in pl.info.iter().enumerate() {
                if let Some(s) = info {
                    if s.nis.hashcode == 0x060006cf {
                        line.push_str(&format!(" | CS289 pl{pi} slot{slot} clock {:.0} flags {:x} play {} player_at {:?}", s.clock, s.flags, s.play,
                            s.player_at.map(|m| format!("({:.2},{:.2},{:.2})", m[3][0], m[3][1], m[3][2]))));
                        for (si, (kind, obj, fk, lk)) in s.stream_bindings().iter().enumerate() {
                            let om = obj.and_then(|o| sim.drones.scripts.objects.list.get(o)).and_then(|x| x.as_ref()).map(|o| match o {
                                nf_game::nis::Obj::Entity { matrix, hash, .. } => format!("E{:08x}({:.2},{:.2},{:.2})", hash, matrix[3][0], matrix[3][1], matrix[3][2]),
                                nf_game::nis::Obj::Anim { skin, script, frame, .. } => format!("A{:08x}/s{:08x}@f{:.0}", skin, script, frame),
                                nf_game::nis::Obj::Camera { matrix, .. } => format!("C({:.2},{:.2},{:.2})", matrix[3][0], matrix[3][1], matrix[3][2]),
                            });
                            line.push_str(&format!(" | s{si} k{kind} fk{fk} lk{lk} obj {obj:?} {om:?}", ));
                        }
                    }
                }
            }
        }
        eprintln!("{line}");
        if t == 2190 {
            // The floor grid around the -52 landing: raycast down from 1.2 above each cell.
            for x in [0.0, 1.0, 1.8, 2.2, 2.6, 3.0, 3.4, 3.8, 4.2, 4.6, 5.0, 5.4, 6.0, 7.0] {
                let mut row = String::new();
                for z in [1.0, 0.0, -1.0, -1.5, -2.0, -2.5, -3.0, -3.5, -4.0, -5.0] {
                    let hit = sim.world.segment_hit([x, -51.0, z], [x, -57.0, z]);
                    row.push_str(&format!(" z{z:.1}{}", hit.map(|h| format!("{:.2}", -51.0 + h.0 * -6.0)).unwrap_or("--".into())));
                }
                eprintln!("floor x{x:.1}:{row}");
            }
            // The doors near the landing: their positions and states.
            for (i, d) in sim.drones.doors.list.iter().enumerate().skip(45).take(12) {
                eprintln!("door {i}: pos ({:.2},{:.2},{:.2}) state {:#x}", d.pos[0], d.pos[1], d.pos[2], d.state);
            }
            // Floor probes down the shaft at the marker's x/z, from -52 to -160.
            for (x, z) in [(1.8, -3.45f32), (1.8, -6.83f32), (4.37, -3.78f32)] {
                let mut row = String::new();
                let mut y = -52.0f32;
                while y > -160.0 {
                    if let Some(h) = sim.world.segment_hit([x, y, z], [x, y - 1.0, z]) {
                        let fy = y - h.0;
                        row.push_str(&format!(" {:.2}", fy));
                        y = fy - 0.5;
                    } else { y -= 1.0; }
                }
                eprintln!("shafts x{x:.2} z{z:.2} floors:{row}");
            }
        }
        if t >= 2700 { break; }
    }
}
